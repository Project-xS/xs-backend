use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use crate::models::user::{NewUser, User};
use crate::db::errors::RepositoryError;
use crate::db::schema::users;
use crate::db::schema::users::email;
use crate::db::DbConnection;

pub struct UserOperations {
    pool: Pool<ConnectionManager<PgConnection>>
}

impl UserOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub fn create_user(&self, new_user: NewUser) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        diesel::insert_into(users::table)
            .values(&new_user)
            .get_result(conn.connection())
            .map_err(|e| RepositoryError::DatabaseError(e))
    }

    pub fn get_user_by_rfid(&self, rfid: i32) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        users::table
            .find(rfid)
            .get_result::<User>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(rfid.to_string()),
                other => RepositoryError::DatabaseError(other)
            })
    }

    pub fn get_user_by_email(&self, email_addr: &str) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        users::table
            .filter(email.eq(email_addr))
            .get_result::<User>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(email_addr.to_string()),
                other => RepositoryError::DatabaseError(other)
            })
    }
}

impl Clone for UserOperations {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}