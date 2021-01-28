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
pub struct RowStream<DB, T>
where
    DB: sqlx::Database,
{
    pool: Box<Pool<DB>>,
    #[borrows(pool)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<T, sqlx::Error>>,
}
impl<DB, T> RowStream<DB, T>
where
    DB: sqlx::Database,
{
    #[allow(dead_code)]
    pub fn gen<F>(pool: Pool<DB>, f: F) -> Self
    where
        F: for<'this> FnOnce(
            &'this <Box<Pool<DB>> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<T, sqlx::Error>>,
    {
        RowStreamBuilder {
            pool: Box::new(pool),
            inner_builder: f,
        }
        .build()
    }
}
impl<DB, T> Stream for RowStream<DB, T>
where
    DB: sqlx::Database,
{
    type Item = Result<T, sqlx::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}

#[ouroboros::self_referencing]
pub struct RowWStmtStream<DB, T>
where
    DB: sqlx::Database,
{
    pool: Box<Pool<DB>>,
    sql: Box<String>,
    #[borrows(pool, sql)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<T, sqlx::Error>>,
}
impl<DB, T> RowWStmtStream<DB, T>
where
    DB: sqlx::Database,
{
    #[allow(dead_code)]
    pub fn gen<S, F>(pool: Pool<DB>, sql: S, f: F) -> Self
    where
        S: ToString,
        F: for<'this> FnOnce(
            &'this <Box<Pool<DB>> as ::core::ops::Deref>::Target,
            &'this <Box<String> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<T, sqlx::Error>>,
    {
        RowWStmtStreamBuilder {
            pool: Box::new(pool),
            sql: Box::new(sql.to_string()),
            inner_builder: f,
        }
        .build()
    }
}
impl<DB, T> Stream for RowWStmtStream<DB, T>
where
    DB: sqlx::Database,
{
    type Item = Result<T, sqlx::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
