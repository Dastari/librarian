//! A seekable, read-ahead reader over one open file on an SMB share.
//!
//! A video player scrubbing asks for a range somewhere in the file and reads it in order. The
//! reader keeps a few chunks of the file in flight ahead of the read position (each a positioned
//! read on the shared open handle) so a sequential read runs at the link's speed instead of one
//! round trip per chunk, and a seek throws the window away and starts again at the new position.

use std::collections::VecDeque;
use std::io::{self, SeekFrom};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use smb2::FileReader;
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};
use tokio::task::JoinHandle;

/// Bytes asked of the server per positioned read.
const CHUNK: u64 = 2 * 1024 * 1024;
/// Chunks kept in flight beyond the one being read.
const AHEAD: usize = 2;

enum Slot {
    Pending(JoinHandle<io::Result<Vec<u8>>>),
    Ready(Vec<u8>),
}

struct Chunk {
    start: u64,
    len: u64,
    slot: Slot,
}

impl Chunk {
    fn end(&self) -> u64 {
        match &self.slot {
            Slot::Ready(data) => self.start + data.len() as u64,
            Slot::Pending(_) => self.start + self.len,
        }
    }
}

pub struct SmbReader {
    reader: Arc<FileReader>,
    size: u64,
    pos: u64,
    chunks: VecDeque<Chunk>,
    seek_target: Option<u64>,
}

impl SmbReader {
    pub fn new(reader: FileReader) -> Self {
        let size = reader.size();
        Self {
            reader: Arc::new(reader),
            size,
            pos: 0,
            chunks: VecDeque::new(),
            seek_target: None,
        }
    }

    fn schedule(&mut self, start: u64) {
        let len = CHUNK.min(self.size - start);
        let reader = self.reader.clone();
        let task = tokio::spawn(async move {
            reader
                .read_at(start, len)
                .await
                .map_err(|error| io::Error::other(error.to_string()))
        });
        self.chunks.push_back(Chunk {
            start,
            len,
            slot: Slot::Pending(task),
        });
    }

    fn drop_chunks(&mut self) {
        for chunk in self.chunks.drain(..) {
            if let Slot::Pending(task) = chunk.slot {
                task.abort();
            }
        }
    }

    /// Keep the window full from the end of the last chunk.
    fn fill_window(&mut self) {
        while self.chunks.len() < 1 + AHEAD {
            let next = self.chunks.back().map_or(self.pos, Chunk::end);
            if next >= self.size {
                break;
            }
            self.schedule(next);
        }
    }
}

impl Drop for SmbReader {
    fn drop(&mut self) {
        self.drop_chunks();
    }
}

impl AsyncRead for SmbReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        if buf.remaining() == 0 || this.pos >= this.size {
            return Poll::Ready(Ok(()));
        }
        while this
            .chunks
            .front()
            .is_some_and(|chunk| chunk.end() <= this.pos)
        {
            this.chunks.pop_front();
        }
        // After a seek the window no longer covers the position.
        if this
            .chunks
            .front()
            .is_none_or(|chunk| chunk.start > this.pos)
        {
            this.drop_chunks();
        }
        this.fill_window();
        let Some(front) = this.chunks.front_mut() else {
            return Poll::Ready(Ok(()));
        };
        if let Slot::Pending(task) = &mut front.slot {
            match Pin::new(task).poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(Ok(data))) => front.slot = Slot::Ready(data),
                Poll::Ready(Ok(Err(error))) => {
                    this.chunks.pop_front();
                    return Poll::Ready(Err(error));
                }
                Poll::Ready(Err(error)) => {
                    this.chunks.pop_front();
                    return Poll::Ready(Err(io::Error::other(error.to_string())));
                }
            }
        }
        let start = front.start;
        let Slot::Ready(data) = &front.slot else {
            return Poll::Pending;
        };
        let offset = (this.pos - start) as usize;
        if offset >= data.len() {
            // A short read ended the file early: the server's size changed under us.
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the file on the share ended before its reported size",
            )));
        }
        let take = (data.len() - offset).min(buf.remaining());
        buf.put_slice(&data[offset..offset + take]);
        this.pos += take as u64;
        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for SmbReader {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        let size = self.size as i128;
        let target = match position {
            SeekFrom::Start(offset) => offset as i128,
            SeekFrom::End(delta) => size + delta as i128,
            SeekFrom::Current(delta) => self.pos as i128 + delta as i128,
        };
        if target < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before the start of the file",
            ));
        }
        self.seek_target = Some(target as u64);
        Ok(())
    }

    fn poll_complete(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        if let Some(target) = self.seek_target.take() {
            self.pos = target;
        }
        Poll::Ready(Ok(self.pos))
    }
}
