use crate::db::errors::RepositoryError;
use crate::db::schema::menu_items::dsl::*;
use crate::db::DbConnection;
use crate::models::admin::{MenuItem, NewMenuItem, UpdateMenuItem};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;

use log::error;

pub struct MenuOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl MenuOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub fn add_menu_item(&self, menu_item: NewMenuItem) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("add_menu_item: failed to acquire DB connection: {}", e);
            e
        })?;

        diesel::insert_into(menu_items)
            .values(&menu_item)
            .get_result(conn.connection())
            .map_err(|e| {
                error!(
                    "add_menu_item: error inserting menu item '{}': {}",
                    menu_item.name, e
                );
                RepositoryError::DatabaseError(e)
            })
    }

    pub fn remove_menu_item(&self, id: i32) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "remove_menu_item: failed to acquire DB connection for id {}: {}",
                id, e
            );
            e
        })?;

        diesel::delete(menu_items.filter(item_id.eq(id)))
            .get_result(conn.connection())
            .map_err(|e| {
                error!(
                    "remove_menu_item: error deleting menu item with id {}: {}",
                    id, e
                );
                match e {
                    Error::NotFound => RepositoryError::NotFound(format!("menu_items: {}", id)),
                    other => RepositoryError::DatabaseError(other),
                }
            })
    }

    pub fn update_menu_item(
        &self,
        itemid: i32,
        changed_menu_item: UpdateMenuItem,
    ) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "update_menu_item: failed to acquire DB connection for id {}: {}",
                itemid, e
            );
            e
        })?;

        diesel::update(menu_items.filter(item_id.eq(itemid)))
            .set(&changed_menu_item)
            .get_result(conn.connection())
            .map_err(|e| {
                error!(
                    "update_menu_item: error updating menu item with id {}: {}",
                    itemid, e
                );
                match e {
                    Error::NotFound => RepositoryError::NotFound(format!("menu_items: {}", itemid)),
                    other => RepositoryError::DatabaseError(other),
                }
            })
    }

    pub fn get_all_menu_items(&self) -> Result<Vec<MenuItem>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("get_all_menu_items: failed to acquire DB connection: {}", e);
            e
        })?;

        menu_items
            .order_by(item_id.asc())
            .load::<MenuItem>(conn.connection())
            .map_err(|e| {
                error!("get_all_menu_items: error fetching menu items: {}", e);
                RepositoryError::DatabaseError(e)
            })
    }

    pub fn get_menu_item(&self, itemid: i32) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_menu_item: failed to acquire DB connection for id {}: {}",
                itemid, e
            );
            e
        })?;

        menu_items
            .filter(item_id.eq(itemid))
            .limit(1)
            .get_result(conn.connection())
            .map_err(|e| {
                error!(
                    "get_menu_item: error fetching menu item with id {}: {}",
                    itemid, e
                );
                match e {
                    Error::NotFound => RepositoryError::NotFound(format!("menu_items: {}", itemid)),
                    other => RepositoryError::DatabaseError(other),
                }
            })
    }
}

impl Clone for MenuOperations {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
