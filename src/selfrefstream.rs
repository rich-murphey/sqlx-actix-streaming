use futures::{
    prelude::*,
    stream::BoxStream,
    task::{Context, Poll},
};
pub use std::io::Write;
use std::pin::Pin;

#[ouroboros::self_referencing]
pub struct SelfRefStream<Args, Item, Error>
where
    Args: 'static,
{
    args: Args,
    #[borrows(args)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Item, Error>>,
}

impl<Args, Item, Error> SelfRefStream<Args, Item, Error>
where
    Args: 'static,
{
    #[inline]
    pub fn build(
        args: Args,
        inner_builder: impl for<'this> FnOnce(&'this Args) -> BoxStream<'this, Result<Item, Error>>,
    ) -> Self {
        SelfRefStreamBuilder {
            args,
            inner_builder,
        }
        .build()
    }
}

impl<Args, Item, Error> Stream for SelfRefStream<Args, Item, Error>
where
    Args: 'static,
{
    type Item = Result<Item, Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
