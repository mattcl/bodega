use std::fmt::Debug;

/// Indicated that the given type can produce a cursor for use in pagination.
pub trait Cursored {
    /// The type of the cursor.
    type CursorType: Debug + Clone + Into<sea_query::SimpleExpr>;

    /// Get the value of the cursor for this instance.
    fn cursor_value(&self) -> Self::CursorType;

    /// Get a reference to the column corresponding to the cursor (i.e. `id`).
    fn cursor_column() -> sea_query::DynIden;
}

/// Indicates the given type can be used for filtering paginated entries for a
/// model manager.
pub trait CursoredFilter {
    /// The type of the cursor.
    type Entity: Cursored;

    /// Get the cursor, if present.
    fn cursor(&self) -> Option<<Self::Entity as Cursored>::CursorType>;

    /// Sets the cursor to the specified value.
    ///
    /// We should prefer to use the builder for the filter to set this, but,
    /// internally, say for walking pages, this is a convenience. For this
    /// reason, this does not support _clearing_ the cursor (setting it to
    /// `None`).
    fn set_cursor(&mut self, cursor: <Self::Entity as Cursored>::CursorType);

    /// Get the page limit.
    fn page_limit(&self) -> usize;

    /// The ordering ASC or DESC.
    ///
    /// This is tied to the cursor because we need to know if we're looking
    /// above or below the given cursor.
    fn cursor_column_order() -> sea_query::Order {
        sea_query::Order::Asc
    }
}

/// A wrapper around the entities returned from the database that also includes
/// information required for requesting the next page of entries.
#[derive(Debug, Clone)]
pub struct Paginated<T: Cursored> {
    pub entries: Vec<T>,
    pub next_cursor: Option<<T as Cursored>::CursorType>,
    pub limit: usize,
}

impl<T: Cursored> Default for Paginated<T> {
    fn default() -> Self {
        Self {
            entries: Vec::default(),
            next_cursor: None,
            limit: 10,
        }
    }
}

impl<T> Paginated<T>
where
    T: Cursored,
{
    pub fn new(entries: Vec<T>, limit: usize) -> Self {
        // this has the effect that we will indicate that an extra page of
        // entries might exist if the number of entries is an exact multiple of
        // the limit.
        let next_cursor = if entries.len() >= limit {
            entries.last().map(|e| e.cursor_value())
        } else {
            None
        };

        Self {
            entries,
            next_cursor,
            limit,
        }
    }

    pub fn has_next(&self) -> bool {
        self.next_cursor.is_some()
    }
}

#[cfg(test)]
mod tests {
    use sea_query::{enum_def, IntoIden};

    use super::*;

    #[enum_def]
    struct Dummy {
        id: i64,
    }

    impl Cursored for Dummy {
        type CursorType = i64;

        fn cursor_value(&self) -> Self::CursorType {
            self.id
        }

        fn cursor_column() -> sea_query::DynIden {
            DummyIden::Id.into_iden()
        }
    }

    fn entries() -> Vec<Dummy> {
        vec![
            Dummy { id: 1 },
            Dummy { id: 2 },
            Dummy { id: 3 },
            Dummy { id: 4 },
            Dummy { id: 5 },
            Dummy { id: 6 },
            Dummy { id: 7 },
            Dummy { id: 8 },
            Dummy { id: 9 },
            Dummy { id: 10 },
        ]
    }

    #[test]
    fn cursor_set_when_enough_entries() {
        let p = Paginated::new(entries(), 10);
        assert_eq!(p.next_cursor, Some(10));

        let p = Paginated::new(entries().into_iter().take(2).collect(), 2);
        assert_eq!(p.next_cursor, Some(2));
    }

    #[test]
    fn cursor_none_when_not_enough_entries() {
        let entries = entries();
        let num = entries.len() + 1;
        let p = Paginated::new(entries, num);
        assert_eq!(p.next_cursor, None);
    }
}
