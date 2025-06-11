# bodega

`bodega` is a small, relatively simple object store implementation with lots of
"character." Honestly, though, **don't use this.** I have it set up for the way I
like to write my store layer in rust services, and it probably won't fit your
use case and I have no intention of supporting things beyond my own needs.

It's loosely based on the controller setup in this
[series](https://www.youtube.com/watch?v=Gc5Nj5LJe1U&list=PL7r-PXl6ZPcCTTxjmsb9bFZB9i01fAtI7)
by Jeremy Chone, and seeks to use more of the `sea_query` built-in stuff like
the `Iden` derivation, while also providing generation of CRUD methods on
controllers and various trait derivations.

This will only work with postgres, and is intended to complement a store layer
using `sqlx` and `sea_query`, among other things.

Again, you probably don't want to use this.


## Example

```rust
use bodega::{
    Select, Insert, Update, Filter, CursoredFilter, DbBmc, IdType, uuid_id
};
use chrono::{DateTime, Utc};
use derive_builder::Builder;
use sea_query::Expr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[uuid_id]
pub struct BookId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Select)]
#[sea_query::enum_def]
pub struct Book {
    #[select(cursor)]
    id: BookId,
    title: String,
    author: String,
    pages: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insert)]
#[insert(iden_enum = BookIden)]
pub struct BookCreate {
    title: String,
    author: String,
    pages: i64,
}

#[derive(Debug, Clone, Update)]
#[update(iden_enum = BookIden)]
pub struct BookUpdate {
    title: Option<String>,
    author: Option<String>,
    pages: Option<i64>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Builder)]
pub struct BookFilters {
    #[builder(default = 20)]
    limit: usize,
    #[builder(setter(strip_option), default)]
    cursor: Option<BookId>,
    #[builder(setter(strip_option), default)]
    author: Option<String>,
}

impl CursoredFilter for BookFilters {
    type Entity = Book;

    fn cursor(&self) -> Option<<Self::Entity as bodega::Cursored>::CursorType> {
        self.cursor
    }

    fn set_cursor(&mut self, cursor: <Self::Entity as bodega::Cursored>::CursorType) {
        self.cursor = Some(cursor);
    }

    fn page_limit(&self) -> usize {
        self.limit
    }
}

impl Filter for BookFilters {
    fn filter_query(&self, query: &mut sea_query::SelectStatement) {
        if let Some(ref author) = self.author {
            query.and_where(Expr::col(BookIden::Author).eq(author));
        }
    }
}

#[derive(Debug, Clone, DbBmc)]
#[db_bmc(
    model = Book,
    id_type = BookId,
    methods(
        create = BookCreate,
        get,
        list,
        update = BookUpdate,
        delete,
        count,
    )
)]
pub struct BookBmc;
```
