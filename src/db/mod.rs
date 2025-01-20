use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{r2d2, PgConnection};

mod admin;
mod errors;
pub mod schema;
pub mod users;

pub use admin::canteen::CanteenOperations;
pub use admin::menu::MenuOperations;
pub use errors::RepositoryError;
pub use users::user::UserOperations;

pub fn establish_connection_pool(database_url: &str) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder().max_size(20).build(manager).unwrap()
}

// Connection Guard - Manages pool
pub struct DbConnection<'a> {
    conn: r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl DbConnection<'_> {
    pub fn new(pool: &Pool<ConnectionManager<PgConnection>>) -> Result<Self, RepositoryError> {
        Ok(Self {
            conn: pool.get().map_err(RepositoryError::ConnectionPoolError)?,
            _lifetime: std::marker::PhantomData,
        })
    }

    pub fn connection(&mut self) -> &mut PgConnection {
        &mut self.conn
    }
}
