use crate::db::{DbConnection, RepositoryError};
use crate::enums::common::{
    ActiveItemCount, ItemContainer, OrderItemContainer, TimedActiveItemCount,
};
use crate::models::common::TimeBandEnum;
use crate::models::{admin::MenuItemCheck, common::OrderItems, user::NewPastOrder};
use chrono::{DateTime, Utc};
use diesel::dsl::sum;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::PgConnection;
use log::{debug, error};
use std::collections::HashMap;

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::active_order_items)]
struct OrderItem {
    order_id: i32,
    item_id: i32,
    quantity: i16,
}

#[derive(Queryable, Debug)]
struct ItemNameQtyTime {
    item_id: i32,
    item_name: String,
    total_quantity: Option<i64>,
    deliver_at: Option<TimeBandEnum>,
}

#[derive(Queryable, Clone, Debug)]
struct OrderDeliverItems {
    user_id: i32,
    item_id: i32,
    price: i32,
    quantity: i16,
    ordered_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct OrderOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl OrderOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub fn create_order(
        &self,
        userid: i32,
        itemids: Vec<i32>,
        order_deliver_at: Option<String>,
    ) -> Result<(), RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("create_order: failed to acquire DB connection: {}", e);
            e
        })?;

        if itemids.is_empty() {
            return Err(RepositoryError::ValidationError(format!(
                "Order is empty for user: {:?}",
                &userid
            )));
        }

        let mut ordered_qty: HashMap<i32, i64> = HashMap::new();
        let items_in_order: Vec<MenuItemCheck>;
        let canteen_id_in_order: i32;

        for &item in &itemids {
            let qty = ordered_qty.entry(item).or_insert(0);
            *qty += 1;
        }

        // Check item availability
        {
            use crate::db::schema::*;
            items_in_order = menu_items::table
                .filter(menu_items::item_id.eq_any(itemids.clone()))
                .select(MenuItemCheck::as_select())
                .load::<MenuItemCheck>(conn.connection())
                .map_err(|e| {
                    error!(
                        "create_order: error loading menu items for item_ids {:?}: {}",
                        itemids, e
                    );
                    match e {
                        Error::NotFound => RepositoryError::NotFound(format!(
                            "menu_items: No menu item matched for {:?}",
                            &itemids
                        )),
                        other => RepositoryError::DatabaseError(other),
                    }
                })?;

            if ordered_qty.len() != items_in_order.len() {
                return Err(RepositoryError::ValidationError(format!(
                    "Order contains missing menu items: {:?}",
                    &itemids
                )));
            }

            canteen_id_in_order = items_in_order.first().unwrap().canteen_id;

            for item in &items_in_order {
                if canteen_id_in_order != item.canteen_id {
                    return Err(RepositoryError::ValidationError(format!(
                        "Order contains items from multiple canteens: {:?} for user: {}",
                        &itemids, userid
                    )));
                }
                if !item.is_available {
                    return Err(RepositoryError::NotAvailable(
                        item.item_id,
                        item.name.clone(),
                        "Not available".to_string(),
                    ));
                } else if item.stock != -1
                    && (item.stock as i64) < *ordered_qty.get(&item.item_id).unwrap_or(&1)
                {
                    // -1 -> unlimited stock
                    return Err(RepositoryError::NotAvailable(
                        item.item_id,
                        item.name.clone(),
                        "Out of stock".to_string(),
                    ));
                }
            }
        }

        conn.connection().transaction(|conn| {
            let order_total_price = items_in_order
                .iter()
                .map(|e| e.price * *ordered_qty.get(&e.item_id).unwrap_or(&1) as i32)
                .sum::<i32>();

            // Add to active orders
            {
                let order_deliver_time_enum: Option<TimeBandEnum> =
                    TimeBandEnum::get_enum_from_str(order_deliver_at.as_deref());
                let new_order_id: i32;
                {
                    use crate::db::schema::active_orders::dsl::*;
                    new_order_id = diesel::insert_into(active_orders)
                        .values((
                            user_id.eq(&userid),
                            canteen_id.eq(&canteen_id_in_order),
                            total_price.eq(&order_total_price),
                            deliver_at.eq(&order_deliver_time_enum),
                        ))
                        .returning(order_id)
                        .get_result::<i32>(conn)
                        .map_err(RepositoryError::DatabaseError)?;
                }

                let mut new_order_items: Vec<OrderItem> = Vec::new();
                for (item, qty) in ordered_qty.iter() {
                    new_order_items.push(OrderItem {
                        order_id: new_order_id,
                        item_id: *item,
                        quantity: *qty as i16,
                    })
                }

                {
                    use crate::db::schema::active_order_items::dsl::*;
                    diesel::insert_into(active_order_items)
                        .values(&new_order_items)
                        .execute(conn)
                        .map_err(RepositoryError::DatabaseError)?;
                }
            }

            // Decrement stock and is_available
            {
                let mut updated_stock: HashMap<i32, i64> = HashMap::new();
                for item in items_in_order {
                    updated_stock.insert(
                        item.item_id,
                        (item.stock as i64) - *ordered_qty.get(&item.item_id).unwrap_or(&1),
                    );
                }

                use crate::db::schema::menu_items::dsl::*;

                for (item, new_stock) in updated_stock {
                    diesel::update(menu_items.filter(item_id.eq(item)))
                        .set((stock.eq(new_stock as i32), is_available.eq(new_stock > 0)))
                        .execute(conn)
                        .map_err(|e| {
                            error!(
                                "create_order: error updating stock for item id {}: {}",
                                item, e
                            );
                            match e {
                                Error::NotFound => RepositoryError::NotFound(format!(
                                    "menu_items: Can't find item id {} to update stock",
                                    item
                                )),
                                other => RepositoryError::DatabaseError(other),
                            }
                        })?;
                }
            }
            Ok(())
        })
    }

    pub fn get_all_orders_by_count(
        &self,
        search_canteen_id: i32,
    ) -> Result<TimedActiveItemCount, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_all_orders_by_count: failed to acquire DB connection.: {}",
                e
            );
            e
        })?;

        use crate::db::schema::*;

        let db_resp = active_order_items::table
            .inner_join(
                active_orders::table.on(active_order_items::order_id.eq(active_orders::order_id)),
            )
            .inner_join(menu_items::table.on(active_order_items::item_id.eq(menu_items::item_id)))
            .group_by((
                active_order_items::item_id,
                active_orders::deliver_at,
                menu_items::name,
            ))
            .select((
                active_order_items::item_id,
                menu_items::name,
                sum(active_order_items::quantity),
                active_orders::deliver_at,
            ))
            .filter(active_orders::canteen_id.eq(search_canteen_id))
            .load::<ItemNameQtyTime>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_all_orders_by_count: error querying order items count: {}",
                    e
                );
                match e {
                    Error::NotFound => {
                        RepositoryError::NotFound(format!("get_all_orders_by_count: {}", e))
                    }
                    other => RepositoryError::DatabaseError(other),
                }
            })?;

        let mut resp = TimedActiveItemCount::new();
        for item in db_resp {
            let deliver_time_string: String = if item.deliver_at.is_some() {
                item.deliver_at.unwrap().human_readable().to_string()
            } else {
                "Instant".to_string()
            };
            if let Some(val) = resp.get_mut(&deliver_time_string) {
                (*val).push(ActiveItemCount {
                    item_id: item.item_id,
                    item_name: item.item_name,
                    num_ordered: item.total_quantity.unwrap_or(1),
                });
            } else {
                resp.insert(
                    deliver_time_string,
                    vec![ActiveItemCount {
                        item_id: item.item_id,
                        item_name: item.item_name,
                        num_ordered: item.total_quantity.unwrap_or(1),
                    }],
                );
            }
        }
        Ok(resp)
    }

    fn group_order_items(items: Vec<OrderItems>) -> Vec<OrderItemContainer> {
        debug!("Ungrouped order items: {:?}", &items);
        let mut grouped: HashMap<i32, (i32, Option<TimeBandEnum>, Vec<ItemContainer>)> =
            HashMap::new();

        for item in items {
            let (_, _, new_item) = grouped
                .entry(item.order_id)
                .or_insert_with(|| (item.total_price, item.deliver_at, Vec::new()));

            new_item.push(ItemContainer {
                canteen_name: item.canteen_name,
                name: item.name,
                quantity: item.quantity,
                is_veg: item.is_veg,
                pic_link: item.pic_link,
                description: item.description,
            });
        }
        debug!("Grouped order items: {:?}", &grouped);

        grouped
            .into_iter()
            .map(|(order_id, (total_price, deliver_at, items))| {
                let order_deliver_time_string = match deliver_at.as_ref() {
                    Some(deliver_at) => deliver_at.human_readable().to_string(),
                    None => "Instant".to_string(),
                };
                OrderItemContainer {
                    order_id,
                    items,
                    total_price,
                    deliver_at: order_deliver_time_string,
                }
            })
            .collect()
    }

    pub fn get_orders_by_rfid(
        &self,
        search_rfid: &str,
    ) -> Result<Vec<OrderItemContainer>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_orders_by_rfid: failed to acquire DB connection for rfid '{}': {}",
                search_rfid, e
            );
            e
        })?;
        use crate::db::schema::*;
        let order_items = users::table
            .inner_join(active_orders::table.on(users::user_id.eq(active_orders::user_id)))
            .inner_join(
                active_order_items::table
                    .on(active_orders::order_id.eq(active_order_items::order_id)),
            )
            .inner_join(canteens::table.on(active_orders::canteen_id.eq(canteens::canteen_id)))
            .inner_join(menu_items::table.on(active_order_items::item_id.eq(menu_items::item_id)))
            .filter(users::rfid.eq(&search_rfid))
            .select((
                active_orders::order_id,
                canteens::canteen_name,
                active_orders::total_price,
                active_orders::deliver_at,
                menu_items::name,
                active_order_items::quantity,
                menu_items::is_veg,
                menu_items::pic_link,
                menu_items::description,
            ))
            .order_by(active_orders::ordered_at.desc())
            .load::<OrderItems>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_orders_by_rfid: error querying order items for rfid '{}': {}",
                    search_rfid, e
                );
                match e {
                    Error::NotFound => {
                        RepositoryError::NotFound(format!("get_user_by_rfid: {}", search_rfid))
                    }
                    other => RepositoryError::DatabaseError(other),
                }
            })?;

        Ok(Self::group_order_items(order_items))
    }

    pub fn get_orders_by_userid(
        &self,
        search_user_id: &i32,
    ) -> Result<Vec<OrderItemContainer>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_orders_by_userid: failed to acquire DB connection for user_id {}: {}",
                search_user_id, e
            );
            e
        })?;
        use crate::db::schema::*;
        let order_items = active_orders::table
            .inner_join(
                active_order_items::table
                    .on(active_orders::order_id.eq(active_order_items::order_id)),
            )
            .inner_join(menu_items::table.on(active_order_items::item_id.eq(menu_items::item_id)))
            .inner_join(canteens::table.on(menu_items::canteen_id.eq(canteens::canteen_id)))
            .filter(active_orders::user_id.eq(search_user_id))
            .select((
                active_orders::order_id,
                canteens::canteen_name,
                active_orders::total_price,
                active_orders::deliver_at,
                menu_items::name,
                active_order_items::quantity,
                menu_items::is_veg,
                menu_items::pic_link,
                menu_items::description,
            ))
            .order_by(active_orders::ordered_at.desc())
            .load::<OrderItems>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_orders_by_userid: error loading order items for user_id {}: {}",
                    search_user_id, e
                );
                match e {
                    Error::NotFound => {
                        RepositoryError::NotFound(format!("get_user_by_userid: {}", search_user_id))
                    }
                    other => RepositoryError::DatabaseError(other),
                }
            })?;

        Ok(Self::group_order_items(order_items))
    }
    pub fn get_orders_by_orderid(
        &self,
        search_order_id: &i32,
    ) -> Result<OrderItemContainer, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_orders_by_orderid: failed to acquire DB connection for order_id {}: {}",
                search_order_id, e
            );
            e
        })?;
        use crate::db::schema::*;
        let order_items = active_order_items::table
            .inner_join(menu_items::table.on(active_order_items::item_id.eq(menu_items::item_id)))
            .inner_join(canteens::table.on(menu_items::canteen_id.eq(canteens::canteen_id)))
            .inner_join(
                active_orders::table.on(active_order_items::order_id.eq(active_orders::order_id)),
            )
            .filter(active_order_items::order_id.eq(search_order_id))
            .select((
                active_order_items::order_id,
                canteens::canteen_name,
                active_orders::total_price,
                active_orders::deliver_at,
                menu_items::name,
                active_order_items::quantity,
                menu_items::is_veg,
                menu_items::pic_link,
                menu_items::description,
            ))
            .load::<OrderItems>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_orders_by_orderid: error fetching order items for order_id {}: {}",
                    search_order_id, e
                );
                match e {
                    Error::NotFound => RepositoryError::NotFound(format!(
                        "get_user_by_orderid: {}",
                        search_order_id
                    )),
                    other => RepositoryError::DatabaseError(other),
                }
            })?;

        let resp = Self::group_order_items(order_items);
        Ok(resp.into_iter().next().unwrap_or(OrderItemContainer {
            order_id: *search_order_id,
            total_price: 0,
            deliver_at: String::new(),
            items: Vec::new(),
        }))
    }

    pub fn order_actions(
        &self,
        search_order_id: &i32,
        deliver_status: &str,
    ) -> Result<(), RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("order_actions: get_orders_by_orderid: failed to acquire DB connection for order_id {}: {}", search_order_id, e);
            e
        })?;

        conn.connection().transaction(|conn| {
            let order_items: Vec<OrderDeliverItems>;
            {
                use crate::db::schema::*;
                order_items = active_orders::table
                    .inner_join(active_order_items::table.on(
                        active_orders::order_id.eq(active_order_items::order_id)
                    ))
                    .select((
                        active_orders::user_id,
                        active_order_items::item_id,
                        active_orders::total_price,
                        active_order_items::quantity,
                        active_orders::ordered_at
                    ))
                    .filter(active_orders::order_id.eq(search_order_id))
                    .load::<OrderDeliverItems>(conn)
                    .map_err(|e| {
                        error!("order_actions: error fetching order items for order_id {}: {}", search_order_id, e);
                        match e {
                            Error::NotFound => RepositoryError::NotFound(format!("order_actions: {}", search_order_id)),
                            other => RepositoryError::DatabaseError(other),
                        }
                    })?;
                if order_items.is_empty() {
                    return Err(RepositoryError::NotFound(format!("order_actions: {}", search_order_id)));
                }
            }
            let items_in_order: Vec<i32> = order_items
                .iter().flat_map(|item| {
                    std::iter::repeat(item.item_id).take(item.quantity as usize)
                })
                .collect();
            let first_item = order_items.first().unwrap();

            {
                use crate::db::schema::past_orders::dsl::*;
                diesel::insert_into(past_orders)
                    .values(&NewPastOrder {
                        order_id: search_order_id.to_string(),
                        user_id: first_item.user_id,
                        items: items_in_order,
                        price: first_item.price,
                        order_status: deliver_status == "delivered",
                        ordered_at: first_item.ordered_at
                    })
                    .execute(conn)
                    .map_err(RepositoryError::DatabaseError)?;
            }

            {
                use crate::db::schema::*;
                diesel::delete(active_orders::table
                    .filter(active_orders::order_id.eq(search_order_id)))
                    .execute(conn)
                    .map_err(|e| {
                        error!("order_actions: error fetching order items for order_id during delete: {}: {}", search_order_id, e);
                        match e {
                            Error::NotFound => RepositoryError::NotFound(format!("order_actions: {}", search_order_id)),
                            other => RepositoryError::DatabaseError(other),
                        }
                    })?;
            }
            Ok(())
        })
    }
}
