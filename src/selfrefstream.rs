// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-
use futures::{
    prelude::*,
    stream::BoxStream,
    task::{Context, Poll},
};
pub use std::io::Write;
use std::pin::Pin;

#[ouroboros::self_referencing]
pub struct SelfRefStream<Bindings, Item>
where
    Bindings: 'static,
{
    bindings: Box<Bindings>,
    #[borrows(mut bindings)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Item, sqlx::Error>>,
}
impl<Bindings, Item> SelfRefStream<Bindings, Item>
where
    Bindings: 'static,
{
    #[inline]
    pub fn build(
        bindings: Bindings,
        inner_builder: impl for<'this> FnOnce(
            &'this mut <Box<Bindings> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Item, sqlx::Error>>,
    ) -> Self {
        Self::new(Box::new(bindings), inner_builder)
    }
}
impl<Bindings, Item> Stream for SelfRefStream<Bindings, Item>
where
    Bindings: 'static,
{
    type Item = Result<Item, sqlx::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
