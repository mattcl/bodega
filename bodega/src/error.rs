use snafu::Snafu;

use crate::{DbBmcError, DbModelManagerError, OpError};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Attempted empty update for '{entity}' with id '{id}'"))]
    EmptyUpdate { entity: &'static str, id: String },

    #[snafu(display("Could not find '{entity}' with id '{id}'"))]
    EntityNotFound { entity: &'static str, id: String },

    #[snafu(display("Failed to create DB pool: {message}"))]
    FailedToCreateDBPool { message: String },

    #[snafu(display("DbBmc error: "))]
    DbBmc { source: DbBmcError },

    #[snafu(display("ModelManagr error: "))]
    ModelManager { source: DbModelManagerError },

    #[snafu(display("Transaction serialization error: "))]
    TransactionSerialization { source: SerializationError },

    #[snafu(display("Transaction retries exceeded: "))]
    TransactionRetriesExceeded { source: Box<Error> },

    #[snafu(transparent)]
    SqlxMigrate { source: sqlx::migrate::MigrateError },
}

impl Error {
    /// If applicable, returns the name of the constraint that triggered the error.
    ///
    /// This is a convenience proxy for the constraint on the underlying [`sqlx::Error`].
    pub fn constraint(&self) -> Option<&str> {
        match self {
            Error::DbBmc {
                source:
                    DbBmcError::Operation {
                        source:
                            OpError::Sqlx {
                                source: sqlx::Error::Database(ref e),
                            },
                        ..
                    },
            } => e.constraint(),
            Error::ModelManager { source } => match source.source() {
                sqlx::Error::Database(ref e) => e.constraint(),
                _ => None,
            },
            _ => None,
        }
    }
}

#[derive(Debug, Snafu)]
pub enum SerializationError {
    #[snafu(transparent)]
    DbBmc { source: DbBmcError },

    #[snafu(transparent)]
    ModelManager { source: DbModelManagerError },
}

// we want to explicitly distinguish serialization errors to make it easier for
// clients to retry.
impl From<DbBmcError> for Error {
    fn from(value: DbBmcError) -> Self {
        match value {
            DbBmcError::Operation {
                source:
                    OpError::Sqlx {
                        source: sqlx::Error::Database(ref e),
                    },
                ..
            } if e.code() == Some("40001".into()) => Self::TransactionSerialization {
                source: SerializationError::DbBmc { source: value },
            },
            _ => Self::DbBmc { source: value },
        }
    }
}

impl From<DbModelManagerError> for Error {
    fn from(value: DbModelManagerError) -> Self {
        match value {
            DbModelManagerError::TransactionCommit {
                source: sqlx::Error::Database(ref e),
            } if e.code() == Some("40001".into()) => Self::TransactionSerialization {
                source: SerializationError::ModelManager { source: value },
            },
            _ => Self::ModelManager { source: value },
        }
    }
}
