use crate::db::errors::RepositoryError;
use crate::db::schema::menu_items::dsl::*;
use crate::db::{AssetOperations, DbConnection, S3Error};
use crate::enums::admin::MenuItemWithPic;
use crate::models::admin::{MenuItem, NewMenuItem, UpdateMenuItem};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use futures::future::join_all;
use log::error;

pub struct MenuOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    asset_ops: AssetOperations,
}

impl MenuOperations {
    pub async fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            pool,
            asset_ops: AssetOperations::new().await.unwrap(),
        }
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

    pub async fn upload_menu_item_pic(&self, menu_item_to_set: &i32) -> Result<String, S3Error> {
        self.asset_ops
            .get_upload_presign_url(&format!("items/{}", menu_item_to_set))
            .await
    }

    pub async fn set_menu_item_pic(
        &self,
        item_id_to_update: &i32,
    ) -> Result<usize, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("set_menu_item_pic: failed to acquire DB connection: {}", e);
            e
        })?;
        info!(
            "set_menu_item_pic: item_id_to_update: {}",
            item_id_to_update
        );

        let etag = self
            .asset_ops
            .get_object_etag(&item_id_to_update.to_string())
            .await?;
        if etag.is_some() {
            info!("set_menu_item_pic: etag: {}", etag.clone().unwrap());
        } else {
            info!("set_menu_item_pic: no etag found");
        }

        diesel::update(menu_items.filter(item_id.eq(item_id_to_update)))
            .set((pic_link.eq(true), pic_etag.eq(etag)))
            .execute(conn.connection())
            .map_err(|e| {
                error!(
                    "approve_menu_item_pic: error approving pic for menu item with id {}: {}",
                    item_id_to_update, e
                );
                match e {
                    Error::NotFound => {
                        RepositoryError::NotFound(format!("menu_items: {item_id_to_update}"))
                    }
                    other => RepositoryError::DatabaseError(other),
                }
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
                    Error::NotFound => RepositoryError::NotFound(format!("menu_items: {id}")),
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
                    Error::NotFound => RepositoryError::NotFound(format!("menu_items: {itemid}")),
                    other => RepositoryError::DatabaseError(other),
                }
            })
    }

    pub async fn get_all_menu_items(&self) -> Result<Vec<MenuItemWithPic>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("get_all_menu_items: failed to acquire DB connection: {}", e);
            e
        })?;

        let items = menu_items
            .order_by(item_id.asc())
            .load::<MenuItem>(conn.connection())
            .map_err(|e| {
                error!("get_all_menu_items: error fetching menu items: {}", e);
                RepositoryError::DatabaseError(e)
            })?;

        let futures = items.iter().map(async |item| {
            let mut item_with_pic: MenuItemWithPic = item.into();
            if item.pic_link {
                let pic_url = self
                    .asset_ops
                    .get_object_presign(&item.item_id.to_string())
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

    pub async fn get_menu_item(&self, itemid: i32) -> Result<MenuItemWithPic, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_menu_item: failed to acquire DB connection for id {}: {}",
                itemid, e
            );
            e
        })?;

        let item = menu_items
            .filter(item_id.eq(itemid))
            .first::<MenuItem>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_menu_item: error fetching menu item with id {}: {}",
                    itemid, e
                );
                match e {
                    Error::NotFound => RepositoryError::NotFound(format!("menu_items: {itemid}")),
                    other => RepositoryError::DatabaseError(other),
                }
            })?;

        let mut item_with_pic: MenuItemWithPic = (&item).into();
        if item.pic_link {
            let pic_url = self
                .asset_ops
                .get_object_presign(&item.item_id.to_string())
                .await
                .ok();
            item_with_pic.pic_link = pic_url;
            Ok(item_with_pic)
        } else {
            Ok(item_with_pic)
        }
    }
}

impl Clone for MenuOperations {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            asset_ops: self.asset_ops.clone(),
        }
    }
}
