// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-
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
pub struct RowStream<Db, Row>
where
    Db: sqlx::Database,
{
    pool: Box<Pool<Db>>,
    #[borrows(pool)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<Db, Row> RowStream<Db, Row>
where
    Db: sqlx::Database,
{
    #[allow(dead_code)]
    pub fn make(
        pool: &Pool<Db>,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Pool<Db>> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    ) -> Self {
        RowStreamBuilder {
            pool: Box::new(pool.clone()),
            inner_builder,
        }
        .build()
    }
}
impl<Db, Row> Stream for RowStream<Db, Row>
where
    Db: sqlx::Database,
{
    type Item = Result<Row, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}

#[ouroboros::self_referencing]
pub struct RowStreamDyn<Db, Row>
where
    Db: sqlx::Database,
{
    pool: Box<Pool<Db>>,
    sql: Box<String>,
    #[borrows(pool, sql)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<Db, Row> RowStreamDyn<Db, Row>
where
    Db: sqlx::Database,
{
    #[allow(dead_code)]
    pub fn make(
        pool: &Pool<Db>,
        sql: impl ToString,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Pool<Db>> as ::core::ops::Deref>::Target,
            &'this <Box<String> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    ) -> Self {
        RowStreamDynBuilder {
            pool: Box::new(pool.clone()),
            sql: Box::new(sql.to_string()),
            inner_builder,
        }
        .build()
    }
}
impl<Db, Row> Stream for RowStreamDyn<Db, Row>
where
    Db: sqlx::Database,
{
    type Item = Result<Row, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
