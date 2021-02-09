use crate::ring_buffer::RingBuffer;
use futures::task::{Context, Poll};
use futures::Sink;
use std::pin::Pin;
use std::sync::Arc;

/// An error thrown by the [`AsyncPublisher`] as part of the `Sink` trait implementation.
///
#[derive(Debug, Clone)]
pub struct PublishError;

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "publisher encountered error it could not recover from")
    }
}

/// A handle to asynchronously publish to the event-bus.
///
/// Implements the [`Sink`] trait to asynchronously publish a stream of events to the event-bus.
///
/// # Example
///
/// Basic usage:
///
/// ```ignore
/// let eventbus = Eventador::new(4)?;
/// let mut publisher: AsyncPublisher<usize> = eventbus.async_publisher();
///
/// let mut i: usize = 1234;
/// publisher.send(i).await?;
/// ```
///
pub struct AsyncPublisher<T> {
    ring: Arc<RingBuffer>,
    buffer_size: usize,
    events: Vec<T>,
}

impl<T: 'static + Unpin> AsyncPublisher<T> {
    pub(crate) fn new(ring: Arc<RingBuffer>, buffer: usize) -> Self {
        let buffer = if buffer == 0 { buffer + 1 } else { buffer };

        Self {
            ring,
            buffer_size: buffer,
            events: Vec::with_capacity(buffer),
        }
    }

    pub(crate) fn publish(&mut self) {
        while let Some(event) = self.events.pop() {
            let sequence = self.ring.next();

            let envelope = self
                .ring
                .get_envelope(sequence)
                .expect("ring buffer was not pre-populated with empty event envelopes");

            envelope.overwrite(sequence, event);
        }
    }
}

impl<T: 'static + Unpin> Sink<T> for AsyncPublisher<T> {
    type Error = PublishError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.events.len() >= self.buffer_size {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(mut self: Pin<&mut Self>, event: T) -> Result<(), Self::Error> {
        self.events.push(event);

        Ok(())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.publish();

        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.buffer_size = 0;
        self.publish();

        drop(self);
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    // use crate::async_publisher::*;
}
