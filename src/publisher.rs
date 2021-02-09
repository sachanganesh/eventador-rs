use crate::futures::AsyncPublisher;
use crate::ring_buffer::RingBuffer;
use crate::Eventador;
use std::sync::Arc;

/// A handle to publish events to the event-bus.
///
/// Although the [`Eventador::publish`] function has the exact same behavior, this handle offers an API
/// that mirrors the [`AsyncPublisher`].
///
/// # Example
///
/// Basic usage:
///
/// ```ignore
/// let eventbus = Eventador::new(4)?;
/// let mut publisher = eventbus.publisher();
///
/// let i: usize = 1234;
/// publisher.send(i);
/// ```
///
pub struct Publisher {
    ring: Arc<RingBuffer>,
}

impl Publisher {
    pub(crate) fn new(ring: Arc<RingBuffer>) -> Self {
        Self { ring }
    }

    /// Publish an event on the event-bus.
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// let eventbus = Eventador::new(4)?;
    ///
    /// let i: usize = 1234;
    /// eventbus.publish(i);
    /// ```
    ///
    pub fn send<T: 'static + Send>(&mut self, event: T) {
        let sequence = self.ring.next();

        let envelope = self
            .ring
            .get_envelope(sequence)
            .expect("ring buffer was not pre-populated with empty event envelopes");

        envelope.overwrite(sequence, event);
    }
}
