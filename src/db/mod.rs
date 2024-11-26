use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{r2d2, PgConnection};

pub mod schema;
pub mod handlers;
mod errors;

pub use errors::RepositoryError;
pub use handlers::user::UserOperations;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection_pool(database_url: &str) -> Result<DbPool, r2d2::PoolError> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder()
        .build(manager)
}

// Connection Guard
pub struct DbConnection<'a> {
    conn: r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> DbConnection<'a> {
    pub fn new(pool: &DbPool) -> Result<Self, RepositoryError> {
        Ok(Self {
            conn: pool.get().map_err(|e| RepositoryError::DatabaseError(e.into()))?,
            _lifetime: std::marker::PhantomData,
        })
    }

    pub fn connection(&mut self) -> &mut PgConnection {
        &mut self.conn
    }
}