use crate::db::{DbConnection, RepositoryError};
use crate::models::admin::{ActiveItemCount, MenuItemCheck};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::PgConnection;
use std::collections::HashMap;

#[derive(Clone)]
pub struct OrderOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl OrderOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
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
                if !item.is_available || !item.list {
                    return Err(RepositoryError::NotAvailable(
                        item.item_id,
                        item.name,
                        "Not available".to_string(),
                    ));
                } else if item.stock != -1 && item.stock < 1 {
                    return Err(RepositoryError::NotAvailable(
                        item.item_id,
                        item.name,
                        "Out of stock".to_string(),
                    ));
                }
            }
        }

        conn.connection().transaction(|connection| {
            // Add to active common
            {
                use crate::db::schema::active_orders::dsl::*;

                diesel::insert_into(active_orders)
                    .values((user_id.eq(&userid), items.eq(&itemids)))
                    .execute(connection)
                    .map_err(RepositoryError::DatabaseError)?;
            }
            // Add to item counts
            {
                use crate::db::schema::active_item_count::dsl::*;

                for (food, qty) in ordered_qty.iter() {
                    diesel::update(active_item_count)
                        .filter(item_id.eq(food))
                        .set(num_ordered.eq(num_ordered + qty))
                        .execute(connection)
                        .map_err(|e| match e {
                            Error::NotFound => RepositoryError::NotFound(format!(
                                "active_item_count: Can't find item id to update: {}",
                                food
                            )),
                            other => RepositoryError::DatabaseError(other),
                        })?;
                }

                Ok(())
            }
        })
    }

    pub fn get_all_orders_by_count(&self) -> Result<Vec<ActiveItemCount>, RepositoryError> {
        use crate::db::schema::active_item_count::dsl::*;

        let mut conn = DbConnection::new(&self.pool)?;
        active_item_count
            .load::<ActiveItemCount>(conn.connection())
            .map_err(RepositoryError::DatabaseError)
    }
}
