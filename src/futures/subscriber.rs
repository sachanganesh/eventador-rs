use crate::alertable::Alertable;
use crate::event::EventRead;
use crate::ring_buffer::{EventWrapper, RingBuffer};
use crate::sequence::Sequence;
use crate::WaitStrategy;
use futures::task::{Context, Poll, Waker};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

impl Alertable for Waker {
    fn alert(&self) {
        self.wake_by_ref();
    }
}

/// A handle to subscribe to events and receive them asynchronously.
///
/// Implements the [`Stream`] trait to offer subscribed events from the event-bus as an asynchronous
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
    current_event: Option<EventWrapper>,
    _marker: std::marker::PhantomData<&'a T>,
}

unsafe impl<'a, T> Send for AsyncSubscriber<'a, T> {}

impl<'a, T> AsyncSubscriber<'a, T>
where
    T: Send,
{
    pub(crate) fn new(ring: Arc<RingBuffer>, sequence: Arc<Sequence>) -> Self {
        Self {
            ring,
            sequence,
            current_event: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Get the current internal sequence number for the [`AsyncSubscriber`].
    ///
    /// This sequence number signifies what events the Subscriber may have already read, and any
    /// events with a sequence value higher than this are events that are still unread.
    pub fn sequence(&self) -> u64 {
        self.sequence.get()
    }
}

impl<'a, T: 'static> Stream for AsyncSubscriber<'a, T> {
    type Item = EventRead<'a, T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let sequence = self.sequence.get();
            let envelope = if let Some(envelope) = self.current_event.take() {
                envelope
            } else {
                let envelope = self
                    .ring
                    .get_envelope(sequence)
                    .expect("ring buffer was not pre-populated with empty event envelopes")
                    .clone();

                envelope.start_waiting();
                envelope
            };

            let envelope_sequence = envelope.sequence();
            if sequence == envelope_sequence {
                let event_opt: Option<EventRead<T>> = unsafe { envelope.read() };
                envelope.stop_waiting();

                self.sequence.increment();
                if let Some(event) = event_opt {
                    return Poll::Ready(Some(event));
                }
            } else if sequence > envelope_sequence {
                envelope.add_subscriber(Box::new(cx.waker().clone()));
                self.current_event.replace(envelope);
                return Poll::Pending;
            } else {
                // Publisher has overwritten an event that has not been read yet
                match self.ring.wait_strategy() {
                    WaitStrategy::AllSubscribers => unreachable!(),

                    _ => {
                        self.sequence.set(envelope_sequence);
                    }
                }
            }
        }
    }
}
