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
    #[allow(dead_code)]
    InternalError(String),

    #[error("Error fetching assets: {0}")]
    AssetError(String),
}

#[derive(Debug, Error)]
pub enum S3Error {
    #[error("File not found in S3 for key: {0}")]
    NotFound(String),
    #[error("S3 Service error: {0}")]
    S3ServiceError(String),
    #[error("S3 operation failed: {0}")]
    S3OperationFailed(String),
}

impl From<S3Error> for RepositoryError {
    fn from(error: S3Error) -> RepositoryError {
        RepositoryError::AssetError(error.to_string())
    }
}
