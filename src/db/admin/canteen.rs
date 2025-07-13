use crate::db::errors::RepositoryError;
use crate::db::schema::canteens::dsl::*;
use crate::db::DbConnection;
use crate::models::admin::{Canteen, CanteenLoginSuccess, MenuItem, NewCanteen};
use diesel::dsl::{case_when, sql};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::sql_types::{Bool, Text};
use log::error;

pub struct CanteenOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl CanteenOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
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

    pub fn set_canteen_pic(&self, canteen_id_to_update: &i32) -> Result<usize, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "approve_canteen_pic: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        diesel::update(canteens.filter(canteen_id.eq(canteen_id_to_update)))
            .set(pic_link.eq(true))
            .execute(conn.connection())
            .map_err(|e| {
                error!(
                    "approve_canteen_pic: error approving pic for menu item with id {}: {}",
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

    pub fn get_all_canteens(&self) -> Result<Vec<Canteen>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("get_all_canteens: failed to acquire DB connection: {}", e);
            e
        })?;

        canteens
            .order_by(canteen_id.asc())
            .load::<Canteen>(conn.connection())
            .map_err(|e| {
                error!("get_all_canteens: error fetching canteens: {}", e);
                RepositoryError::DatabaseError(e)
            })
    }

    pub fn get_canteen_items(
        &self,
        search_canteen_id: i32,
    ) -> Result<Vec<MenuItem>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("get_canteen_items: failed to acquire DB connection: {}", e);
            e
        })?;

        use crate::db::schema::menu_items::dsl::*;
        menu_items
            .filter(canteen_id.eq(search_canteen_id))
            .order(item_id.asc())
            .load::<MenuItem>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_canteen_items: error fetching canteen items for {:?}: {}",
                    search_canteen_id, e
                );
                RepositoryError::DatabaseError(e)
            })
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
        }
    }
}
