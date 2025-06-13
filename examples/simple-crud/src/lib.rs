use bodega::{
    store_enum, uuid_id, Cursored, CursoredFilter, DbBmc, Filter, Insert, JsonValue, Select, Update,
};
use chrono::{DateTime, Utc};
use derive_builder::Builder;
use sea_query::Expr;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[uuid_id]
pub struct BookId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, AsRefStr)]
#[store_enum]
pub enum Genre {
    Mystery,
    ScienceFiction,
    Fantasy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonValue)]
pub struct Meta {
    spine_size: u32,
    book_weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Select)]
#[sea_query::enum_def]
pub struct Book {
    #[select(cursor)]
    id: BookId,
    title: String,
    author: String,
    genre: Vec<Genre>,
    pages: i64,
    #[sqlx(json)]
    meta: Meta,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insert)]
#[insert(iden_enum = BookIden)]
pub struct BookCreate {
    title: String,
    author: String,
    #[insert(cust_opt)]
    genre: Option<Genre>,
    meta: Meta,
    pages: i64,
}

#[derive(Debug, Clone, Update)]
#[update(iden_enum = BookIden)]
pub struct BookUpdate {
    title: Option<String>,
    author: Option<String>,
    genre: Option<Genre>,
    meta: Option<Meta>,
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
    #[builder(setter(strip_option), default)]
    genre: Option<Genre>,
}

impl CursoredFilter for BookFilters {
    type Entity = Book;

    fn cursor(&self) -> Option<<Self::Entity as Cursored>::CursorType> {
        self.cursor
    }

    fn set_cursor(&mut self, cursor: <Self::Entity as Cursored>::CursorType) {
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

        if let Some(genre) = self.genre {
            query.and_where(Expr::col(BookIden::Genre).eq(genre));
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
        list_paginated = BookFilters,
        update = BookUpdate,
        delete,
        count,
    )
)]
pub struct BookBmc;
