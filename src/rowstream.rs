// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-
use futures::{
    prelude::*,
    stream::BoxStream,
    task::{Context, Poll},
};
use sqlx::{database::Database, pool::PoolConnection, Acquire};
pub use std::io::Write;
use std::pin::Pin;

#[ouroboros::self_referencing]
pub struct RowStream<DB, Args, Item>
where
    DB: Database,
    Args: 'static,
{
    conn: Box<PoolConnection<DB>>,
    args: Box<Args>,
    #[borrows(mut conn, args)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Item, sqlx::Error>>,
}
impl<DB, Args, Item> RowStream<DB, Args, Item>
where
    DB: Database,
    Args: 'static,
{
    #[inline]
    pub async fn build<'c>(
        pool: impl Acquire<'c, Database = DB, Connection = PoolConnection<DB>>,
        args: Args,
        inner_builder: impl for<'this> FnOnce(
            &'this mut PoolConnection<DB>,
            &'this Args,
        ) -> BoxStream<'this, Result<Item, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error> {
        let conn = pool.acquire().await?;
        Ok(Self::new(Box::new(conn), Box::new(args), inner_builder))
    }
}
impl<DB, Args, Item> Stream for RowStream<DB, Args, Item>
where
    DB: Database,
    Args: 'static,
{
    type Item = Result<Item, sqlx::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
