use actix_web::{
    error::ErrorInternalServerError,
    web::{Bytes, BytesMut},
};
use futures::{
    Stream,
    StreamExt,
    task::{Context, Poll},
};
pub use std::io::Write;
use std::pin::Pin;

enum ByteStreamState {
    New,
    Started,
    Running,
    Finished,
}

pub struct ByteStream<T, St>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
{
    inner: Pin<Box<St>>,
    state: ByteStreamState,
    buf_size: usize,
    item_size: usize,
    prefix: Vec<u8>,
    separator: Vec<u8>,
    suffix: Vec<u8>,
    buf: BytesWriter,
    f: Box<dyn FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>>,
}

impl<T, St> ByteStream<T, St>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
{
    #![allow(dead_code)]
    const DEFAULT_BUF_SIZE: usize = 4096; 

    pub fn new(
        inner: Pin<Box<St>>,
        f: Box<dyn FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>>,
    ) -> Self {
        // TODO: this should be a builder.
        Self {
            inner,
            state: ByteStreamState::New,
            buf_size: Self::DEFAULT_BUF_SIZE,
            item_size: 0,
            prefix: "[".as_bytes().to_vec(),
            separator: ",".as_bytes().to_vec(),
            suffix: "]".as_bytes().to_vec(),
            buf: BytesWriter(BytesMut::with_capacity(Self::DEFAULT_BUF_SIZE)),
            f,
        }
    }
    pub fn pin<F>(inner: St, f: F) -> Self
    where
        F: 'static + FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>,
    {
        Self::new(Box::pin(inner), Box::new(f))
    }
    pub fn prefix<S: ToString>(mut self, s: S) -> Self {
        self.prefix = s.to_string().into_bytes();
        self
    }
    pub fn separator<S: ToString>(mut self, s: S) -> Self {
        self.separator = s.to_string().into_bytes();
        self
    }
    pub fn suffix<S: ToString>(mut self, s: S) -> Self {
        self.suffix = s.to_string().into_bytes();
        self
    }
    pub fn size(mut self, size: usize) -> Self {
        self.buf_size = size;
        self
    }
    pub fn json_array(inner: St) -> Self
    where
        T: serde::Serialize,
    {
        Self::pin(inner, move |buf, t| {
            serde_json::to_writer(buf, t).map_err(ErrorInternalServerError)
        })
    }
    fn put_prefix(&mut self) {
        self.buf.0.extend_from_slice(&self.prefix);
    }
    fn put_separator(&mut self) {
        self.buf.0.extend_from_slice(&self.separator);
    }
    fn put_suffix(&mut self) {
        self.buf.0.extend_from_slice(&self.suffix);
    }
    fn get_bytes(&mut self) -> Bytes {
        self.buf.0.split().freeze()
    }
    fn reserve(&mut self) {
        self.buf.0.reserve(self.buf_size);
    }
    fn adjust_item_size(&mut self, inital_len: usize) {
        let item_size = self.buf.0.len() - inital_len;
        if self.item_size < item_size {
            self.item_size = item_size;
            while self.buf_size < self.item_size * 5 / 4 {
                self.buf_size <<= 1;
            }
        }
    }
    fn has_room_for_item(&self) -> bool {
        self.item_size <= self.buf.0.capacity() - self.buf.0.len()
    }
    fn write_record(&mut self, record: &T) -> Result<(), actix_web::Error> {
        (self.f)(&mut self.buf, record)
    }
}

impl<T, St> Stream for ByteStream<T, St>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
{
    type Item = Result<Bytes, actix_web::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use ByteStreamState::*;
        use Poll::*;
        if let Finished = self.state {
            return Ready(None);
        }
        self.reserve();
        if let New = self.state {
            self.state = ByteStreamState::Started;
            self.put_prefix();
        }
        loop {
            match self.inner.poll_next_unpin(cx) {
                Ready(Some(Ok(record))) => {
                    match self.state {
                        Started => self.state = Running,
                        Running => self.put_separator(),
                        _ => (),
                    };
                    let initial_len = self.buf.0.len();
                    if let Err(e) = self.write_record(&record) {
                        return Ready(Some(Err(ErrorInternalServerError(e))));
                    }
                    self.adjust_item_size(initial_len);
                    if self.has_room_for_item() {
                        continue;
                    } else {
                        return Ready(Some(Ok(self.get_bytes())));
                    }
                }
                Ready(Some(Err(e))) => return Ready(Some(Err(ErrorInternalServerError(e)))),
                Ready(None) => {
                    self.state = ByteStreamState::Finished;
                    self.put_suffix();
                    return Ready(Some(Ok(self.get_bytes())));
                }
                Pending => {
                    if self.buf.0.is_empty() {
                        return Pending;
                    } else {
                        return Ready(Some(Ok(self.get_bytes())));
                    }
                }
            }
        }
    }
}

pub struct BytesWriter(pub BytesMut);

impl std::io::Write for BytesWriter {
    fn write(&mut self, src: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(src);
        Ok(src.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
