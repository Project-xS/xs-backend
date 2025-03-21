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
}

#[derive(Debug)]
pub struct S3Error(String);

impl<T: aws_sdk_s3::error::ProvideErrorMetadata> From<T> for S3Error {
    fn from(value: T) -> Self {
        S3Error(format!(
            "{}: {}",
            value
                .code()
                .map(String::from)
                .unwrap_or("unknown code".into()),
            value
                .message()
                .map(String::from)
                .unwrap_or("missing reason".into()),
        ))
    }
}

impl std::error::Error for S3Error {}

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
