use bytes::BytesMut;
use tokio::sync::Mutex;

pub struct BufferPool {
    small: Mutex<Vec<BytesMut>>,
    medium: Mutex<Vec<BytesMut>>,
    large: Mutex<Vec<BytesMut>>,
}

const SMALL_MAX: usize = 2_048;
const MEDIUM_MAX: usize = 32_768;

const SMALL_DEPTH: usize = 64;
const MEDIUM_DEPTH: usize = 64;
const LARGE_DEPTH: usize = 16;

impl BufferPool {
    pub fn new() -> Self {
        Self {
            small: Mutex::new(Vec::with_capacity(SMALL_DEPTH)),
            medium: Mutex::new(Vec::with_capacity(MEDIUM_DEPTH)),
            large: Mutex::new(Vec::with_capacity(LARGE_DEPTH)),
        }
    }

    pub async fn acquire(&self, size_hint: usize) -> BytesMut {
        let (pool, min_cap) = if size_hint <= SMALL_MAX {
            (&self.small, SMALL_MAX.max(size_hint))
        } else if size_hint <= MEDIUM_MAX {
            (&self.medium, MEDIUM_MAX.max(size_hint))
        } else {
            (&self.large, size_hint)
        };

        let candidate = pool.lock().await.pop();
        match candidate {
            Some(mut buf) if buf.capacity() >= size_hint => {
                buf.clear();
                buf
            },
            _ => BytesMut::with_capacity(min_cap),
        }
    }

    pub async fn release(&self, mut buffer: BytesMut) {
        buffer.clear();
        let cap = buffer.capacity();
        let (pool, depth) = if cap <= SMALL_MAX {
            (&self.small, SMALL_DEPTH)
        } else if cap <= MEDIUM_MAX {
            (&self.medium, MEDIUM_DEPTH)
        } else {
            (&self.large, LARGE_DEPTH)
        };

        let mut p = pool.lock().await;
        if p.len() < depth {
            p.push(buffer);
        }
    }

    pub async fn depths(&self) -> (usize, usize, usize) {
        (
            self.small.lock().await.len(),
            self.medium.lock().await.len(),
            self.large.lock().await.len(),
        )
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new()
    }
}
