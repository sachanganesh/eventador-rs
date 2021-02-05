use crate::event::EventRead;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crossbeam::sync::Unparker;
use std::sync::Arc;

/// A handle to receive events that were subscribed to from the event-bus
///
/// The [`Subscriber`] will not receive intended events that were published to the event-bus
/// before time of subscription. It will only receive intended events that are published after the
/// time of subscription, as they will have a higher sequence number than the Subscriber's internal
/// sequence value.
///
/// # Example
///
/// Basic usage:
///
/// ```ignore
/// let eventbus = Eventador::new(4)?;
///
/// // subscribe first, before publishing!
/// let subscriber = eventbus.subscribe::<usize>();
///
/// let mut i: usize = 1234;
/// eventbus.publish(i);
///
/// let mut msg = subscriber.recv().unwrap();
/// assert_eq!(i, *msg);
/// ```
///
pub struct Subscriber<'a, T> {
    ring: &'a RingBuffer,
    sequence: Arc<Sequence>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T> Subscriber<'a, T>
where
    T: 'static + Send,
{
    pub(crate) fn new(ring: &'a RingBuffer, sequence: Arc<Sequence>) -> Self {
        Self {
            ring,
            sequence,
            _marker: std::marker::PhantomData,
        }
    }

    /// Get the current internal sequence number for the [`Subscriber`]
    ///
    /// This sequence number signifies what events the Subscriber may have already read, and any
    /// events with a sequence value higher than this are events that are still unread.
    pub fn sequence(&self) -> u64 {
        self.sequence.get()
    }

    /// Synchronously read an event of the correct type from the event-bus
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
    /// let mut msg = subscriber.recv().unwrap();
    /// assert_eq!(i, *msg);
    /// ```
    ///
    pub fn recv<'b>(&self) -> Option<EventRead<'b, T>> {
        let sequence = self.sequence.increment();
        self.ring.get_event(sequence)
    }
}

pub(crate) trait SubscriberAlert {
    fn alert(&self);
}

impl SubscriberAlert for Unparker {
    fn alert(&self) {
        self.unpark();
    }
}
