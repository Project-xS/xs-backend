use crate::db::errors::RepositoryError;
use crate::db::schema::canteens::dsl::*;
use crate::db::{AssetOperations, DbConnection};
use crate::enums::admin::{CanteenDetailsWithPic, MenuItemWithPic};
use crate::models::admin::{Canteen, CanteenDetails, CanteenLoginSuccess, MenuItem, NewCanteen};
use diesel::dsl::{case_when, sql};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::sql_types::{Bool, Text};
use futures::future::join_all;
use log::error;

pub struct CanteenOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    asset_ops: AssetOperations,
}

impl CanteenOperations {
    pub async fn new(
        pool: Pool<ConnectionManager<PgConnection>>,
        asset_ops: AssetOperations,
    ) -> Self {
        Self { pool, asset_ops }
    }

    pub fn create_canteen(&self, canteen: NewCanteen) -> Result<usize, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("create_canteen: failed to acquire DB connection: {}", e);
            e
        })?;

        diesel::insert_into(canteens)
            .values(&canteen)
            .execute(conn.connection())
            .map_err(|e| {
                error!(
                    "create_canteen: error inserting canteen '{}': {}",
                    canteen.canteen_name, e
                );
                RepositoryError::DatabaseError(e)
            })
    }

    pub async fn upload_canteen_pic(
        &self,
        canteen_id_to_set: &i32,
    ) -> Result<String, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("upload_canteen_pic: failed to acquire DB connection: {}", e);
            e
        })?;

        canteens
            .filter(canteen_id.eq(canteen_id_to_set))
            .first::<Canteen>(conn.connection())
            .map_err(|e| {
                error!(
                    "upload_canteen_pic: error generating presign upload {}: {}",
                    canteen_id_to_set, e
                );
                match e {
                    Error::NotFound => {
                        RepositoryError::NotFound(format!("canteens: {canteen_id_to_set}"))
                    }
                    other => RepositoryError::DatabaseError(other),
                }
            })?;

        let res = self
            .asset_ops
            .get_upload_presign_url(&format!("canteens/{}", canteen_id_to_set))
            .await?;

        Ok(res)
    }

    pub async fn set_canteen_pic(
        &self,
        canteen_id_to_update: &i32,
    ) -> Result<usize, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("set_menu_item_pic: failed to acquire DB connection: {}", e);
            e
        })?;

        let etag = self
            .asset_ops
            .get_object_etag(&format!("canteens/{}", canteen_id_to_update))
            .await?;
        if etag.is_some() {
            info!("set_menu_item_pic: etag: {}", etag.clone().unwrap());
        } else {
            info!("set_menu_item_pic: no etag found");
        }

        diesel::update(canteens.filter(canteen_id.eq(canteen_id_to_update)))
            .set(has_pic.eq(true))
            .execute(conn.connection())
            .map_err(|e| {
                error!(
                    "set_menu_item_pic: error approving pic for menu item with id {}: {}",
                    canteen_id_to_update, e
                );
                match e {
                    Error::NotFound => {
                        RepositoryError::NotFound(format!("canteens: {canteen_id_to_update}"))
                    }
                    other => RepositoryError::DatabaseError(other),
                }
            })
    }

    // pub fn delete_canteen(&self, id: i32) -> Result<usize, RepositoryError> {
    //     todo!()
    // }

    // pub fn edit_canteen(&self) {
    //     todo!()
    // }

    pub async fn get_all_canteens(&self) -> Result<Vec<CanteenDetailsWithPic>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("get_all_canteens: failed to acquire DB connection: {}", e);
            e
        })?;

        let canteen_details = canteens
            .order_by(canteen_id.asc())
            .select(CanteenDetails::as_select())
            .load::<CanteenDetails>(conn.connection())
            .map_err(|e| {
                error!("get_all_canteens: error fetching canteens: {}", e);
                RepositoryError::DatabaseError(e)
            })?;

        let futures = canteen_details.iter().map(async |canteen| {
            let mut canteen_with_pic: CanteenDetailsWithPic = canteen.into();
            canteen_with_pic
                .populate_pic_link_from(&self.asset_ops, canteen)
                .await;
            canteen_with_pic
        });

        let results = join_all(futures).await;
        Ok(results)
    }

    pub async fn get_canteen_items(
        &self,
        search_canteen_id: i32,
    ) -> Result<Vec<MenuItemWithPic>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("get_canteen_items: failed to acquire DB connection: {}", e);
            e
        })?;

        use crate::db::schema::menu_items::dsl::*;
        let items = menu_items
            .filter(canteen_id.eq(search_canteen_id))
            .order(item_id.asc())
            .load::<MenuItem>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_canteen_items: error fetching canteen items for {:?}: {}",
                    search_canteen_id, e
                );
                RepositoryError::DatabaseError(e)
            })?;

        let futures = items.iter().map(async |item| {
            let mut item_with_pic: MenuItemWithPic = item.into();
            item_with_pic
                .populate_pic_link_from(&self.asset_ops, item)
                .await;
            item_with_pic
        });

        let results = join_all(futures).await;
        Ok(results)
    }

    pub fn login_canteen(
        &self,
        try_username: &str,
        try_password: &str,
    ) -> Result<Option<CanteenLoginSuccess>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("login_canteen: failed to acquire DB connection: {}", e);
            e
        })?;

        let query = canteens.filter(username.eq(try_username)).select((
            canteen_id,
            canteen_name,
            case_when::<_, _, Bool>(
                password.eq(sql::<Text>("crypt(")
                    .bind::<Text, _>(try_password)
                    .sql(", password)")),
                true,
            )
            .otherwise(false),
        ));

        match query.get_result::<(i32, String, bool)>(conn.connection()) {
            Ok(resp) => {
                let (matched_canteen_id, matched_canteen_name, is_password_correct) = resp;
                if is_password_correct {
                    Ok(Some(CanteenLoginSuccess {
                        canteen_id: matched_canteen_id,
                        canteen_name: matched_canteen_name,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(Error::NotFound) => Ok(None),
            Err(e) => {
                error!(
                    "login_canteen: error logging in canteen {:?}: {}",
                    try_username, e
                );
                Err(RepositoryError::DatabaseError(e))
            }
        }
    }
}

impl Clone for CanteenOperations {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            asset_ops: self.asset_ops.clone(),
        }
    }
}
