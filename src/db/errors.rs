use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Entity not found")]
    NotFound,
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
}