use sqlx::{postgres::PgPoolOptions, Executor, Pool, Postgres};

use crate::{Error, Result};

pub type Db = Pool<Postgres>;

pub async fn new_db_pool(db_connect_url: &str, max_connections: u32) -> Result<Db> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(db_connect_url)
        .await
        .map_err(|e| Error::FailedToCreateDBPool(e.to_string()))
}

/// Acts as an interface to a db connection pool that is Clone + Send + Sync.
///
/// This type can be cloned freely, as the underlying pool is already a smart
/// pointer.
#[derive(Debug, Clone)]
pub struct DbModelManager {
    db: Db,
}

impl DbModelManager {
    pub async fn new(db_connect_url: &str, max_connections: u32) -> Result<Self> {
        let db = new_db_pool(db_connect_url, max_connections).await?;

        Ok(DbModelManager { db })
    }

    pub fn new_from_pool(pool: Db) -> Self {
        pool.into()
    }

    pub async fn check_db_connectivity(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(self.db()).await?;
        Ok(())
    }

    /// Begin a new transaction.
    pub async fn begin(&self) -> Result<Transaction> {
        let mut raw = self.db().begin().await?;
        raw.execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;")
            .await?;

        Ok(Transaction(raw))
    }

    /// Get a reference to the db pool. Can only be used within this crate.
    pub(crate) fn db(&self) -> &Db {
        &self.db
    }
}

impl From<Db> for DbModelManager {
    fn from(db: Db) -> Self {
        Self { db }
    }
}

/// We wrap a transaction in this type to prevent a caller from outside of this
/// crate having direct access to the transaction, and therefore access to an
/// executor that can manipulate the database without going through the exposed
/// interfaces of this crate.
pub struct Transaction<'a>(sqlx::Transaction<'a, Postgres>);

impl Transaction<'_> {
    pub(crate) fn executor(&mut self) -> impl Executor<Database = Postgres> {
        &mut *self.0
    }

    /// Commit the underlying transaction.
    pub async fn commit(self) -> Result<()> {
        Ok(self.0.commit().await?)
    }

    /// Roll the underlying transaction back.
    pub async fn rollback(self) -> Result<()> {
        Ok(self.0.rollback().await?)
    }
}

/// Indicates that this type can provide an executor.
pub trait AsExecutor: private::ActualExecutor {
    fn is_transaction(&self) -> bool {
        false
    }
}

// by splitting the trait, we can expose functions generic over implementors
// of the public trait, but have the actual function for getting the executor
// in the private interface. This prevents people from getting at the raw
// executor outside of the context of this crate. If they _could_ get at this,
// then it would be possible to execute a query against the db outside of this
// crate.
pub(crate) mod private {
    use sqlx::{Executor, Postgres};

    pub trait ActualExecutor {
        fn as_executor(&mut self) -> impl Executor<Database = Postgres>;
    }
}

impl private::ActualExecutor for Transaction<'_> {
    fn as_executor(&mut self) -> impl Executor<Database = Postgres> {
        self.executor()
    }
}

impl AsExecutor for Transaction<'_> {
    fn is_transaction(&self) -> bool {
        true
    }
}

impl private::ActualExecutor for DbModelManager {
    // an unfortunate side-effect of supporting transactions is that we need
    // as_executor to operate on mutable references. This means that, when using
    // the db model manager as the executor, we need a mutable reference,
    // despite nothing actually requiring it to be mutable other than this
    // interface.
    fn as_executor(&mut self) -> impl Executor<Database = Postgres> {
        self.db()
    }
}

// the model manager by itself is not a transaction
impl AsExecutor for DbModelManager {}
