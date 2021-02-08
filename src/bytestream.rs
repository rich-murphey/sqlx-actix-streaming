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
enum ByteStreamState {
    Unused,
    Empty,
    NonEmpty,
    Error,
    Done,
}
#[cfg(feature = "logging")]
use log::*;

pub struct ByteStream<T, St, F>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
    F: FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>,
{
    inner: Pin<Box<St>>,
    state: ByteStreamState,
    buf_size: usize,
    item_size: usize,
    prefix: Vec<u8>,
    separator: Vec<u8>,
    suffix: Vec<u8>,
    buf: BytesWriter,
    serializer: Box<F>,
    items: usize,
}

impl<T, St, F> ByteStream<T, St, F>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
    F: FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>,
{
    #![allow(dead_code)]
    const DEFAULT_BUF_SIZE: usize = 2048;
    pub fn new(inner: Pin<Box<St>>, serializer: Box<F>) -> Self {
        // TODO: this should be a builder.
        Self {
            inner,
            state: ByteStreamState::Unused,
            buf_size: Self::DEFAULT_BUF_SIZE,
            item_size: 0,
            prefix: "[".as_bytes().to_vec(),
            separator: ",".as_bytes().to_vec(),
            suffix: "]".as_bytes().to_vec(),
            buf: BytesWriter(BytesMut::with_capacity(Self::DEFAULT_BUF_SIZE)),
            serializer,
            items: 0,
        }
    }
    /// pin and box the stream and box the serializer function.
    pub fn make(inner: St, serializer: F) -> Self {
        Self::new(Box::pin(inner), Box::new(serializer))
    }
    /// set the prefix for the json array. '[' by default.
    pub fn prefix<S: ToString>(mut self, s: S) -> Self {
        self.prefix = s.to_string().into_bytes();
        self
    }
    /// set the separator for the json array elements. ',' by default.
    pub fn separator<S: ToString>(mut self, s: S) -> Self {
        self.separator = s.to_string().into_bytes();
        self
    }
    /// set the suffix for the json array. ']' by default.
    pub fn suffix<S: ToString>(mut self, s: S) -> Self {
        self.suffix = s.to_string().into_bytes();
        self
    }
    /// set the buffer size for the json text.
    pub fn size(mut self, size: usize) -> Self {
        self.buf_size = size;
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
    fn reserve(&mut self) {
        self.buf.0.reserve(self.buf_size);
    }
    // Make the buffer 20% larger than the largest item, and round it
    // up to a power of two.
    #[inline]
    fn adjust_item_size(&mut self, inital_len: usize) {
        let item_size = self.buf.0.len() - inital_len;
        if self.item_size < item_size {
            self.item_size = item_size;
            while self.buf_size < self.item_size * 5 / 4 {
                self.buf_size <<= 1;
            }
        }
    }
    // return true if there is room for one more item in the buffer.
    #[inline]
    fn has_room_for_item(&self) -> bool {
        let remaining_space = self.buf.0.capacity() - self.buf.0.len();
        self.item_size <= remaining_space
    }
    // use the given closure to write a record to the output buffer.
    #[inline]
    fn write_record(&mut self, record: &T) -> Result<(), actix_web::Error> {
        (self.serializer)(&mut self.buf, record)
    }
}

impl<T, St, F> Stream for ByteStream<T, St, F>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
    F: FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>,
{
    type Item = Result<Bytes, actix_web::Error>;


    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use ByteStreamState::*;
        use Poll::*;
        if let Done = self.state {
            return Ready(None);
        }
        self.reserve();
        if let Unused = self.state {
            self.state = Empty;
            self.put_prefix();
        }
        loop {
            match self.inner.poll_next_unpin(cx) {
                Ready(Some(Ok(record))) => {
                    self.items += 1;
                    match self.state {
                        Empty => self.state = NonEmpty,
                        NonEmpty => self.put_separator(),
                        _ => (),
                    };
                    let initial_len = self.buf.0.len();
                    if let Err(e) = self.write_record(&record) {
                        #[cfg(feature = "logging")]
                        error!("write_record: {:?}", e);
                        return Ready(Some(Err(ErrorInternalServerError(e))));
                    }
                    self.adjust_item_size(initial_len);
                    if self.has_room_for_item() {
                        continue;
                    }
                    return Ready(Some(Ok(self.get_bytes())));
                }
                Ready(Some(Err(e))) => {
                    self.state = Error;
                    #[cfg(feature = "logging")]
                    error!("poll_next: {:?}", e);
                    return Ready(Some(Err(ErrorInternalServerError(e))));
                }
                Ready(None) => {
                    self.state = Done;
                    self.put_suffix();
                    return Ready(Some(Ok(self.get_bytes())));
                }
                Pending => {
                    if self.buf.0.is_empty() {
                        return Pending;
                    }
                    return Ready(Some(Ok(self.get_bytes())));
                }
            }
        }
    }
}

#[cfg(feature = "logging")]
impl<T, St, F> Drop for ByteStream<T, St, F>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
    F: FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>,
{
    fn drop(&mut self) {
        if !matches!(self.state, ByteStreamState::Done) {
            error!("dropped ByteStream in state: {:?} after {} items", self.state, self.items);
        }
    }
}

pub struct BytesWriter(pub BytesMut);

impl std::io::Write for BytesWriter {
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
