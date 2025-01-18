use crate::db::errors::RepositoryError;
use crate::db::schema::canteens::dsl::*;
use crate::db::DbConnection;
use crate::models::admin::{Canteen, NewCanteen};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

pub struct CanteenOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl CanteenOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub fn create_canteen(&self, canteen: NewCanteen) -> Result<usize, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        diesel::insert_into(canteens)
            .values(&canteen)
            .execute(conn.connection())
            .map_err(RepositoryError::DatabaseError)
    }

    // pub fn delete_canteen(&self, id: i32) -> Result<usize, RepositoryError> {
    //     todo!()
    // }

    // pub fn edit_canteen(&self) {
    //     todo!()
    // }

    pub fn get_all_canteens(&self) -> Result<Vec<Canteen>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        canteens
            .load::<Canteen>(conn.connection())
            .map_err(RepositoryError::DatabaseError)
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
