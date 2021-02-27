// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-]
use actix_web::{
    error::ErrorInternalServerError,
    web::{Bytes, BytesMut},
};
use futures::{
    task::{Context, Poll},
    Stream, TryStream, TryStreamExt,
};
#[cfg(feature = "logging")]
use log::*;
pub use std::io::Write;
use std::pin::Pin;

pub struct BytesWriter(pub BytesMut);
impl BytesWriter {
    #[inline]
    pub fn finish(self) -> BytesMut {
        self.0
    }
    #[inline]
    pub fn freeze(mut self) -> Bytes {
        self.0.split().freeze()
    }
}

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

#[cfg_attr(feature = "logging", derive(Debug))]
pub enum State {
    /// Unused is the initial state of a new instance. Change to Empty
    /// upon self.poll_next().
    Unused,
    /// Empty means self.poll_next() has been called at least once.
    /// Change to NonEmpty when inner_stream.poll_next() returns
    /// Ready(Ok(item).
    Empty,
    /// NonEmpty means inner_stream.poll_next() has returned a
    /// Ready(Ok(item) at least once. Change to Done when
    /// inner_stream.poll_next() returns Ready(None).
    NonEmpty,
    /// Done means inner_stream.poll_next() has returned Ready(None).
    Done,
}

const BYTESTREAM_DEFAULT_ITEM_SIZE: usize = 2048;

pub struct ByteStream<InnerStream, Serializer>
where
    InnerStream: TryStream + Unpin,
    // required for conversion to ErrorInternalServerError(e):
    <InnerStream as TryStream>::Error: std::fmt::Debug + std::fmt::Display + 'static,
    Serializer: FnMut(&mut BytesWriter, &<InnerStream as TryStream>::Ok) -> Result<(), actix_web::Error>
        + Unpin,
{
    inner_stream: InnerStream,
    serializer: Serializer,
    state: State,
    item_size: usize,
    prefix: Vec<u8>,
    delimiter: Vec<u8>,
    suffix: Vec<u8>,
    buf: BytesWriter,
    #[cfg(feature = "logging")]
    item_count: usize,
}

impl<InnerStream, Serializer> ByteStream<InnerStream, Serializer>
where
    InnerStream: TryStream + Unpin,
    <InnerStream as TryStream>::Error: std::fmt::Debug + std::fmt::Display + 'static,
    Serializer: FnMut(&mut BytesWriter, &<InnerStream as TryStream>::Ok) -> Result<(), actix_web::Error>
        + Unpin,
{
    #[inline]
    pub fn new(inner_stream: InnerStream, serializer: Serializer) -> Self {
        Self::with_size(inner_stream, serializer, BYTESTREAM_DEFAULT_ITEM_SIZE)
    }
    pub fn with_size(inner_stream: InnerStream, serializer: Serializer, size: usize) -> Self {
        Self {
            inner_stream,
            serializer,
            state: State::Unused,
            item_size: size,
            prefix: vec![b'['],
            delimiter: vec![b','],
            suffix: vec![b']'],
            buf: BytesWriter(BytesMut::with_capacity(size)),
            #[cfg(feature = "logging")]
            item_count: 0,
        }
    }
    /// Set the prefix for the json array. '[' by default.
    #[inline]
    pub fn prefix<S: ToString>(mut self, s: S) -> Self {
        self.prefix = s.to_string().into_bytes();
        self
    }
    /// Set the delimiter for the json array elements. ',' by default.
    #[inline]
    pub fn delimiter<S: ToString>(mut self, s: S) -> Self {
        self.delimiter = s.to_string().into_bytes();
        self
    }
    /// Set the suffix for the json array. ']' by default.
    #[inline]
    pub fn suffix<S: ToString>(mut self, s: S) -> Self {
        self.suffix = s.to_string().into_bytes();
        self
    }
    // append the configured prefix to the output buffer.
    #[inline]
    fn put_prefix(&mut self) {
        self.buf.0.extend_from_slice(&self.prefix);
    }
    // append the configured delimiter to the output buffer.
    #[inline]
    fn put_delimiter(&mut self) {
        self.buf.0.extend_from_slice(&self.delimiter);
    }
    // append the configured suffix to the output buffer.
    #[inline]
    fn put_suffix(&mut self) {
        self.buf.0.extend_from_slice(&self.suffix);
    }
    // return the buffered output bytes.
    #[inline]
    fn bytes(&mut self) -> Bytes {
        self.buf.0.split().freeze()
    }
    // use the serializer to write one item to the buffer.
    #[inline]
    fn write_item(
        &mut self,
        record: &<InnerStream as TryStream>::Ok,
    ) -> Result<(), actix_web::Error> {
        (self.serializer)(&mut self.buf, record)
    }
}

impl<InnerStream, Serializer> Stream for ByteStream<InnerStream, Serializer>
where
    InnerStream: TryStream + Unpin,
    <InnerStream as TryStream>::Error: std::fmt::Debug + std::fmt::Display + 'static,
    Serializer: FnMut(&mut BytesWriter, &<InnerStream as TryStream>::Ok) -> Result<(), actix_web::Error>
        + Unpin,
{
    type Item = Result<Bytes, actix_web::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;
        use State::*;
        match self.state {
            Unused => {
                self.state = Empty;
                self.put_prefix();
            }
            Done => return Ready(None),
            _ => (),
        }
        loop {
            match self.inner_stream.try_poll_next_unpin(cx) {
                Ready(Some(Ok(record))) => {
                    #[cfg(feature = "logging")]
                    {
                        self.item_count += 1;
                    }
                    match self.state {
                        Empty => self.state = NonEmpty,
                        NonEmpty => self.put_delimiter(),
                        _ => (),
                    };
                    let initial_len = self.buf.0.len();
                    if let Err(e) = self.write_item(&record) {
                        #[cfg(feature = "logging")]
                        error!("write_item: {:?}", e);
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
                    break Ready(Some(Ok(self.bytes())));
                }
                Ready(Some(Err(e))) => {
                    #[cfg(feature = "logging")]
                    error!("poll_next: {:?}", e);
                    break Ready(Some(Err(ErrorInternalServerError(e))));
                }
                Ready(None) => {
                    self.state = Done;
                    self.put_suffix();
                    break Ready(Some(Ok(self.bytes())));
                }
                Pending => {
                    if self.buf.0.is_empty() {
                        break Pending;
                    }
                    break Ready(Some(Ok(self.bytes())));
                }
            }
        }
    }
}

#[cfg(feature = "logging")]
impl<InnerStream, Serializer> Drop for ByteStream<InnerStream, Serializer>
where
    InnerStream: TryStream + Unpin,
    <InnerStream as TryStream>::Error: std::fmt::Debug + std::fmt::Display + 'static,
    Serializer: FnMut(&mut BytesWriter, &<InnerStream as TryStream>::Ok) -> Result<(), actix_web::Error>
        + Unpin,
{
    #[inline]
    fn drop(&mut self) {
        if !matches!(self.state, State::Done) {
            warn!(
                "dropped ByteStream in state: {:?} after {} items",
                self.state, self.item_count
            );
        }
    }
}
