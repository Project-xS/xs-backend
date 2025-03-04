use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{r2d2, PgConnection};

mod admin;
mod common;
mod errors;
pub mod schema;
mod users;

pub use admin::canteen::CanteenOperations;
pub use admin::menu::MenuOperations;
pub use common::orders::OrderOperations;
pub use common::search::SearchOperations;
pub use errors::RepositoryError;
pub use users::user::UserOperations;

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use log::{info, error};

pub fn establish_connection_pool(database_url: &str) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    info!("Initializing database connection pool...");
    Pool::builder().max_size(50).build(manager).expect("Failed to create DB connection pool")
}

pub fn run_db_migrations(db: Pool<ConnectionManager<PgConnection>>) -> Result<(), RepositoryError> {
    let mut conn = DbConnection::new(&db)?.conn;
    info!("Running database migrations...");
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    Ok(())
}

// Connection Guard - Manages pool
pub struct DbConnection<'a> {
    conn: r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl DbConnection<'_> {
    pub fn new(pool: &Pool<ConnectionManager<PgConnection>>) -> Result<Self, RepositoryError> {
        Ok(Self {
            conn: pool.get().map_err(|e| {
                error!("DbConnection::new: failed to acquire connection from pool: {}", e);
                RepositoryError::ConnectionPoolError(e)
            })?,
            _lifetime: std::marker::PhantomData,
        })
    }

    pub fn connection(&mut self) -> &mut PgConnection {
        &mut self.conn
    }
}
