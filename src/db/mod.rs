use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{r2d2, PgConnection};

pub mod schema;
pub mod users;
mod errors;

pub use errors::RepositoryError;
pub use users::user::UserOperations;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection_pool(database_url: &str) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder()
        .build(manager)
        .unwrap()
}

// Connection Guard - Manages pool
pub struct DbConnection<'a> {
    conn: r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> DbConnection<'a> {
    pub fn new(pool: &DbPool) -> Result<Self, RepositoryError> {
        Ok(Self {
            conn: pool.get().map_err(|e| RepositoryError::ConnectionPoolError(e))?,
            _lifetime: std::marker::PhantomData,
        })
    }

    pub fn connection(&mut self) -> &mut PgConnection {
        &mut self.conn
    }
}
