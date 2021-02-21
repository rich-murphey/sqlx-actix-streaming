// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-
use actix_web::*;
use futures::{
    prelude::*,
    stream::BoxStream,
    task::{Context, Poll},
};
pub use std::io::Write;
use std::pin::Pin;

#[ouroboros::self_referencing]
pub struct SqlxStream<Bindings, Item>
where
    Bindings: 'static,
{
    params: Box<Bindings>,
    #[borrows(params)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Item, sqlx::Error>>,
}
impl<Bindings, Item> SqlxStream<Bindings, Item>
{
    #[allow(dead_code)]
    pub fn make(
        params: Bindings,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Bindings> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Item, sqlx::Error>>,
    ) -> Self {
        SqlxStreamBuilder {
            params: Box::new(params),
            inner_builder,
        }
        .build()
    }
}
impl<Bindings, Item> Stream for SqlxStream<Bindings, Item>
{
    type Item = Result<Item, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
