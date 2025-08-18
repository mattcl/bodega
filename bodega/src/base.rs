use std::fmt::Display;

use sea_query::{
    DynIden, Expr, PostgresQueryBuilder, Query, SelectStatement, SimpleExpr, TableRef,
};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use sqlx::{postgres::PgRow, prelude::FromRow, Postgres};

use crate::{AsExecutor, Cursored, CursoredFilter, Error, Paginated, Result};

/// Indicates that this type supports selection from the db by defining the
/// columns that should be fetched.
pub trait Select: Send + Unpin + for<'r> FromRow<'r, PgRow> {
    /// Returns a vector of column references for use when selecting rows from
    /// thd db.
    fn select_cols() -> Vec<DynIden>;
}

/// Indicates that this type supports insertion into the db by defining the
/// colums that should be inserted along with their values.
pub trait Insert {
    /// Returns a vector of idens used for inserting a row in the db.
    fn insert_cols(&self) -> Vec<DynIden>;

    /// Returns a vector of values correpsonding to the columns from
    /// `insert_cols`. Used for inserting a row in the db.
    ///
    /// Consumes `self`.
    fn insert_vals(self) -> Vec<SimpleExpr>;
}

/// Indicates that this type supports updating a row in the db by defining
/// (column, value) pairs.
pub trait Update {
    /// Returns a vector of (iden, expression) pairs used for updating a row in
    /// the db.
    fn update_values(self) -> Vec<(DynIden, SimpleExpr)>;
}

/// Indicates that this type can add filtering conditions to select statements.
pub trait Filter {
    fn filter_query(&self, _query: &mut SelectStatement) {
        // nothing by default
    }
}

/// Indicates that this type can be used as an ID for the purposes of model
/// controllers.
pub trait IdType: ToString + Clone + Send + Unpin + sqlx::Type<Postgres> {
    fn id_value(&self) -> sea_query::SimpleExpr;
}

impl<T> IdType for T
where
    T: ToString + Clone + Send + Unpin + sqlx::Type<Postgres>,
    for<'any> &'any T: Into<sea_query::SimpleExpr>,
{
    fn id_value(&self) -> sea_query::SimpleExpr {
        self.into()
    }
}

#[derive(Debug, Snafu)]
pub enum DbBmcError {
    #[snafu(display("Error performing {operation} for entity '{entity}': "))]
    Operation {
        entity: &'static str,
        operation: DbBmcOp,
        source: OpError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub enum DbBmcOp {
    Count,
    Create,
    Delete,
    Get,
    List,
    ListPaginated,
    Update,
}

impl Display for DbBmcOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbBmcOp::Count => "COUNT",
            DbBmcOp::Create => "CREATE",
            DbBmcOp::Delete => "DELETE",
            DbBmcOp::Get => "GET",
            DbBmcOp::List => "LIST",
            DbBmcOp::ListPaginated => "LIST PAGINATED",
            DbBmcOp::Update => "UPDATE",
        }
        .fmt(f)
    }
}

#[derive(Debug, Snafu)]
pub enum OpError {
    #[snafu(display("Sqlx error: "))]
    Sqlx { source: sqlx::Error },

    #[snafu(display("Error converting i64 to usize: "))]
    Usize { source: std::num::TryFromIntError },
}

/// The core trait that defines a type that acts as a database model controller.
///
/// While not _necessary_ for types to implement this to interact with the db,
/// it acts as a useful standardization for most models that have primary keys.
pub trait DbBmc {
    /// The name of the entity this controller acts on (i.e. `User`).
    const ENTITY: &'static str;

    /// The name of the table in the database the entities reside in.
    const TABLE: &'static str;

    type Error;

    /// The type of the ID column
    type IdType: IdType;

    fn id_column() -> DynIden;

    fn id_to_value(id: &Self::IdType) -> sea_query::SimpleExpr {
        id.id_value()
    }

    // provided methods

    fn get_table_ref() -> TableRef {
        TableRef::Table(DynIden::new(Self::TABLE))
    }
}

/// Counts all of the rows in a model manager's table.
pub async fn count<MC, X>(executor: &mut X) -> Result<usize>
where
    MC: DbBmc,
    X: AsExecutor,
{
    let query = Query::select()
        .expr(Expr::col(MC::id_column()).count())
        .from(MC::get_table_ref())
        .to_owned();

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
    let (num,) = sqlx::query_as_with::<_, (i64,), _>(&sql, values)
        .fetch_one(executor.as_executor())
        .await
        .context(SqlxSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::Count,
        })?;

    // this should practically never fail, but fine.
    Ok(usize::try_from(num)
        .context(UsizeSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::Count,
        })?)
}

/// Insert a new row into the model manager's table using the specified executor.
pub async fn create<MC, X, I, E>(executor: &mut X, data: I) -> Result<E>
where
    MC: DbBmc,
    X: AsExecutor,
    I: Insert,
    E: Select,
{
    let mut query = Query::insert();
    query
        .into_table(MC::get_table_ref())
        .columns(data.insert_cols())
        .values_panic(data.insert_vals())
        .returning(Query::returning().columns(E::select_cols()));

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let res = sqlx::query_as_with::<_, _, _>(&sql, values)
        .fetch_one(executor.as_executor())
        .await
        .context(SqlxSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::Create,
        })?;

    Ok(res)
}

/// Get a row from the model manager's table using the specified id and executor.
pub async fn get<MC, X, E>(executor: &mut X, id: &<MC as DbBmc>::IdType) -> Result<E>
where
    MC: DbBmc,
    X: AsExecutor,
    E: Select,
{
    let mut query = Query::select();

    query
        .from(MC::get_table_ref())
        .columns(E::select_cols())
        .and_where(Expr::col(MC::id_column()).eq(MC::id_to_value(id)));

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let entity = sqlx::query_as_with(&sql, values)
        .fetch_optional(executor.as_executor())
        .await
        .context(SqlxSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::Get,
        })?
        .ok_or_else(|| Error::EntityNotFound {
            entity: MC::ENTITY,
            id: id.to_string(),
        })?;

    Ok(entity)
}

/// List all rows from the model manager's table using the specified executor.
///
/// If you need pagination/filtering, use [list_paginated].
pub async fn list<MC, X, E>(executor: &mut X) -> Result<Vec<E>>
where
    MC: DbBmc,
    X: AsExecutor,
    E: Select,
{
    let mut query = Query::select();

    query.from(MC::get_table_ref()).columns(E::select_cols());

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let entities: Vec<E> = sqlx::query_as_with(&sql, values)
        .fetch_all(executor.as_executor())
        .await
        .context(SqlxSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::List,
        })?;

    Ok(entities)
}

/// Get a page of rows from the model manager's table using the specified executor and filters.
///
/// If you want to just list all rows, use [list]
pub async fn list_paginated<MC, X, F, E>(executor: &mut X, filter: &F) -> Result<Paginated<E>>
where
    MC: DbBmc,
    X: AsExecutor,
    F: Filter + CursoredFilter,
    E: Select + Cursored,
{
    let mut query = Query::select();

    query
        .from(MC::get_table_ref())
        .columns(E::select_cols())
        .order_by(E::cursor_column(), F::cursor_column_order())
        .limit(filter.page_limit() as u64);

    filter.filter_query(&mut query);

    if let Some(cursor) = filter.cursor() {
        if F::cursor_column_order() == sea_query::Order::Asc {
            query.and_where(Expr::col(E::cursor_column()).gt(cursor));
        } else {
            query.and_where(Expr::col(E::cursor_column()).lt(cursor));
        }
    }

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let entities: Vec<E> = sqlx::query_as_with(&sql, values)
        .fetch_all(executor.as_executor())
        .await
        .context(SqlxSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::ListPaginated,
        })?;

    Ok(Paginated::new(entities, filter.page_limit()))
}

/// Update a row in the model manager's table using the specified executor, id, and data.
pub async fn update<MC, X, U, E>(executor: &mut X, id: &<MC as DbBmc>::IdType, data: U) -> Result<E>
where
    MC: DbBmc,
    X: AsExecutor,
    U: Update,
    E: Select,
{
    let values = data.update_values();
    if values.is_empty() {
        return Err(Error::EmptyUpdate {
            entity: MC::ENTITY,
            id: id.to_string(),
        });
    }

    let mut query = Query::update();

    query
        .table(MC::get_table_ref())
        .values(values)
        .and_where(Expr::col(MC::id_column()).eq(MC::id_to_value(id)))
        .returning(Query::returning().columns(E::select_cols()));

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let entity = sqlx::query_as_with(&sql, values)
        .fetch_optional(executor.as_executor())
        .await
        .context(SqlxSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::Update,
        })?
        .ok_or_else(|| Error::EntityNotFound {
            entity: MC::ENTITY,
            id: id.to_string(),
        })?;

    Ok(entity)
}

/// Delete a row in the model manager's table, using the specified executor and id.
pub async fn delete<MC, X>(executor: &mut X, id: &<MC as DbBmc>::IdType) -> Result<()>
where
    MC: DbBmc,
    X: AsExecutor,
{
    let mut query = Query::delete();

    query
        .from_table(MC::get_table_ref())
        .and_where(Expr::col(MC::id_column()).eq(MC::id_to_value(id)));

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let count = sqlx::query_with(&sql, values)
        .execute(executor.as_executor())
        .await
        .context(SqlxSnafu)
        .context(OperationSnafu {
            entity: MC::ENTITY,
            operation: DbBmcOp::Delete,
        })?
        .rows_affected();

    if count == 0 {
        return Err(Error::EntityNotFound {
            entity: MC::ENTITY,
            id: id.to_string(),
        });
    }

    Ok(())
}
