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
    pub fn make(
        pool: &Pool<DB>,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Pool<DB>> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    ) -> Self {
        RowStreamBuilder {
            pool: Box::new(pool.clone()),
            inner_builder,
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
pub struct RowStreamWithParams<DB, Row, Params>
where
    DB: sqlx::Database,
    Params: 'static,
{
    pool: Box<Pool<DB>>,
    params: Box<Params>,
    #[borrows(pool, params)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<DB, Row, Params> RowStreamWithParams<DB, Row, Params>
where
    DB: sqlx::Database,
    Params: 'static,
{
    #[allow(dead_code)]
    pub fn make(
        pool: Pool<DB>,
        params: Params,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Pool<DB>> as ::core::ops::Deref>::Target,
            &'this <Box<Params> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    ) -> Self {
        RowStreamWithParamsBuilder {
            pool: Box::new(pool),
            params: Box::new(params),
            inner_builder,
        }
        .build()
    }
}
impl<DB, Row, Params> Stream for RowStreamWithParams<DB, Row, Params>
where
    DB: sqlx::Database,
    Params: 'static,
{
    type Item = Result<Row, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}

#[ouroboros::self_referencing]
pub struct RowStreamDyn<DB, Row>
where
    DB: sqlx::Database,
{
    pool: Box<Pool<DB>>,
    sql: Box<String>,
    #[borrows(pool, sql)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<DB, Row> RowStreamDyn<DB, Row>
where
    DB: sqlx::Database,
{
    #[allow(dead_code)]
    pub fn make(
        pool: &Pool<DB>,
        sql: impl ToString,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Pool<DB>> as ::core::ops::Deref>::Target,
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
impl<DB, Row> Stream for RowStreamDyn<DB, Row>
where
    DB: sqlx::Database,
{
    type Item = Result<Row, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
