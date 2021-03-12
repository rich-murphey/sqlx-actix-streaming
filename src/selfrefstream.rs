// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-
use futures::{
    prelude::*,
    stream::BoxStream,
    task::{Context, Poll},
};
pub use std::io::Write;
use std::pin::Pin;

#[ouroboros::self_referencing]
pub struct SelfRefStream<Args, Item>
where
    Args: 'static,
{
    args: Box<Args>,
    #[borrows(args)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Item, sqlx::Error>>,
}
impl<Args, Item> SelfRefStream<Args, Item>
where
    Args: 'static,
{
    #[inline]
    pub fn build(
        args: Args,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Args> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Item, sqlx::Error>>,
    ) -> Self {
        Self::new(Box::new(args), inner_builder)
    }
}
impl<Args, Item> Stream for SelfRefStream<Args, Item>
where
    Args: 'static,
{
    type Item = Result<Item, sqlx::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
