// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-
use actix_web::{
    error::ErrorInternalServerError,
    web::{Bytes, BytesMut},
};
use futures::{
    task::{Context, Poll},
    Stream, StreamExt,
};
pub use std::io::Write;
use std::pin::Pin;

#[derive(Debug)]
pub enum ByteStreamState {
    /// self.poll_next() has never been called.
    Unused,
    /// inner.poll_next() has never returned an item.
    Empty,
    /// inner.poll_next() has returned an item.
    NonEmpty,
    /// inner.poll_next() has returned Ready(None).
    Done,
}
#[cfg(feature = "logging")]
use log::*;

pub struct ByteStream<InnerVal, InnerStream, Serializer>
where
    InnerStream: Stream<Item = Result<InnerVal, sqlx::Error>>,
    Serializer: FnMut(&mut BytesWriter, &InnerVal) -> Result<(), actix_web::Error>,
{
    inner: Pin<Box<InnerStream>>,
    serializer: Box<Serializer>,
    state: ByteStreamState,
    item_size: usize,
    prefix: Vec<u8>,
    separator: Vec<u8>,
    suffix: Vec<u8>,
    buf: BytesWriter,
    #[cfg(feature = "logging")]
    item_count: usize,
}

impl<InnerVal, InnerStream, Serializer> ByteStream<InnerVal, InnerStream, Serializer>
where
    InnerStream: Stream<Item = Result<InnerVal, sqlx::Error>>,
    Serializer: FnMut(&mut BytesWriter, &InnerVal) -> Result<(), actix_web::Error>,
{
    #![allow(dead_code)]
    const DEFAULT_ITEM_SIZE: usize = 2048;

    pub fn new(stream: InnerStream, serializer: Serializer) -> Self {
        Self {
            inner: Box::pin(stream),
            serializer: Box::new(serializer),
            state: ByteStreamState::Unused,
            item_size: Self::DEFAULT_ITEM_SIZE,
            prefix: vec![b'['],
            separator: vec![b','],
            suffix: vec![b']'],
            buf: BytesWriter(BytesMut::with_capacity(Self::DEFAULT_ITEM_SIZE)),
            #[cfg(feature = "logging")]
            item_count: 0,
        }
    }
    /// Set the serializer. The serializer writes each item to the buffer as json text.
    pub fn serializer(mut self, s: Serializer) -> Self {
        self.serializer = Box::new(s);
        self
    }
    /// Set the prefix for the json array. '[' by default.
    pub fn prefix<S: ToString>(mut self, s: S) -> Self {
        self.prefix = s.to_string().into_bytes();
        self
    }
    /// Set the separator for the json array elements. ',' by default.
    pub fn separator<S: ToString>(mut self, s: S) -> Self {
        self.separator = s.to_string().into_bytes();
        self
    }
    /// Set the suffix for the json array. ']' by default.
    pub fn suffix<S: ToString>(mut self, s: S) -> Self {
        self.suffix = s.to_string().into_bytes();
        self
    }
    /// Set the expected size of the json text of a single item.
    pub fn size(mut self, size: usize) -> Self {
        self.item_size = size;
        self
    }
    // append the configured prefix to the output buffer.
    #[inline]
    fn put_prefix(&mut self) {
        self.buf.0.extend_from_slice(&self.prefix);
    }
    // append the configured separator to the output buffer.
    #[inline]
    fn put_separator(&mut self) {
        self.buf.0.extend_from_slice(&self.separator);
    }
    // append the configured suffix to the output buffer.
    #[inline]
    fn put_suffix(&mut self) {
        self.buf.0.extend_from_slice(&self.suffix);
    }
    // return the buffered output bytes.
    #[inline]
    fn get_bytes(&mut self) -> Bytes {
        self.buf.0.split().freeze()
    }
    // ensure capacity to write one additional item into the buffer.
    #[inline]
    fn reserve_one_item(&mut self) {
        self.buf.0.reserve(self.item_size);
    }
    // use the given closure to write a record to the buffer.
    #[inline]
    fn write_record(&mut self, record: &InnerVal) -> Result<(), actix_web::Error> {
        (self.serializer)(&mut self.buf, record)
    }
}

impl<InnerVal, InnerStream, Serializer> Stream for ByteStream<InnerVal, InnerStream, Serializer>
where
    InnerStream: Stream<Item = Result<InnerVal, sqlx::Error>>,
    Serializer: FnMut(&mut BytesWriter, &InnerVal) -> Result<(), actix_web::Error>,
{
    type Item = Result<Bytes, actix_web::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use ByteStreamState::*;
        use Poll::*;
        if let Done = self.state {
            return Ready(None);
        }
        self.reserve_one_item();
        if let Unused = self.state {
            self.state = Empty;
            self.put_prefix();
        }
        loop {
            match self.inner.poll_next_unpin(cx) {
                Ready(Some(Ok(record))) => {
                    #[cfg(feature = "logging")]
                    {
                        self.item_count += 1;
                    }
                    match self.state {
                        Empty => self.state = NonEmpty,
                        NonEmpty => self.put_separator(),
                        _ => (),
                    };
                    let initial_len = self.buf.0.len();
                    if let Err(e) = self.write_record(&record) {
                        #[cfg(feature = "logging")]
                        error!("write_record: {:?}", e);
                        break Ready(Some(Err(ErrorInternalServerError(e))));
                    }
                    let item_size = self.buf.0.len() - initial_len;
                    if self.item_size < item_size {
                        self.item_size = item_size.next_power_of_two();
                    }
                    let remaining_space = self.buf.0.capacity() - self.buf.0.len();
                    if item_size <= remaining_space {
                        continue;
                    }
                    break Ready(Some(Ok(self.get_bytes())));
                }
                Ready(Some(Err(e))) => {
                    #[cfg(feature = "logging")]
                    error!("poll_next: {:?}", e);
                    break Ready(Some(Err(ErrorInternalServerError(e))));
                }
                Ready(None) => {
                    self.state = Done;
                    self.put_suffix();
                    break Ready(Some(Ok(self.get_bytes())));
                }
                Pending => {
                    if self.buf.0.is_empty() {
                        break Pending;
                    }
                    break Ready(Some(Ok(self.get_bytes())));
                }
            }
        }
    }
}

#[cfg(feature = "logging")]
impl<InnerVal, InnerStream, Serializer> Drop for ByteStream<InnerVal, InnerStream, Serializer>
where
    InnerStream: Stream<Item = Result<InnerVal, sqlx::Error>>,
    Serializer: FnMut(&mut BytesWriter, &InnerVal) -> Result<(), actix_web::Error>,
{
    fn drop(&mut self) {
        if !matches!(self.state, ByteStreamState::Done) {
            warn!(
                "dropped ByteStream in state: {:?} after {} items",
                self.state, self.item_count
            );
        }
    }
}

pub struct BytesWriter(pub BytesMut);

impl Write for BytesWriter {
    #[inline]
    fn write(&mut self, src: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(src);
        Ok(src.len())
    }
    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
