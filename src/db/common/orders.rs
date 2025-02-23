use crate::db::{DbConnection, RepositoryError};
use crate::enums::common::ActiveItemCount;
use crate::models::admin::MenuItemCheck;
use dashmap::DashMap;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::PgConnection;
use std::collections::HashMap;

#[derive(Clone)]
pub struct OrderOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    active_item_counts: DashMap<i32, i32>,
}

impl OrderOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        let active_item_counts = DashMap::<i32, i32>::new();
        {
            use crate::db::schema::active_orders::dsl::*;
            let mut conn = DbConnection::new(&pool).unwrap();

            debug!("Pulling active order item counts from table...");
            let orders = active_orders
                .select(items)
                .load::<Vec<Option<i32>>>(conn.connection())
                .unwrap();

            for order in orders {
                for item in order {
                    if let Some(mut val) = active_item_counts.get_mut(&item.unwrap_or(-1)) {
                        *val += 1;
                    } else {
                        active_item_counts.insert(item.unwrap_or(-1), 1);
                    }
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
        let mut ordered_qty: HashMap<i32, i32> = HashMap::new();
        for &item in &itemids {
            let qty = ordered_qty.entry(item).or_insert(0);
            *qty += 1;
        }

        // Check item availability
        {
            use crate::db::schema::menu_items::dsl::*;
            let items_in_order = menu_items
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
            for item in items_in_order {
                if !item.is_available {
                    return Err(RepositoryError::NotAvailable(
                        item.item_id,
                        item.name,
                        "Not available".to_string(),
                    ));
                } else if item.stock != -1 && item.stock < *ordered_qty.get(&item.item_id).unwrap_or(&1) { // -1 -> unlimited stock
                    return Err(RepositoryError::NotAvailable(
                        item.item_id,
                        item.name,
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

        // Add to active common
        {
            use crate::db::schema::active_orders::dsl::*;

            diesel::insert_into(active_orders)
                .values((user_id.eq(&userid), items.eq(&itemids)))
                .execute(conn.connection())
                .map_err(RepositoryError::DatabaseError)?;
        }
        Ok(())
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
}
