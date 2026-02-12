use crate::db::{DbConnection, RepositoryError};
use crate::models::admin::MenuItemCheck;
use crate::models::common::{NewHeldOrder, TimeBandEnum};
use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::PgConnection;
use log::{debug, error, info, warn};
use std::cmp::max;
use std::collections::HashMap;

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::held_order_items)]
struct HeldOrderItemInsert {
    hold_id: i32,
    item_id: i32,
    quantity: i16,
}

#[derive(Queryable, Debug)]
struct HeldItemRestore {
    item_id: i32,
    quantity: i16,
}

#[derive(Queryable, Debug)]
#[allow(dead_code)]
struct HeldOrderWithItems {
    hold_id: i32,
    user_id: i32,
    canteen_id: i32,
    total_price: i32,
    deliver_at: Option<TimeBandEnum>,
    item_id: i32,
    quantity: i16,
}

#[derive(Clone)]
pub struct HoldOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    hold_ttl_secs: i64,
}

impl HoldOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>, hold_ttl_secs: i64) -> Self {
        Self {
            pool,
            hold_ttl_secs,
        }
    }

    /// Hold (reserve) an order: validate items, decrement stock, insert into held tables.
    /// Returns (hold_id, expires_at_epoch).
    pub fn hold_order(
        &self,
        userid: i32,
        itemids: Vec<i32>,
        order_deliver_at: Option<String>,
    ) -> Result<(i32, i64), RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("hold_order: failed to acquire DB connection: {}", e);
            e
        })?;

        if itemids.is_empty() {
            return Err(RepositoryError::ValidationError(format!(
                "Order is empty for user: {:?}",
                &userid
            )));
        }

        let mut ordered_qty: HashMap<i32, i64> = HashMap::new();
        for &item in &itemids {
            let qty = ordered_qty.entry(item).or_insert(0);
            *qty += 1;
        }

        let expires_at = Utc::now() + Duration::seconds(self.hold_ttl_secs);

        conn.connection().transaction(|conn| {
            // Validate items and lock rows to prevent concurrent oversells.
            let items_in_order: Vec<MenuItemCheck>;
            let canteen_id_in_order: i32;
            {
                use crate::db::schema::*;
                items_in_order = menu_items::table
                    .filter(menu_items::item_id.eq_any(itemids.clone()))
                    .for_update()
                    .select(MenuItemCheck::as_select())
                    .load::<MenuItemCheck>(conn)
                    .map_err(|e| {
                        error!(
                            "hold_order: error loading menu items for item_ids {:?}: {}",
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
                        return Err(RepositoryError::NotAvailable(
                            item.item_id,
                            item.name.clone(),
                            "Out of stock".to_string(),
                        ));
                    }
                }
            }

            let order_total_price = items_in_order
                .iter()
                .map(|e| e.price * *ordered_qty.get(&e.item_id).unwrap_or(&1) as i32)
                .sum::<i32>();

            let order_deliver_time_enum: Option<TimeBandEnum> =
                TimeBandEnum::get_enum_from_str(order_deliver_at.as_deref());

            // Insert held order
            let new_hold_id: i32;
            let new_held_order = NewHeldOrder {
                user_id: userid,
                canteen_id: canteen_id_in_order,
                total_price: order_total_price,
                deliver_at: order_deliver_time_enum,
                expires_at,
            };
            {
                use crate::db::schema::held_orders::dsl::*;
                new_hold_id = diesel::insert_into(held_orders)
                    .values(&new_held_order)
                    .returning(hold_id)
                    .get_result::<i32>(conn)
                    .map_err(RepositoryError::DatabaseError)?;
            }

            // Insert held order items
            {
                let mut new_items: Vec<HeldOrderItemInsert> = Vec::new();
                for (item, qty) in ordered_qty.iter() {
                    new_items.push(HeldOrderItemInsert {
                        hold_id: new_hold_id,
                        item_id: *item,
                        quantity: *qty as i16,
                    });
                }

                use crate::db::schema::held_order_items::dsl::*;
                diesel::insert_into(held_order_items)
                    .values(&new_items)
                    .execute(conn)
                    .map_err(RepositoryError::DatabaseError)?;
            }

            // Decrement stock
            {
                let mut updated_stock: HashMap<i32, i64> = HashMap::new();
                for item in &items_in_order {
                    updated_stock.insert(
                        item.item_id,
                        max(
                            (item.stock as i64) - *ordered_qty.get(&item.item_id).unwrap_or(&1),
                            -1,
                        ),
                    );
                }

                use crate::db::schema::menu_items::dsl::*;
                for (item, new_stock) in updated_stock {
                    diesel::update(menu_items.filter(item_id.eq(item)))
                        .set((
                            stock.eq(new_stock as i32),
                            is_available.eq(new_stock > 0 || new_stock == -1),
                        ))
                        .execute(conn)
                        .map_err(|e| {
                            error!(
                                "hold_order: error updating stock for item id {}: {}",
                                item, e
                            );
                            RepositoryError::DatabaseError(e)
                        })?;
                }
            }

            debug!(
                "hold_order: created hold {} for user {} with items {:?}, expires at {}",
                new_hold_id, userid, itemids, expires_at
            );

            Ok((new_hold_id, expires_at.timestamp()))
        })
    }

    /// Confirm a held order: move to active_orders, delete from held tables.
    /// Returns the new order_id.
    pub fn confirm_held_order(
        &self,
        search_hold_id: i32,
        requesting_user_id: i32,
    ) -> Result<i32, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("confirm_held_order: failed to acquire DB connection: {}", e);
            e
        })?;

        conn.connection().transaction(|conn| {
            // Fetch held order with items
            let held_data: Vec<HeldOrderWithItems>;
            {
                use crate::db::schema::*;
                held_data = held_orders::table
                    .inner_join(
                        held_order_items::table
                            .on(held_orders::hold_id.eq(held_order_items::hold_id)),
                    )
                    .filter(held_orders::hold_id.eq(search_hold_id))
                    .select((
                        held_orders::hold_id,
                        held_orders::user_id,
                        held_orders::canteen_id,
                        held_orders::total_price,
                        held_orders::deliver_at,
                        held_order_items::item_id,
                        held_order_items::quantity,
                    ))
                    .load::<HeldOrderWithItems>(conn)
                    .map_err(|e| {
                        error!(
                            "confirm_held_order: error fetching hold {}: {}",
                            search_hold_id, e
                        );
                        RepositoryError::DatabaseError(e)
                    })?;
            }

            if held_data.is_empty() {
                return Err(RepositoryError::NotFound(format!(
                    "Hold {} not found",
                    search_hold_id
                )));
            }

            let first = &held_data[0];

            // Ownership check
            if first.user_id != requesting_user_id {
                return Err(RepositoryError::ValidationError(
                    "You do not own this hold".to_string(),
                ));
            }

            // Expiry check — fetch expires_at
            {
                use crate::db::schema::held_orders::dsl::*;
                let hold_expires: chrono::DateTime<chrono::Utc> = held_orders
                    .filter(hold_id.eq(search_hold_id))
                    .select(expires_at)
                    .first::<chrono::DateTime<chrono::Utc>>(conn)
                    .map_err(RepositoryError::DatabaseError)?;

                if Utc::now() > hold_expires {
                    // Hold has expired — clean it up and return error
                    Self::restore_stock_for_hold(conn, search_hold_id)?;
                    diesel::delete(held_orders.filter(hold_id.eq(search_hold_id)))
                        .execute(conn)
                        .map_err(RepositoryError::DatabaseError)?;
                    return Err(RepositoryError::ValidationError(
                        "Hold has expired. Items have been released.".to_string(),
                    ));
                }
            }

            // Create active order
            let new_order_id: i32;
            {
                use crate::db::schema::active_orders::dsl::*;
                new_order_id = diesel::insert_into(active_orders)
                    .values((
                        user_id.eq(first.user_id),
                        canteen_id.eq(first.canteen_id),
                        total_price.eq(first.total_price),
                        deliver_at.eq(&first.deliver_at),
                    ))
                    .returning(order_id)
                    .get_result::<i32>(conn)
                    .map_err(RepositoryError::DatabaseError)?;
            }

            // Create active order items
            {
                use crate::db::schema::active_order_items::dsl::*;
                for row in &held_data {
                    diesel::insert_into(active_order_items)
                        .values((
                            order_id.eq(new_order_id),
                            item_id.eq(row.item_id),
                            quantity.eq(row.quantity),
                        ))
                        .execute(conn)
                        .map_err(RepositoryError::DatabaseError)?;
                }
            }

            // Delete held order (cascade deletes items)
            {
                use crate::db::schema::held_orders::dsl::*;
                diesel::delete(held_orders.filter(hold_id.eq(search_hold_id)))
                    .execute(conn)
                    .map_err(RepositoryError::DatabaseError)?;
            }

            debug!(
                "confirm_held_order: hold {} confirmed as order {} for user {}",
                search_hold_id, new_order_id, requesting_user_id
            );

            Ok(new_order_id)
        })
    }

    /// Release a held order: restore stock, delete from held tables.
    pub fn release_held_order(
        &self,
        search_hold_id: i32,
        requesting_user_id: i32,
    ) -> Result<(), RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("release_held_order: failed to acquire DB connection: {}", e);
            e
        })?;

        conn.connection().transaction(|conn| {
            // Verify ownership
            {
                use crate::db::schema::held_orders::dsl::*;
                let hold_user: i32 = held_orders
                    .filter(hold_id.eq(search_hold_id))
                    .select(user_id)
                    .first::<i32>(conn)
                    .map_err(|e| match e {
                        Error::NotFound => {
                            RepositoryError::NotFound(format!("Hold {} not found", search_hold_id))
                        }
                        other => RepositoryError::DatabaseError(other),
                    })?;

                if hold_user != requesting_user_id {
                    return Err(RepositoryError::ValidationError(
                        "You do not own this hold".to_string(),
                    ));
                }
            }

            // Restore stock
            Self::restore_stock_for_hold(conn, search_hold_id)?;

            // Delete held order (cascade deletes items)
            {
                use crate::db::schema::held_orders::dsl::*;
                diesel::delete(held_orders.filter(hold_id.eq(search_hold_id)))
                    .execute(conn)
                    .map_err(RepositoryError::DatabaseError)?;
            }

            debug!(
                "release_held_order: hold {} released for user {}",
                search_hold_id, requesting_user_id
            );

            Ok(())
        })
    }

    /// Clean up all expired holds: restore stock and delete.
    /// Returns the number of expired holds cleaned up.
    pub fn cleanup_expired_holds(&self) -> Result<usize, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "cleanup_expired_holds: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        // Find expired hold IDs
        let expired_hold_ids: Vec<i32>;
        {
            use crate::db::schema::held_orders::dsl::*;
            expired_hold_ids = held_orders
                .filter(expires_at.lt(Utc::now()))
                .select(hold_id)
                .load::<i32>(conn.connection())
                .map_err(RepositoryError::DatabaseError)?;
        }

        if expired_hold_ids.is_empty() {
            return Ok(0);
        }

        let count = expired_hold_ids.len();
        info!(
            "cleanup_expired_holds: found {} expired holds to clean up",
            count
        );

        for expired_id in &expired_hold_ids {
            conn.connection()
                .transaction::<(), RepositoryError, _>(|conn| {
                    Self::restore_stock_for_hold(conn, *expired_id)?;

                    use crate::db::schema::held_orders::dsl::*;
                    diesel::delete(held_orders.filter(hold_id.eq(expired_id)))
                        .execute(conn)
                        .map_err(RepositoryError::DatabaseError)?;

                    debug!(
                        "cleanup_expired_holds: cleaned up expired hold {}",
                        expired_id
                    );
                    Ok(())
                })?;
        }

        warn!("cleanup_expired_holds: released {} expired holds", count);
        Ok(count)
    }

    /// Restore stock for all items in a held order. Must be called within a transaction.
    fn restore_stock_for_hold(
        conn: &mut PgConnection,
        search_hold_id: i32,
    ) -> Result<(), RepositoryError> {
        let items: Vec<HeldItemRestore>;
        {
            use crate::db::schema::held_order_items::dsl::*;
            items = held_order_items
                .filter(hold_id.eq(search_hold_id))
                .select((item_id, quantity))
                .load::<HeldItemRestore>(conn)
                .map_err(RepositoryError::DatabaseError)?;
        }

        use crate::db::schema::menu_items;
        for item in &items {
            // Only restore stock for items that don't have unlimited stock (-1)
            let current_stock: i32 = menu_items::table
                .filter(menu_items::item_id.eq(item.item_id))
                .select(menu_items::stock)
                .first::<i32>(conn)
                .map_err(RepositoryError::DatabaseError)?;

            if current_stock != -1 {
                let new_stock = current_stock + item.quantity as i32;
                diesel::update(menu_items::table.filter(menu_items::item_id.eq(item.item_id)))
                    .set((
                        menu_items::stock.eq(new_stock),
                        menu_items::is_available.eq(true),
                    ))
                    .execute(conn)
                    .map_err(|e| {
                        error!(
                            "restore_stock_for_hold: error restoring stock for item {}: {}",
                            item.item_id, e
                        );
                        RepositoryError::DatabaseError(e)
                    })?;
            }
        }

        Ok(())
    }
}
