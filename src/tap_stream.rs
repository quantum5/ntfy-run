use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const BUF_SIZE: usize = 16384;

pub struct TapStream<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    source: R,
    target: W,
    buffer: [u8; BUF_SIZE],
    buf_start: usize,
    buf_end: usize,
}

pub enum ReadOrWrite<'a> {
    Read(&'a [u8]),
    Written,
    EOF,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> TapStream<R, W> {
    pub fn new(source: R, target: W) -> TapStream<R, W> {
        TapStream {
            source,
            target,
            buffer: [0; BUF_SIZE],
            buf_start: 0,
            buf_end: 0,
        }
    }

    pub async fn step(&mut self) -> io::Result<ReadOrWrite> {
        if self.buf_start == self.buf_end {
            let bytes = self.source.read(&mut self.buffer[..]).await?;
            self.buf_start = 0;
            self.buf_end = bytes;

            if bytes == 0 {
                Ok(ReadOrWrite::EOF)
            } else {
                Ok(ReadOrWrite::Read(&self.buffer[0..bytes]))
            }
        } else {
            let bytes = self
                .target
                .write(&self.buffer[self.buf_start..self.buf_end])
                .await?;

            self.buf_start += bytes;
            Ok(ReadOrWrite::Written)
        }
    }
}
