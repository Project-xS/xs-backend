use crate::db::{DbConnection, RepositoryError};
use crate::enums::common::{ActiveItemCount, OrderItemContainer, ItemContainer};
use crate::models::{common::OrderItems, admin::MenuItemCheck};
use dashmap::DashMap;
use diesel::dsl::sum;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::PgConnection;
use std::collections::HashMap;

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::active_order_items)]
struct OrderItem {
    order_id: i32,
    item_id: i32,
    quantity: i16,
}

#[derive(Clone, Debug)]
struct ItemNameQty {
    item_name: String,
    quantity: i64,
}

#[derive(Clone)]
pub struct OrderOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    active_item_counts: DashMap<i32, ItemNameQty>,
}

impl OrderOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        let active_item_counts = DashMap::<i32, ItemNameQty>::new();
        {
            use crate::db::schema::*;
            let mut conn = DbConnection::new(&pool).unwrap();

            debug!("Pulling active order item counts from table...");
            let orders = active_order_items::table
                .inner_join(menu_items::table.on(active_order_items::item_id.eq(menu_items::item_id)))
                .group_by((active_order_items::item_id, menu_items::name))
                .select((
                    active_order_items::item_id,
                    menu_items::name,
                    sum(active_order_items::quantity),
                ))
                .load::<(i32, String, Option<i64>)>(conn.connection())
                .unwrap();

            for (item_id, name, qty_opt) in orders {
                let qty = qty_opt.unwrap_or(1);
                active_item_counts.insert(item_id, ItemNameQty { item_name: name, quantity: qty });
            }
        }
        debug!("Updated active item counts: {:?}", &active_item_counts);

        Self {
            pool,
            active_item_counts,
        }
    }

    pub fn create_order(&self, userid: i32, itemids: Vec<i32>) -> Result<(), RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;
        let mut ordered_qty: HashMap<i32, i64> = HashMap::new();
        let items_in_order: Vec<MenuItemCheck>;
        for &item in &itemids {
            let qty = ordered_qty.entry(item).or_insert(0);
            *qty += 1;
        }

        // Check item availability
        {
            use crate::db::schema::menu_items::dsl::*;
            items_in_order = menu_items
                .filter(item_id.eq_any(itemids.clone()))
                .select(MenuItemCheck::as_select())
                .load::<MenuItemCheck>(conn.connection())
                .map_err(|e| match e {
                    Error::NotFound => RepositoryError::NotFound(format!(
                        "menu_items: No menu item matched for {:?}",
                        &itemids
                    )),
                    other => RepositoryError::DatabaseError(other),
                })?;

            if ordered_qty.len() != items_in_order.len() {
                return Err(RepositoryError::NotFound(format!(
                    "menu_items: Contains missing menu items: {:?}",
                    &itemids
                )));
            }
            for item in &items_in_order {
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
        // Add to item counts
        {
            for (food, qty) in ordered_qty.iter() {
                if let Some(mut val) = self.active_item_counts.get_mut(food) {
                    val.value_mut().quantity += qty;
                } else {
                    self.active_item_counts.insert(*food, ItemNameQty {
                        item_name: "".to_string(),
                        quantity: *qty
                    });
                }
            }
        }

        conn.connection().transaction(|conn| {
            // Add to active orders
            {
                let new_order_id: i32;
                {
                    use crate::db::schema::active_orders::dsl::*;
                    new_order_id = diesel::insert_into(active_orders)
                        .values(user_id.eq(&userid))
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
                        .map_err(|e| match e {
                            Error::NotFound => RepositoryError::NotFound(format!(
                                "menu_items: Can't find item id to update stock: {}",
                                item
                            )),
                            other => RepositoryError::DatabaseError(other),
                        })?;
                }
            }
            Ok(())
        })
    }

    pub fn get_all_orders_by_count(&self) -> Vec<ActiveItemCount> {
        let mut response: Vec<ActiveItemCount> = Vec::with_capacity(self.active_item_counts.len());
        debug!(
            "Fetched item counts from map: {:?}",
            &self.active_item_counts
        );
        for element in self.active_item_counts.iter() {
            response.push(ActiveItemCount {
                item_id: *element.key(),
                item_name: (*element.value().item_name).parse().unwrap(),
                num_ordered: element.value().quantity,
            });
        }
        response
    }

    fn group_order_items(items: Vec<OrderItems>) -> Vec<OrderItemContainer> {
        let mut grouped = HashMap::new();

        for item in items {
            grouped.entry(item.order_id)
                .or_insert_with(Vec::new)
                .push(ItemContainer {
                    canteen_name: item.canteen_name,
                    name: item.name,
                    quantity: item.quantity,
                    is_veg: item.is_veg,
                    pic_link: item.pic_link,
                    description: item.description
                });
        }

        grouped.into_iter()
            .map(|(order_id, items)| OrderItemContainer { order_id, items })
            .collect()
    }

    pub fn get_orders_by_rfid(&self, search_rfid: &str) -> Result<Vec<OrderItemContainer>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;
        use crate::db::schema::*;
        let order_items = users::table
            .inner_join(
                active_orders::table.on(
                    users::user_id.eq(active_orders::user_id)
                )
            )
            .inner_join(
                active_order_items::table.on(
                    active_orders::order_id.eq(active_order_items::order_id)
                )
            )
            // Then menu items
            .inner_join(
                menu_items::table.on(
                    active_order_items::item_id.eq(menu_items::item_id)
                )
            )
            // Finally canteen info
            .inner_join(
                canteens::table.on(
                    menu_items::canteen_id.eq(canteens::canteen_id)
                )
            )
            .filter(users::rfid.eq(&search_rfid))
            .select((
                active_orders::order_id,
                canteens::canteen_name,
                menu_items::name,
                active_order_items::quantity,
                menu_items::is_veg,
                menu_items::pic_link,
                menu_items::description
            ))
            .order_by(active_orders::ordered_at.desc())
            .load::<OrderItems>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(format!("get_user_by_rfid: {}", search_rfid)),
                other => RepositoryError::DatabaseError(other),
            })?;

        Ok(Self::group_order_items(order_items))
    }

    pub fn get_orders_by_userid(&self, search_user_id: &i32) -> Result<Vec<OrderItemContainer>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;
        use crate::db::schema::*;
        let order_items = active_orders::table
            .inner_join(
                active_order_items::table.on(
                    active_orders::order_id.eq(active_order_items::order_id)
                )
            )
            // Then menu items
            .inner_join(
                menu_items::table.on(
                    active_order_items::item_id.eq(menu_items::item_id)
                )
            )
            // Finally canteen info
            .inner_join(
                canteens::table.on(
                    menu_items::canteen_id.eq(canteens::canteen_id)
                )
            )
            .filter(active_orders::user_id.eq(search_user_id))
            .select((
                active_orders::order_id,
                canteens::canteen_name,
                menu_items::name,
                active_order_items::quantity,
                menu_items::is_veg,
                menu_items::pic_link,
                menu_items::description
            ))
            .order_by(active_orders::ordered_at.desc())
            .load::<OrderItems>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(format!("get_user_by_userid: {}", search_user_id)),
                other => RepositoryError::DatabaseError(other),
            })?;

        Ok(Self::group_order_items(order_items))
    }
    pub fn get_orders_by_orderid(&self, search_order_id: &i32) -> Result<OrderItemContainer, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;
        use crate::db::schema::*;
        let order_items = active_order_items::table
            .inner_join(
                menu_items::table.on(
                    active_order_items::item_id.eq(menu_items::item_id)
                )
            )
            // Finally canteen info
            .inner_join(
                canteens::table.on(
                    menu_items::canteen_id.eq(canteens::canteen_id)
                )
            )
            .filter(active_order_items::order_id.eq(search_order_id))
            .select((
                active_order_items::order_id,
                canteens::canteen_name,
                menu_items::name,
                active_order_items::quantity,
                menu_items::is_veg,
                menu_items::pic_link,
                menu_items::description
            ))
            .load::<OrderItems>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(format!("get_user_by_orderid: {}", search_order_id)),
                other => RepositoryError::DatabaseError(other),
            })?;

        let resp = Self::group_order_items(order_items);
        Ok(resp.into_iter().next().unwrap_or(OrderItemContainer {
            order_id: *search_order_id,
            items: Vec::new()
        }))
    }
}
