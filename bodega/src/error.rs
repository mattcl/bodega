pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Attempted empty update for '{entity}' with id '{id}'")]
    EmptyUpdate { entity: &'static str, id: String },

    #[error("Could not find '{entity}' with id '{id}'")]
    EntityNotFound { entity: &'static str, id: String },

    #[error("Failed to create DB pool: {0}")]
    FailedToCreateDBPool(String),

    #[error("Transaction serialization error: {0}")]
    SerializationError(Box<dyn sqlx::error::DatabaseError>),

    #[error("Transaction retries exceeded: {0}")]
    TransactionRetriesExceeded(Box<Error>),

    #[error(transparent)]
    TransformIntError(#[from] std::num::TryFromIntError),

    #[error(transparent)]
    Sqlx(sqlx::Error),

    #[error(transparent)]
    SqlxMigrate(#[from] sqlx::migrate::MigrateError),
}

// we want to explicitly distinguish serialization errors to make it easier for
// clients to retry.
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::Database(e) if e.code() == Some("40001".into()) => {
                Self::SerializationError(e)
            }
            _ => Self::Sqlx(value),
        }
    }
}
