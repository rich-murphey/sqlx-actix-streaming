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
pub struct RowStream<Bindings, Row>
where
    Bindings: 'static,
{
    bindings: Box<Bindings>,
    #[borrows(bindings)]
    #[covariant] // Box is covariant.
    inner: BoxStream<'this, Result<Row, sqlx::Error>>,
}
impl<Bindings, Row> RowStream<Bindings, Row> {
    #[allow(dead_code)]
    pub fn make(
        bindings: Bindings,
        inner_builder: impl for<'this> FnOnce(
            &'this <Box<Bindings> as ::core::ops::Deref>::Target,
        ) -> BoxStream<'this, Result<Row, sqlx::Error>>,
    ) -> Self {
        RowStreamBuilder {
            bindings: Box::new(bindings),
            inner_builder,
        }
        .build()
    }
}
impl<Bindings, Row> Stream for RowStream<Bindings, Row> {
    type Item = Result<Row, sqlx::Error>;
    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_inner_mut(|s| s.as_mut().poll_next(cx))
    }
}
