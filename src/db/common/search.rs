use crate::db::{DbConnection, RepositoryError};
use crate::models::admin::MenuItem;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_types::{Bool, Text};
use diesel::PgConnection;
use log::{debug, error};

#[derive(Clone)]
pub struct SearchOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl SearchOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    /// Performs a fuzzy search on the menu_items table using the pg_trgm extension.
    /// Returns up to 10 menu items ordered by descending similarity.
    pub fn search_menu_items(&self, search_query: &str) -> Result<Vec<MenuItem>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("search_menu_items: failed to acquire DB connection for query '{}': {}", search_query, e);
            e
        })?;
        debug!("search_menu_items: executing fuzzy search for query '{}'", search_query);
        use crate::db::schema::menu_items::dsl::*;
        // SELECT * FROM menu_items
        //            WHERE name % $1
        //            ORDER BY similarity(name, $1) DESC
        //            LIMIT 500;
        menu_items
            .filter(sql::<Bool>("name % ").bind::<Text, _>(search_query))
            .order_by(
                sql::<Text>("similarity (name, ")
                    .bind::<Text, _>(search_query)
                    .sql(")")
                    .desc(),
            )
            .limit(10)
            .load::<MenuItem>(conn.connection())
            .map_err(|e| {
                error!("search_menu_items: error performing search for query '{}': {}", search_query, e);
                RepositoryError::DatabaseError(e)
            })
    }
}
