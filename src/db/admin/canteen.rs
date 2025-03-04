use crate::db::errors::RepositoryError;
use crate::db::schema::canteens::dsl::*;
use crate::db::DbConnection;
use crate::models::admin::{Canteen, NewCanteen};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use log::{error};

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
                error!("create_canteen: error inserting canteen '{}': {}", canteen.canteen_name, e);
                RepositoryError::DatabaseError(e)
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
            .load::<Canteen>(conn.connection())
            .map_err(|e| {
                error!("get_all_canteens: error fetching canteens: {}", e);
                RepositoryError::DatabaseError(e)
            })
    }

    // pub fn get_canteen(&self, canteenid: i32) -> Result<MenuItem, RepositoryError> {
    //     todo!();
    // }
}

impl Clone for CanteenOperations {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
