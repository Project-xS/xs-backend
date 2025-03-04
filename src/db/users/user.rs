use crate::db::errors::RepositoryError;
use crate::db::schema::users;
use crate::db::schema::users::email;
use crate::db::DbConnection;
use crate::models::user::{NewUser, User};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;

use log::{error};

pub struct UserOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl UserOperations {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub fn create_user(&self, new_user: NewUser) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("create_user: failed to acquire DB connection: {}", e);
            e
        })?;

        diesel::insert_into(users::table)
            .values(&new_user)
            .get_result(conn.connection())
            .map_err(|e| {
                error!("create_user: error inserting new user with email '{}': {}", new_user.email, e);
                RepositoryError::DatabaseError(e)
            })
    }

    #[allow(dead_code)]
    pub fn get_user_by_rfid(&self, rfid: i32) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("get_user_by_rfid: failed to acquire DB connection for rfid '{}': {}", rfid, e);
            e
        })?;

        users::table
            .find(rfid)
            .limit(1)
            .get_result::<User>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(rfid.to_string()),
                other => RepositoryError::DatabaseError(other),
            })
    }

    pub fn get_user_by_email(&self, email_addr: &str) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        users::table
            .filter(email.eq(email_addr))
            .limit(1)
            .get_result::<User>(conn.connection())
            .map_err(|e| {
                error!("get_user_by_email: error fetching user with email '{}': {}", email_addr, e);
                match e {
                    Error::NotFound => RepositoryError::NotFound(email_addr.to_string()),
                    other => RepositoryError::DatabaseError(other),
                }
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
