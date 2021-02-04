use crate::ring_buffer::RingBuffer;
use async_channel::RecvError;
use futures::task::{Context, Poll};
use futures::Sink;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AsyncPublishError;

impl std::fmt::Display for AsyncPublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "publisher encountered error it could not recover from")
    }
}

pub struct AsyncPublisher<T> {
    ring: Arc<RingBuffer>,
    sequence: Option<u64>,
    event: Option<T>,
}

impl<T: 'static + Unpin> AsyncPublisher<T> {
    pub(crate) fn new(ring: Arc<RingBuffer>) -> Self {
        Self {
            ring,
            sequence: None,
            event: None,
        }
    }

    pub(crate) fn write_to_ring(&mut self) {
        if let Some(sequence) = self.sequence.take() {
            if let Some(envelope) = self.ring.get_envelope(sequence) {
                if let Some(event) = self.event.take() {
                    envelope.overwrite(sequence, event);
                }
            }
        }
    }
}

impl<T: 'static + Unpin> Sink<T> for AsyncPublisher<T> {
    type Error = RecvError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.event.is_some() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let sequence = self.ring.next();

        self.sequence.replace(sequence);
        self.event.replace(item);

        Ok(())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.write_to_ring();
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        drop(self);
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    // use crate::async_publisher::*;
}
