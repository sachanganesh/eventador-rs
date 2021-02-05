use crate::futures::publisher::AsyncPublisher;
use crate::futures::subscriber::AsyncSubscriber;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::subscriber::Subscriber;
use std::sync::Arc;

/// A lock-free and thread-safe event-bus implementation
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
#[derive(Clone)]
pub struct Eventador {
    ring: Arc<RingBuffer>,
}

impl Eventador {
    /// Creates a new Eventador event-bus
    ///
    /// **The capacity is required to be a power of 2.**
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// let eventbus = Eventador::new(4)?;
    /// ```
    ///
    pub fn new(capacity: u64) -> anyhow::Result<Self> {
        Ok(Self {
            ring: Arc::new(RingBuffer::new(capacity)?),
        })
    }

    /// Publishes an event on the event-bus
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// let eventbus = Eventador::new(4)?;
    ///
    /// let mut i: usize = 1234;
    /// eventbus.publish(i);
    /// ```
    ///
    pub fn publish<T: 'static + Send>(&self, message: T) {
        let sequence = self.ring.next();

        if let Some(event_store) = self.ring.get_envelope(sequence).clone() {
            event_store.overwrite::<T>(sequence, message);
        }
    }

    /// Creates a [`Subscriber`] that is subscribed to events of the provided type
    ///
    /// The [`Subscriber`] will not receive intended events that were published to the event-bus
    /// before time of subscription. It will only receive intended events that are published after
    /// time of subscription.
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
    pub fn subscribe<T: 'static + Send>(&self) -> Subscriber<T> {
        let sequence = Arc::new(Sequence::with_value(self.ring.sequencer().get() + 1));
        self.ring
            .sequencer()
            .register_gating_sequence(sequence.clone());

        Subscriber::new(self.ring.as_ref(), sequence)
    }

    /// Creates an [`AsyncPublisher`] that can publish to the event-bus asynchronously
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
    pub fn async_publisher<T: 'static + Send + Unpin>(&self) -> AsyncPublisher<T> {
        AsyncPublisher::new(self.ring.clone())
    }

    /// Creates an [`AsyncSubscriber`] that can subscribe to events and receive them asynchronously
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
    pub fn async_subscriber<T: 'static + Send + Unpin>(&self) -> AsyncSubscriber<T> {
        let sequence = Arc::new(Sequence::with_value(self.ring.sequencer().get() + 1));
        self.ring
            .sequencer()
            .register_gating_sequence(sequence.clone());

        AsyncSubscriber::new(self.ring.clone(), sequence)
    }
}

impl From<RingBuffer> for Eventador {
    fn from(ring: RingBuffer) -> Self {
        Self {
            ring: Arc::new(ring),
        }
    }
}

impl From<Arc<RingBuffer>> for Eventador {
    fn from(ring: Arc<RingBuffer>) -> Self {
        Self { ring }
    }
}

#[cfg(test)]
mod tests {
    use crate::eventador::Eventador;
    use crate::futures::publisher::AsyncPublisher;
    use async_channel::{unbounded, RecvError};
    use futures::SinkExt;

    #[test]
    fn publish_and_subscribe() {
        let res = Eventador::new(4);
        assert!(res.is_ok());

        let disruptor: Eventador = res.unwrap();

        let subscriber = disruptor.subscribe::<usize>();
        assert_eq!(1, subscriber.sequence()); // @todo double check if it should be this way

        let mut i: usize = 1234;
        disruptor.publish(i);

        let mut msg = subscriber.recv().unwrap();
        assert_eq!(i, *msg);

        i += 1;
        let disruptor2 = disruptor.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            disruptor2.publish(i);
        });

        msg = subscriber.recv().unwrap();
        assert_eq!(i, *msg);
    }

    #[async_std::test]
    async fn async_publish() { // @todo revisit
        let res = Eventador::new(4);
        assert!(res.is_ok());

        let disruptor: Eventador = res.unwrap();

        let subscriber = disruptor.async_subscriber::<usize>();
        let mut publisher: AsyncPublisher<usize> = disruptor.async_publisher();

        let (sender, mut receiver) = unbounded::<Result<usize, RecvError>>();

        let mut i: usize = 1234;
        let mut sent = sender.send(Ok(i)).await;
        assert!(sent.is_ok());

        let _handle = async_std::task::spawn(async move {
            publisher.send_all(&mut receiver).await.unwrap();
        });

        let mut msg = subscriber.recv().await.unwrap();
        assert_eq!(i, *msg);

        i += 1;
        let disruptor2 = disruptor.clone();

        async_std::task::spawn(async move {
            async_std::task::sleep(std::time::Duration::from_secs(3)).await;
            disruptor2.publish(i);
        });

        msg = subscriber.recv().await.unwrap();
        assert_eq!(i, *msg);

        i += 1;
        sent = sender.send(Ok(i)).await;
        assert!(sent.is_ok());

        msg = subscriber.recv().await.unwrap();
        assert_eq!(i, *msg);
    }

    #[derive(Debug, Eq, PartialEq)]
    enum TestEnum {
        SampleA,
    }

    #[test]
    fn enum_specific_subscription() {
        let res = Eventador::new(4);
        assert!(res.is_ok());

        let disruptor: Eventador = res.unwrap();

        let subscriber = disruptor.subscribe::<TestEnum>();
        assert_eq!(1, subscriber.sequence()); // @todo double check if it should be this way

        disruptor.publish(TestEnum::SampleA);

        let msg = subscriber.recv().unwrap();
        assert_eq!(TestEnum::SampleA, *msg);
    }
}
