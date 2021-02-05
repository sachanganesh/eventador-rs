use crate::event::EventRead;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use futures::task::{Context, Poll, Waker};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use crate::subscriber::SubscriberAlert;

impl SubscriberAlert for Waker {
    fn alert(&self) {
        self.wake_by_ref();
    }
}

/// A handle to subscribe to events and receive them asynchronously
///
/// Implements the [`Stream`] trait to offer intended events from the event-bus as an asynchronous
/// stream.
///
/// # Example
///
/// Basic usage:
///
/// ```ignore
/// let eventbus = Eventador::new(4)?;
///
/// let subscriber = eventbus.async_subscriber::<usize>();
/// let mut publisher: AsyncPublisher<usize> = eventbus.async_publisher();
///
/// let mut i: usize = 1234;
/// publisher.send(i).await?;
///
/// let mut msg = subscriber.recv().await.unwrap();
/// assert_eq!(i, *msg);
/// ```
///
pub struct AsyncSubscriber<'a, T> {
    ring: Arc<RingBuffer>,
    sequence: Arc<Sequence>,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> AsyncSubscriber<'a, T>
where
    T: 'static + Send,
{
    pub(crate) fn new(ring: Arc<RingBuffer>, sequence: Arc<Sequence>) -> Self {
        Self {
            ring,
            sequence,
            _marker: std::marker::PhantomData,
        }
    }

    /// Get the current internal sequence number for the [`AsyncSubscriber`]
    ///
    /// This sequence number signifies what events the Subscriber may have already read, and any
    /// events with a sequence value higher than this are events that are still unread.
    pub fn sequence(&self) -> u64 {
        self.sequence.get()
    }
}

impl<'a, T: 'static> Stream for AsyncSubscriber<'a, T> {
    type Item = EventRead<'a, T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let sequence = self.sequence.get();

            let envelope = self
                .ring
                .get_envelope(sequence)
                .expect("ring buffer was not pre-populated with empty event envelopes")
                .clone();

            let envelope_sequence = envelope.sequence();

            if sequence == envelope_sequence {
                self.sequence.increment();
                let event_opt: Option<EventRead<T>> = unsafe { envelope.read() };

                if let Some(event) = event_opt {
                    return Poll::Ready(Some(event));
                }
            } else if sequence > envelope_sequence {
                envelope.add_subscriber(cx.waker().clone());
                return Poll::Pending;
            } else {
                todo!()
            }
        }
    }
}
