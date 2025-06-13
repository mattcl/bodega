/// A wrapper around option for use in produing expressions that can be Null.
///
/// We need to work around a pretty big limitation in sea-query that prevents
/// us from easily casting an option type to null because null is not a distinct
/// type. We also can't just `impl From<Option<OurTYpe>> for sea_query::SimpleExpr`
/// because of the orphan rule.
///
/// To solve this, we're going to use a wrapper type that we _can_ impl the
/// correct value on.
pub struct CustomOption<T>(pub Option<T>);

impl<T> From<Option<T>> for CustomOption<T>
where
    T: Into<sea_query::SimpleExpr>,
{
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

impl<T> CustomOption<T>
where
    T: Into<sea_query::SimpleExpr>,
{
    pub fn into_expr(self) -> sea_query::SimpleExpr {
        self.0
            .map(|v| v.into())
            .unwrap_or_else(|| sea_query::SimpleExpr::Custom("NULL".into()))
    }
}

impl<T> From<CustomOption<T>> for sea_query::SimpleExpr
where
    T: Into<sea_query::SimpleExpr>,
{
    fn from(value: CustomOption<T>) -> Self {
        value.into_expr()
    }
}
