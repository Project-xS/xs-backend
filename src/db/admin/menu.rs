use crate::db::errors::RepositoryError;
use crate::db::schema::menu_items::dsl::*;
use crate::db::DbConnection;
use crate::models::admin::{MenuItem, NewMenuItem, UpdateMenuItem};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;

pub struct MenuOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl MenuOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub fn add_menu_item(&self, menu_item: NewMenuItem) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        diesel::insert_into(menu_items)
            .values(&menu_item)
            .get_result(conn.connection())
            .map_err(RepositoryError::DatabaseError)
    }

    pub fn remove_menu_item(&self, id: i32) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        diesel::delete(menu_items.filter(item_id.eq(id)))
            .get_result(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(format!("menu_items: {}", id)),
                other => RepositoryError::DatabaseError(other),
            })
    }

    pub fn update_menu_item(
        &self,
        itemid: i32,
        changed_menu_item: UpdateMenuItem,
    ) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        diesel::update(menu_items.filter(item_id.eq(itemid)))
            .set(&changed_menu_item)
            .get_result(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(format!("menu_items: {}", itemid)),
                other => RepositoryError::DatabaseError(other),
            })
    }

    pub fn get_all_menu_items(&self) -> Result<Vec<MenuItem>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        menu_items
            .load::<MenuItem>(conn.connection())
            .map_err(RepositoryError::DatabaseError)
    }

    pub fn get_menu_item(&self, itemid: i32) -> Result<MenuItem, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        menu_items
            .filter(item_id.eq(itemid))
            .limit(1)
            .get_result(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(format!("menu_items: {}", itemid)),
                other => RepositoryError::DatabaseError(other),
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
