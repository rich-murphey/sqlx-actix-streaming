use actix_web::*;
use bytes::BytesMut;
use futures::{
    prelude::*,
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
    f: Box<dyn FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>>,
}

impl<T, St> ByteStream<T, St>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
{
    #[allow(dead_code)]
    #[inline]
    pub fn pin<F>(inner: St, f: F) -> Self
    where
        F: 'static + FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>,
    {
        Self::new(Box::pin(inner), Box::new(f))
    }
    #[inline]
    pub fn new(
        inner: Pin<Box<St>>,
        f: Box<dyn FnMut(&mut BytesWriter, &T) -> Result<(), actix_web::Error>>,
    ) -> Self {
        Self {
            inner,
            state: ByteStreamState::New,
            buf_size: 2048,
            item_size: 0,
            prefix: "[".as_bytes().to_vec(),
            separator: ",".as_bytes().to_vec(),
            suffix: "]".as_bytes().to_vec(),
            f,
        }
    }
    #[allow(dead_code)]
    #[inline]
    pub fn prefix<S: ToString>(mut self, s: S) -> Self {
        self.prefix = s.to_string().into_bytes();
        self
    }
    #[allow(dead_code)]
    #[inline]
    pub fn separator<S: ToString>(mut self, s: S) -> Self {
        self.separator = s.to_string().into_bytes();
        self
    }
    #[allow(dead_code)]
    #[inline]
    pub fn suffix<S: ToString>(mut self, s: S) -> Self {
        self.suffix = s.to_string().into_bytes();
        self
    }
    #[allow(dead_code)]
    #[inline]
    pub fn size(mut self, size: usize) -> Self {
        self.buf_size = size;
        self
    }
    #[allow(dead_code)]
    pub fn json_array(inner: St) -> Self
    where
        T: serde::Serialize,
    {
        Self::pin(inner, move |buf, t| {
            serde_json::to_writer(buf, t).map_err(error::ErrorInternalServerError)
        })
    }
    #[inline]
    fn start(&mut self, buf: &mut BytesWriter) {
        self.state = ByteStreamState::Started;
        buf.0.extend_from_slice(&self.prefix);
    }
    #[inline]
    fn put_separator(&mut self, buf: &mut BytesWriter) {
        buf.0.extend_from_slice(&self.separator);
    }
    #[inline]
    fn finish(&mut self, mut buf: BytesWriter) -> web::Bytes {
        self.state = ByteStreamState::Finished;
        buf.0.extend_from_slice(&self.suffix);
        buf.0.freeze()
    }
}

impl<T, St> Stream for ByteStream<T, St>
where
    St: Stream<Item = Result<T, sqlx::Error>>,
{
    type Item = Result<web::Bytes, actix_web::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use ByteStreamState::*;
        use Poll::*;
        let mut buf = BytesWriter(BytesMut::with_capacity(self.buf_size));
        match self.state {
            New => self.start(&mut buf),
            Finished => return Ready(None),
            _ => (),
        }
        loop {
            match self.inner.poll_next_unpin(cx) {
                Ready(Some(Ok(record))) => {
                    match self.state {
                        Started => self.state = Running,
                        Running => self.put_separator(&mut buf),
                        _ => (),
                    };
                    let initial_len = buf.0.len();
                    if let Err(e) = (self.f)(&mut buf, &record) {
                        return Ready(Some(Err(error::ErrorInternalServerError(e))));
                    }
                    let item_size = buf.0.len() - initial_len;
                    if self.item_size < item_size {
                        self.item_size = item_size;
                        while self.buf_size < self.item_size * 5 / 4 {
                            self.buf_size <<= 1;
                        }
                    }
                    if self.item_size > (buf.0.capacity() - buf.0.len()) {
                        return Ready(Some(Ok(buf.0.freeze())));
                    } else {
                        continue;
                    }
                }
                Ready(Some(Err(e))) => return Ready(Some(Err(error::ErrorInternalServerError(e)))),
                Ready(None) => {
                    return if buf.0.is_empty() {
                        Ready(None)
                    } else {
                        Ready(Some(Ok(self.finish(buf))))
                    }
                }
                Pending => {
                    return if buf.0.is_empty() {
                        Pending
                    } else {
                        Ready(Some(Ok(buf.0.freeze())))
                    }
                }
            }
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
