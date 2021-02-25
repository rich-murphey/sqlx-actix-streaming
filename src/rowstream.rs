// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-
use actix_web::*;
use futures::{
    prelude::*,
    stream::BoxStream,
    task::{Context, Poll},
};
pub use std::io::Write;
use std::pin::Pin;
use core::ops::Deref;
use stable_deref_trait::StableDeref;

#[ouroboros::self_referencing]
pub struct RowStream<Bindings, Row>
where
    Bindings: Unpin + StableDeref + 'static,
{
    bindings: Bindings,
    #[borrows(bindings)]
    #[covariant] // StableDeref is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<Bindings, Row> RowStream<Bindings, Row>
where
    Bindings: Unpin + StableDeref + 'static,
{
    #[inline]
    pub fn build(
        bindings: Bindings,
        inner_builder: impl for<'this> FnOnce(
            &'this <Bindings as Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    ) -> Self {
        Self::new(bindings, inner_builder)
    }
}
impl<Bindings, Row> Stream for RowStream<Bindings, Row>
where
    Bindings: Unpin + StableDeref + 'static,
{
    type Item = Result<Row, sqlx::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
