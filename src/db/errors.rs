use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Record not found: {0}")]
    NotFound(String),
    #[error("Database operation error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Error acquiring connection from pool: {0}")]
    ConnectionPoolError(#[from] diesel::r2d2::PoolError),
    #[error("Item with ID {0} ('{1}') is not available: {2}")]
    NotAvailable(i32, String, String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
