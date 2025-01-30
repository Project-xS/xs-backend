use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Entity not found: {0}")]
    NotFound(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Connection pool error: {0}")]
    ConnectionPoolError(#[from] diesel::r2d2::PoolError),
    #[error("Entity {0} cannot be ordered: {1} - Reason: {2}")]
    NotAvailable(i32, String, String)
}
