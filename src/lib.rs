//! This crate provides a lock-free Pub/Sub event-bus inspired by the LMAX Disruptor.
//!
//! Users can configure how publishers handle slow subscribers through `WaitStrategy`
//! policies.
//!
//! Both sync and async APIs are available.
//!
//! # Examples
//!
//! Please use the provided [example programs](https://github.com/sachanganesh/eventador-rs/tree/main/examples)
//! for a more thorough approach on how to use this crate.
//!
//! Basic sync usage:
//!
//! ```
//! use eventador::Eventador;
//! let eventbus = Eventador::new(4).unwrap();
//! let subscriber = eventbus.subscribe::<usize>();
//!
//! let i: usize = 1234;
//! eventbus.publish(i);
//!
//! let mut publisher = eventbus.publisher();
//! publisher.send(i + 1111);
//!
//! let mut msg = subscriber.recv();
//! assert_eq!(i, *msg);
//!
//! msg = subscriber.recv();
//! assert_eq!(i + 1111, *msg);
//! ```
//!
//! Basic async usage:
//!
//! ```ignore
//! use eventador::{Eventador, SinkExt, StreamExt};
//! let eventbus = Eventador::new(4).unwrap();
//!
//! let mut subscriber = eventbus.async_subscriber::<usize>();
//! let mut publisher = eventbus.async_publisher::<usize>(4);
//!
//! let i: usize = 1234;
//! publisher.send(i).await?;
//!
//! let msg = subscriber.next().await?;
//! assert_eq!(i, *msg);
//! ```
//!
//! # Why?
//!
//! Event-buses ease the development burden of concurrent programs by enabling concurrent
//! application subroutines to interact and affect other subroutines through events.
//!
//! Eventador embraces the Rust model of *Choose Your Guarantees &trade;* by offering different
//! policies for publishing when subscribers are lagging. These are represented as
//! [WaitStrategies](https://docs.rs/eventador/latest/eventador/enum.WaitStrategy.html), with the
//! default being to wait for all subscribers to read an event before it is overwritten.
//!
//! # Feature Flags
//!
//! - `async`: enables usage of async APIs
//!
//! # Design Considerations
//!
//! A general overview of the architecture of the library can be found
//! [here](https://github.com/sachanganesh/eventador-rs/blob/main/ARCHITECTURE.md).
//!
//! ## Ring Buffer
//!
//! Like Eventador, most event-bus implementations use some form of ring buffer for the underlying
//! data structure to store published events. As such, an Eventador instance cannot indefinitely
//! grow to accommodate events, unlike a `Vec`. Publishers require configurable policies to decide
//! how and when to overwrite old data in the ring.
//!
//! ## LMAX Disruptor
//!
//! The LMAX Disruptor serves as a basis for a lot of event-bus implementations, though the
//! contemporary architecture of the Disruptor looks very different from the one presented in the
//! outdated LMAX white-paper. Eventador draws from the principles of the current Disruptor
//! architecture, but the similarities stop there.
//!
//! A sequencer atomically assigns an event to an index in the ring buffer on publishing of an
//! event.
//!
//! Subscribers internally have their own sequencer to determine their last read event in the ring
//! buffer. On receiving a subscribed message, the sequencer is atomically updated to reflect that
//! it can now receive the next event.
//!
//! ## Lock-free
//!
//! Eventador has the potential to be a high-contention (aka bottlenecking) structure to a given
//! concurrent program, so the implementation needs to handle contention as effectively as possible.
//! Atomic CAS operations are generally faster than locking, and is the preferred approach to handle
//! contention.
//!
//! ## TypeId
//! This crate relies on the use of `TypeId` to determine what type an event is, and what types of
//! events a subscriber is subscribed to.
//!
//! Unfortunately, due to the limitations of Rust reflection tools, an Enum will have a different
//! TypeId than an Enum variant. This means that a subscriber must subscribe to the Enum type and
//! ignore any variants it's not interested in that it still receives. Likewise, the publisher must
//! publish events as the Enum type and not the variant in order to maintain that consistency.
//!

// #![feature(doc_cfg)]

mod alertable;
mod event;
mod publisher;
mod ring_buffer;
mod sequence;
mod subscriber;
mod wait_strategy;

#[cfg(feature = "async")]
// #[doc(cfg(feature = "async"))]
mod futures;

#[cfg(feature = "async")]
// #[doc(cfg(feature = "async"))]
pub use crate::futures::{AsyncPublisher, AsyncSubscriber, PublishError};

#[cfg(feature = "async")]
// #[doc(cfg(feature = "async"))]
pub use ::futures::{SinkExt, StreamExt};

pub use event::EventRead;
pub use publisher::Publisher;
pub use subscriber::Subscriber;
pub use wait_strategy::WaitStrategy;

use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use std::sync::Arc;

/// A lock-free and thread-safe event-bus implementation.
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
/// let mut msg = subscriber.recv();
/// assert_eq!(i, *msg);
/// ```
///
#[derive(Clone)]
pub struct Eventador {
    ring: Arc<RingBuffer>,
}

impl Eventador {
    /// Creates a new Eventador event-bus.
    ///
    /// **The capacity is required to be a power of 2.**
    ///
    /// This uses the default wait-strategy of [`WaitStrategy::AllSubscribers`], which will ensure
    /// a publisher can't overwrite an event in the ring until all subscribers have read it.
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
            ring: Arc::new(RingBuffer::new(capacity, WaitStrategy::AllSubscribers)?),
        })
    }

    /// Creates a new Eventador event-bus with a specific [`WaitStrategy`] for publishers.
    ///
    /// **The capacity is required to be a power of 2.**
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// let eventbus = Eventador::new(4, WaitStrategy::AllSubscribers)?;
    /// ```
    ///
    pub fn with_strategy(capacity: u64, wait_strategy: WaitStrategy) -> anyhow::Result<Self> {
        Ok(Self {
            ring: Arc::new(RingBuffer::new(capacity, wait_strategy)?),
        })
    }

    /// Synchronously publish an event to the event-bus.
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
    pub fn publish<T: 'static + Send + Sync>(&self, message: T) {
        let sequence = self.ring.next_slot();

        if let Some(envelope) = self.ring.get_envelope(sequence).clone() {
            envelope.overwrite::<T>(sequence, message);
        }
    }

    /// Creates a [`Publisher`] that synchronously publishes messages on the event-bus.
    ///
    /// Although the [`Eventador::publish`] function has the exact same behavior, this handle offers
    /// an API that mirrors the `AsyncPublisher`.
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
    pub fn publisher(&self) -> Publisher {
        Publisher::new(self.ring.clone())
    }

    /// Creates a [`Subscriber`] that subscribes to an event type receives them synchronously.
    ///
    /// The [`Subscriber`] will not receive subscribed events that were published to the event-bus
    /// before time of subscription. It will only receive events that are published after
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
    /// let mut msg = subscriber.recv();
    /// assert_eq!(i, *msg);
    /// ```
    ///
    pub fn subscribe<T: 'static + Send>(&self) -> Subscriber<T> {
        let sequence = Arc::new(Sequence::with_value(self.ring.sequencer().get() + 1));

        self.ring
            .sequencer()
            .register_gating_sequence(sequence.clone());

        Subscriber::new(self.ring.clone(), sequence)
    }

    /// Creates an [`AsyncPublisher`] that can publish to the event-bus asynchronously.
    ///
    /// The buffer size indicates the number of events that can be buffered until a flush is made
    /// to the event bus. Until events are flushed to the event bus, they are not yet published.
    ///
    /// Because events are buffered, an AsyncPublisher can only publish events of the same
    /// type. A new AsyncPublisher must be instantiated for events of another type.
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// let eventbus = Eventador::new(4)?;
    /// let mut publisher: AsyncPublisher<usize> = eventbus.async_publisher(10);
    ///
    /// let mut i: usize = 1234;
    /// publisher.send(i).await?;
    /// ```
    ///
    #[cfg(feature = "async")]
    // #[doc(cfg(feature = "async"))]
    pub fn async_publisher<T: 'static + Send + Sync + Unpin>(
        &self,
        buffer_size: usize,
    ) -> AsyncPublisher<T> {
        AsyncPublisher::new(self.ring.clone(), buffer_size)
    }

    /// Creates an [`AsyncSubscriber`] that subscribes to an event type and receive them
    /// asynchronously.
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
    /// let mut msg = subscriber.next().await.unwrap();
    /// assert_eq!(i, *msg);
    /// ```
    ///
    #[cfg(feature = "async")]
    // #[doc(cfg(feature = "async"))]
    pub fn async_subscriber<T: Send + Unpin>(&self) -> AsyncSubscriber<T> {
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
    #[cfg(feature = "async")]
    use crate::futures::publisher::{AsyncPublisher, PublishError};

    #[cfg(feature = "async")]
    use futures::{
        future::{AbortHandle, Abortable},
        SinkExt, StreamExt,
    };

    #[cfg(feature = "async")]
    use async_channel::unbounded;

    #[cfg(feature = "async")]
    use ntest::timeout;

    use crate::Eventador;

    #[test]
    fn publish_and_subscribe() {
        let res = Eventador::new(2);
        assert!(res.is_ok());

        let eventbus: Eventador = res.unwrap();

        let subscriber = eventbus.subscribe::<usize>();
        assert_eq!(1, subscriber.sequence());

        let mut i: usize = 1234;
        eventbus.publish(i);

        let mut msg = subscriber.recv();
        assert_eq!(i, *msg);

        i += 1111;
        let eventbus2 = eventbus.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            eventbus2.publish(i);
        });

        msg = subscriber.recv();
        assert_eq!(i, *msg);
    }

    #[async_std::test]
    #[timeout(5000)]
    #[cfg(feature = "async")]
    async fn async_publish() {
        println!("Starting test!");
        let res = Eventador::new(4);
        assert!(res.is_ok());

        let eventbus: Eventador = res.unwrap();

        let mut subscriber = eventbus.async_subscriber::<usize>();
        let mut publisher: AsyncPublisher<usize> = eventbus.async_publisher(4);

        let (sender, mut receiver) = unbounded::<Result<usize, PublishError>>();

        let mut i: usize = 1234;
        let mut sent = sender.send(Ok(i)).await;
        assert!(sent.is_ok());

        let (handle, reg) = AbortHandle::new_pair();
        async_std::task::spawn(Abortable::new(
            async move {
                publisher.send_all(&mut receiver).await.unwrap();
            },
            reg,
        ));

        let mut msg = subscriber.next().await.unwrap();
        assert_eq!(i, *msg);
        println!("Passed part 1!");

        i += 1111;
        let eventbus2 = eventbus.clone();

        async_std::task::spawn(async move {
            async_std::task::sleep(std::time::Duration::from_secs(1)).await;
            eventbus2.publish(i);
        });

        msg = subscriber.next().await.unwrap();
        assert_eq!(i, *msg);
        println!("Passed part 2!");

        i += 1111;
        sent = sender.send(Ok(i)).await;
        assert!(sent.is_ok());

        msg = subscriber.next().await.unwrap();
        assert_eq!(i, *msg);
        println!("Passed part 3! Done.");

        handle.abort();
    }

    #[derive(Debug, Eq, PartialEq)]
    enum TestEnum {
        SampleA,
    }

    #[test]
    fn enum_specific_subscription() {
        let res = Eventador::new(4);
        assert!(res.is_ok());
        println!("Passed part 1!");

        let eventbus: Eventador = res.unwrap();

        let subscriber = eventbus.subscribe::<TestEnum>();
        assert_eq!(1, subscriber.sequence());
        println!("Passed part 2!");

        eventbus.publish(TestEnum::SampleA);

        let msg = subscriber.recv();
        assert_eq!(TestEnum::SampleA, *msg);
        println!("Passed part 3! Done.");
    }
}
