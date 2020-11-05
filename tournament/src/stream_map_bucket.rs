use model::Bucket;

use tokio::stream::Stream;

use std::pin::Pin;
use std::task::{Context, Poll};

pub struct StreamMapBucket<S: Stream + Unpin, M: Unpin> {
    inner: Bucket<(bool, M, S)>,
    last_polled: usize,
}

impl<S: Stream + Unpin, M: Unpin> Default for StreamMapBucket<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Stream + Unpin, M: Unpin> StreamMapBucket<S, M> {
    pub fn new() -> Self {
        StreamMapBucket {
            inner: Bucket::new(),
            last_polled: 0,
        }
    }

    /// Replaces the given data at the provided id returning the element at that location if it
    /// already exists
    pub fn insert_stream(
        &mut self,
        id: usize,
        stream: S,
        metadata: M,
        disconnectable: bool,
    ) -> Option<(bool, M, S)> {
        self.inner.insert(id, (disconnectable, metadata, stream))
    }

    pub fn add_stream(&mut self, stream: S, metadata: M) -> usize {
        self.inner.add((true, metadata, stream))
    }

    pub fn remove_stream(&mut self, id: usize) -> Option<(M, S)> {
        self.inner.remove(id).map(|(_, m, s)| (m, s))
    }

    pub fn set_disconnectable(&mut self, id: usize, disconnectable: bool) {
        self.inner.get_mut(id).map(|(d, _, _)| *d = disconnectable);
    }

    pub fn get_stream(&self, id: usize) -> Option<&S> {
        self.inner.get(id).map(|(_, _, s)| s)
    }

    pub fn get_stream_mut(&mut self, id: usize) -> Option<&mut S> {
        self.inner.get_mut(id).map(|(_, _, s)| s)
    }

    pub fn get_metadata(&self, id: usize) -> Option<&M> {
        self.inner.get(id).map(|(_, m, _)| m)
    }

    pub fn get_metadata_mut(&mut self, id: usize) -> Option<&mut M> {
        self.inner.get_mut(id).map(|(_, m, _)| m)
    }
}

pub enum StreamMapEvent<I, M> {
    Disconnection(usize, M),
    Message(usize, I),
}

impl<S: Stream + Unpin, M: Unpin> Stream for StreamMapBucket<S, M> {
    type Item = StreamMapEvent<S::Item, M>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        let max = this.inner.max_id();
        if this.last_polled > max {
            this.last_polled = max;
        }

        for loop_index in 0..max {
            let id = (loop_index + this.last_polled) % max;
            if let Some((disconnectable, _metadata, stream)) = this.inner.get_mut(id) {
                match Pin::new(stream).poll_next(cx) {
                    Poll::Ready(Some(msg)) => {
                        this.last_polled = id;
                        return Poll::Ready(Some(StreamMapEvent::Message(id, msg)));
                    }
                    Poll::Ready(None) => {
                        println!("user disconnected, disconnectable: {}", disconnectable);
                        if *disconnectable {
                            let (m, _) = this.remove_stream(id).unwrap();

                            // Alert the user that a stream has been disconnected
                            return Poll::Ready(Some(StreamMapEvent::Disconnection(id, m)));
                        }
                    }
                    Poll::Pending => {}
                }
            }
        }

        Poll::Pending
    }
}
