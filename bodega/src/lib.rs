#![doc = include_str!("../../README.md")]
mod base;
mod error;
mod model_manger;
mod pagination;

pub use base::{count, create, delete, get, list, list_paginated, update};
pub use base::{DbBmc, Filter, IdType, Insert, Select, Update};
pub use error::{Error, Result};
pub use model_manger::{new_db_pool, AsExecutor, DbModelManager, Transaction};
pub use pagination::{Cursored, CursoredFilter, Paginated};

// macros

/// Derives an implementation for the [`Select`] trait on a struct with named
/// fields, allowing it to be constructed from a response from a query against
/// the store.
///
/// Configuration for `#[select(...)]` field attr
///
/// * `cursor` *Optional - once* Indicate that the annotated field is to be used
///   for pagination at the store layer. This will cause [`Cursored`] to be
///   implemented for the struct.
///
/// # Examples
/// ```
/// use bodega::{Select, uuid_id};
/// use chrono::{DateTime, Utc};
/// use serde::{Deserialize, Serialize};
/// use uuid::Uuid;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// #[uuid_id]
/// pub struct BookId(Uuid);
///
/// #[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Select)]
/// #[sea_query::enum_def]
/// pub struct Book {
///     #[select(cursor)]
///     id: BookId,
///     title: String,
///     author: String,
///     pages: i64,
///     created_at: DateTime<Utc>,
///     updated_at: DateTime<Utc>,
/// }
/// ```
pub use bodega_macros::Select;

/// Derives an implementation for [`Insert`] on a struct with named fields,
/// allowing that struct to be used to be used to create an entry in the store.
///
/// Configuration for `#[insert(...)]` container attr
///
/// * `iden_enum` *Required.* The enum of `Iden` variants for the corresponding
///   model.
///
/// Configuration for `#[insert(...)]` field attr
///
/// * `iden` *Optional.* Override the computed `Iden` variant for this field.
///
/// # Examples
/// ```
/// use bodega::{Select, Insert, uuid_id};
/// use chrono::{DateTime, Utc};
/// use serde::{Deserialize, Serialize};
/// use uuid::Uuid;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// #[uuid_id]
/// pub struct BookId(Uuid);
///
/// #[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Select)]
/// #[sea_query::enum_def]
/// pub struct Book {
///     id: BookId,
///     title: String,
///     author: String,
///     pages: i64,
///     created_at: DateTime<Utc>,
///     updated_at: DateTime<Utc>,
/// }
///
/// #[derive(Debug, Clone, Insert)]
/// #[insert(iden_enum = BookIden)]
/// pub struct BookCreate {
///     title: String,
///     // unnecessary override for example
///     #[insert(iden = BookIden::Author)]
///     author: String,
///     pages: i64,
/// }
/// ```
pub use bodega_macros::Insert;

/// Derives an implementation for [`Update`] on a struct with named fields,
/// allowing that struct to be used to be used to update an entry in the store.
///
/// Configuration for `#[update(...)]` container attr
///
/// * `iden_enum` *Required.* The enum of `Iden` variants for the corresponding
///   model.
///
/// Configuration for `#[update(...)]` field attr
///
/// * `iden` *Optional.* Override the computed `Iden` variant for this field.
///
/// # Examples
/// ```
/// use bodega::{Select, Update, uuid_id};
/// use chrono::{DateTime, Utc};
/// use serde::{Deserialize, Serialize};
/// use uuid::Uuid;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// #[uuid_id]
/// pub struct BookId(Uuid);
///
/// #[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Select)]
/// #[sea_query::enum_def]
/// pub struct Book {
///     id: BookId,
///     title: String,
///     author: String,
///     pages: i64,
///     created_at: DateTime<Utc>,
///     updated_at: DateTime<Utc>,
/// }
///
/// #[derive(Debug, Clone, Update)]
/// #[update(iden_enum = BookIden)]
/// pub struct BookUpdate {
///     title: Option<String>,
///     author: Option<String>,
///     pages: Option<i64>,
///     updated_at: DateTime<Utc>,
/// }
/// ```
pub use bodega_macros::Update;

/// Implement [`DbBmc`] on a type, and optionally add basic CRUD implementations.
///
/// While you're free to implement additional methods and custom CRUD actions,
/// this macro can generate reasonable implementations for most simple models.
///
/// Configuration for `#[db_bmc(...)]` container attr
///
/// * `model = ...` *Required.* The type of model returned from things like SELECT
///   statements.
/// * `id_type = ...` *Required.* The [`IdType`] of the model.
/// * `model_name = "..."` *Optional.* A specific model name like `"foo"`, for use when
///   deriving the model name from the `model` is not sufficient.
/// * `table_name = "..."` *Optional.* A specific table name like `"foos"`, for use when
///   deriving the table name from the model name is not sufficient.
/// * `id_iden = ...` *Optional.* Override the derived `Iden` enum for this `model`.
/// * `error = ...` *Optional.* Return this error type instead of [`crate::Error`].
///   The type passed must implement `From<bodega::Error>`.
/// * `methods(...)` *Optional.* A comma-separated list of methods to implement
///   from the following:
///   * `create = ...`, `get`, `list`, `list_paginated = ...`, `update = ...`, `delete`, `count`.
///
/// Specific configuration for `#[db_bmc(methods(...))]
///
/// * `create = ...` Generate a `create` method on the controller accepting an
///   instance of the specified type that implements [`Insert`]. Returns the
///   created instance as an instance of `model`.
/// * `get` Generate a `get` method on the controller accepting an id. Returns
///   the corresponding instance of the `model` on success.
/// * `list` Generate a `list` method on the controller. Returns a [`Vec<T>`]
///   of the specified `model` containing every row from the store.
/// * `list_paginated = ...` Generate a `list_paginated` method on the controller
///   using the specified type as the [`Filter`]/[`CursoredFilter`]. Returns
///   a single page [`Paginated<T>`] of the given `model` that satisfies the
///   filters.
/// * `update = ...` Generate an `update` method on the controller accepting an
///   id and the specified type. Returns the updated instance as an instance of
///   `model`.
/// * `delete` Generate a `delete` method on the controller accepting an id.
/// * `count` Generate a `count` method on the controller returning the count of
///   all rows of this controller's model in the store.
///
/// # Examples
/// ```
/// use bodega::{
///     Select, Insert, Update, Filter, CursoredFilter, DbBmc, IdType, uuid_id
/// };
/// use chrono::{DateTime, Utc};
/// use derive_builder::Builder;
/// use sea_query::Expr;
/// use serde::{Deserialize, Serialize};
/// use uuid::Uuid;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// #[uuid_id]
/// pub struct BookId(Uuid);
///
/// #[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Select)]
/// #[sea_query::enum_def]
/// pub struct Book {
///     #[select(cursor)]
///     id: BookId,
///     title: String,
///     author: String,
///     pages: i64,
///     created_at: DateTime<Utc>,
///     updated_at: DateTime<Utc>,
/// }
///
/// #[derive(Debug, Clone, Insert)]
/// #[insert(iden_enum = BookIden)]
/// pub struct BookCreate {
///     title: String,
///     author: String,
///     pages: i64,
/// }
///
/// #[derive(Debug, Clone, Update)]
/// #[update(iden_enum = BookIden)]
/// pub struct BookUpdate {
///     title: Option<String>,
///     author: Option<String>,
///     pages: Option<i64>,
///     updated_at: DateTime<Utc>,
/// }
///
/// #[derive(Debug, Clone, Builder)]
/// pub struct BookFilters {
///     #[builder(default = 20)]
///     limit: usize,
///     #[builder(setter(strip_option), default)]
///     cursor: Option<BookId>,
///     #[builder(setter(strip_option), default)]
///     author: Option<String>,
/// }
///
/// impl CursoredFilter for BookFilters {
///     type Entity = Book;
///
///     fn cursor(&self) -> Option<<Self::Entity as bodega::Cursored>::CursorType> {
///         self.cursor
///     }
///
///     fn set_cursor(&mut self, cursor: <Self::Entity as bodega::Cursored>::CursorType) {
///         self.cursor = Some(cursor);
///     }
///
///     fn page_limit(&self) -> usize {
///         self.limit
///     }
/// }
///
/// impl Filter for BookFilters {
///     fn filter_query(&self, query: &mut sea_query::SelectStatement) {
///         if let Some(ref author) = self.author {
///             query.and_where(Expr::col(BookIden::Author).eq(author));
///         }
///     }
/// }
///
/// #[derive(Debug, Clone, DbBmc)]
/// #[db_bmc(
///     model = Book,
///     id_type = BookId,
///     methods(
///         create = BookCreate,
///         get,
///         list,
///         update = BookUpdate,
///         delete,
///         count,
///     )
/// )]
/// pub struct BookBmc;
///
/// assert_eq!(BookBmc::ENTITY, "book");
/// assert_eq!(BookBmc::TABLE, "books");
/// ```
pub use bodega_macros::DbBmc;

/// Modifies a newtype in the form of `Foo(Uuid)` to have functionality that
/// makes it compatible with a store layer.
///
/// # Examples
/// ```
/// use bodega::uuid_id;
/// use serde::{Deserialize, Serialize};
/// use uuid::Uuid;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// #[uuid_id]
/// pub struct BookId(Uuid);
/// ```
pub use bodega_macros::uuid_id;

/// Modifies an enum corresponding to a postgres enum to support various
/// `sea_query` operations.
///
/// # Examples
/// ```
/// use bodega::store_enum;
/// use serde::{Deserialize, Serialize};
/// use strum::AsRefStr;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, AsRefStr)]
/// #[store_enum]
/// pub enum Genre {
///     Mystery,
///     ScienceFiction,
///     Fantasy,
/// }
/// ```
pub use bodega_macros::store_enum;
