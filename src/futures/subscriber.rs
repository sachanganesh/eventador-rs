use crate::event::{EventRead, EventReadLabel};
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use futures::task::{Context, Poll};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

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
/// let subscriber = disruptor.async_subscriber::<usize>();
/// let mut publisher: AsyncPublisher<usize> = disruptor.async_publisher();
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


    /// Asynchronously read an event of the correct type from the event-bus
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// let eventbus = Eventador::new(4)?;
    /// let subscriber = eventbus.subscribe::<usize>();
    ///
    /// let mut i: usize = 1234;
    /// eventbus.publish(i);
    ///
    /// let mut msg = subscriber.recv().await.unwrap();
    /// assert_eq!(i, *msg);
    /// ```
    ///
    pub async fn recv(&self) -> Option<EventRead<'a, T>> {
        loop {
            let sequence = self.sequence.get();

            loop {
                match self.ring.get_event(sequence) {
                    EventReadLabel::Irrelevant => {
                        self.sequence.increment();
                        break;
                    }

                    EventReadLabel::Relevant(event) => {
                        self.sequence.increment();
                        return Some(event);
                    }

                    EventReadLabel::Waiting => {
                        continue;
                    }
                }
            }
        }
    }
}

impl<'a, T: 'static> Stream for AsyncSubscriber<'a, T> {
    type Item = EventRead<'a, T>;

    // @todo: Needs thorough testing due to failed prior tests
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let sequence = self.sequence.get();

            match self.ring.get_event(sequence) {
                EventReadLabel::Irrelevant => {
                    self.sequence.increment();
                    continue;
                }

                EventReadLabel::Relevant(event) => {
                    self.sequence.increment();
                    return Poll::Ready(Some(event));
                }

                EventReadLabel::Waiting => {
                    return Poll::Pending;
                }
            }
        }
    }
}
