use actix_web::*;
use futures::{
    prelude::*,
    stream::BoxStream,
    task::{Context, Poll},
};
use sqlx::Pool;
pub use std::io::Write;
use std::pin::Pin;

#[ouroboros::self_referencing]
pub struct RowStream<DB, Row>
where
    DB: sqlx::Database,
{
    pool: Box<Pool<DB>>,
    #[borrows(pool)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<DB, Row> RowStream<DB, Row>
where
    DB: sqlx::Database,
{
    #[allow(dead_code)]
    pub fn make<F>(pool: &Pool<DB>, f: F) -> Self
    where
        F: for<'this> FnOnce(
            &'this <Box<Pool<DB>> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    {
        RowStreamBuilder {
            pool: Box::new(pool.clone()),
            inner_builder: f,
        }
        .build()
    }
}
impl<DB, Row> Stream for RowStream<DB, Row>
where
    DB: sqlx::Database,
{
    type Item = Result<Row, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}

#[ouroboros::self_referencing]
pub struct SqlRowStream<DB, Row>
where
    DB: sqlx::Database,
{
    pool: Box<Pool<DB>>,
    sql: Box<String>,
    #[borrows(pool, sql)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<DB, Row> SqlRowStream<DB, Row>
where
    DB: sqlx::Database,
{
    #[allow(dead_code)]
    pub fn make<S, F>(pool: &Pool<DB>, sql: S, f: F) -> Self
    where
        S: ToString,
        F: for<'this> FnOnce(
            &'this <Box<Pool<DB>> as ::core::ops::Deref>::Target,
            &'this <Box<String> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    {
        SqlRowStreamBuilder {
            pool: Box::new(pool.clone()),
            sql: Box::new(sql.to_string()),
            inner_builder: f,
        }
        .build()
    }
}
impl<DB, Row> Stream for SqlRowStream<DB, Row>
where
    DB: sqlx::Database,
{
    type Item = Result<Row, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
