use crate::db::{AssetOperations, DbConnection, RepositoryError};
use crate::enums::admin::MenuItemWithPic;
use crate::models::admin::MenuItem;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_types::{Bool, Text};
use diesel::PgConnection;
use futures::future::join_all;
use log::{debug, error};

#[derive(Clone)]
pub struct SearchOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    asset_ops: AssetOperations,
}

impl SearchOperations {
    pub async fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            pool,
            asset_ops: AssetOperations::new().await.unwrap(),
        }
    }

    /// Performs a fuzzy search on the menu_items table using the pg_trgm extension.
    /// Returns up to 10 menu items ordered by descending similarity.
    pub async fn search_menu_items(
        &self,
        search_query: &str,
    ) -> Result<Vec<MenuItemWithPic>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "search_menu_items: failed to acquire DB connection for query '{}': {}",
                search_query, e
            );
            e
        })?;
        debug!(
            "search_menu_items: executing fuzzy search for query '{}'",
            search_query
        );
        use crate::db::schema::menu_items::dsl::*;
        // SELECT * FROM menu_items
        //            WHERE name % $1
        //            ORDER BY similarity(name, $1) DESC
        //            LIMIT 500;
        let items = menu_items
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
                error!(
                    "search_menu_items: error performing search for query '{}': {}",
                    search_query, e
                );
                RepositoryError::DatabaseError(e)
            })?;

        let futures = items.iter().map(async |item| {
            let mut item_with_pic: MenuItemWithPic = item.into();
            if item.pic_link {
                let pic_url = self
                    .asset_ops
                    .get_object_presign(&format!("items/{}", &item.item_id.to_string()))
                    .await
                    .ok();
                item_with_pic.pic_link = pic_url;
                item_with_pic
            } else {
                item_with_pic
            }
        });

        let results = join_all(futures).await;
        Ok(results)
    }

    /// Performs a fuzzy search on the menu_items table using the pg_trgm extension.
    /// Returns up to 10 menu items ordered by descending similarity.
    pub async fn search_menu_items_by_canteen(
        &self,
        from_canteen_id: &i32,
        search_query: &str,
    ) -> Result<Vec<MenuItemWithPic>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "search_menu_items_by_canteen: failed to acquire DB connection for query '{}': {}",
                search_query, e
            );
            e
        })?;
        debug!(
            "search_menu_items_by_canteen: executing fuzzy search for query '{}' for canteen ID: '{}'",
            search_query, from_canteen_id
        );
        use crate::db::schema::menu_items::dsl::*;
        // SELECT * FROM menu_items
        //            WHERE name % $1
        //            AND canteen_id = $2
        //            ORDER BY similarity(name, $1) DESC
        //            LIMIT 500;
        let items = menu_items
            .filter(canteen_id.eq(from_canteen_id))
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
                error!(
                    "search_menu_items_by_canteen: error performing search for query '{}' for canteen '{}': {}",
                    search_query, from_canteen_id, e
                );
                RepositoryError::DatabaseError(e)
            })?;

        let futures = items.iter().map(async |item| {
            let mut item_with_pic: MenuItemWithPic = item.into();
            if item.pic_link {
                let pic_url = self
                    .asset_ops
                    .get_object_presign(&format!("items/{}", &item.item_id.to_string()))
                    .await
                    .ok();
                item_with_pic.pic_link = pic_url;
                item_with_pic
            } else {
                item_with_pic
            }
        });

        let results = join_all(futures).await;
        Ok(results)
    }
}
