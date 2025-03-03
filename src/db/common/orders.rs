use crate::db::{DbConnection, RepositoryError};
use crate::enums::common::ActiveItemCount;
use crate::models::admin::MenuItemCheck;
use crate::models::common::OrderItems;
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

#[derive(Clone)]
pub struct OrderOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    active_item_counts: DashMap<i32, i64>,
}

impl OrderOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        let active_item_counts = DashMap::<i32, i64>::new();
        {
            use crate::db::schema::active_order_items::dsl::*;
            let mut conn = DbConnection::new(&pool).unwrap();

            debug!("Pulling active order item counts from table...");
            let orders = active_order_items
                .group_by(item_id)
                .select((item_id, sum(quantity)))
                .load::<(i32, Option<i64>)>(conn.connection())
                .unwrap();

            for (item, qty) in orders {
                if let Some(mut val) = active_item_counts.get_mut(&item) {
                    *val += qty.unwrap_or(1);
                } else {
                    active_item_counts.insert(item, qty.unwrap_or(1));
                }
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
                    *val += qty;
                } else {
                    self.active_item_counts.insert(*food, *qty);
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
                num_ordered: *element.value(),
            });
        }
        response
    }

    #[allow(dead_code)]
    pub fn get_orders_by_rfid(&self) -> Vec<OrderItems> {
        todo!();
    }

    #[allow(dead_code)]
    pub fn get_orders_by_userid(&self) -> Vec<OrderItems> {
        todo!();
    }
}
