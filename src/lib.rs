//! This crate provides a lock-free Pub/Sub event-bus based on the Disruptor pattern from LMAX.
//!
//! Both sync and async APIs are available.
//!
//! # Why?
//!
//! Event-buses ease the development burden of concurrent programs by enabling concurrent
//! application subroutines to interact and affect other subroutines through events. Of course,
//! a poor implementation can become a serious bottleneck depending on the application's needs.
//!
//! Eventador supports the Rust model of *Choose Your Guarantees &trade;* by presenting
//! configuration options for how to handle event publishing when consumers are lagging.
//! Providing this configurable interface is currently a work in progress.
//!
//! # Examples
//!
//! Please use the provided [examples](#) for a more thorough approach on how to use this crate.
//!
//! # Design Considerations
//!
//! ## Ring Buffer
//!
//! Like Eventador, most event-bus implementations use some form of ring buffer for the underlying
//! data structure to store published events. As such, an Eventador instance cannot indefinitely
//! grow to accommodate events, unlike a [`Vec`]. In the strictest model (and Eventador's default
//! approach), new events must overwrite the oldest event that has already been read by all its
//! subscribers. In other words, publishers cannot publish an event to the ring buffer until all
//! subscribers for the next overwrite-able event have consumed it. This model favors the
//! subscribers so that no event is lost or overwritten without first being handled by every
//! concerned party.
//!
//! Other implementations, like [bus-queue](https://github.com/filipdulic/bus-queue), solve this
//! problem by ignoring lagging subscribers, and treating publishers as first-class operators. This
//! is the opposite extreme to Eventador's default.
//!
//! Ultimately, there should not have to be a compromise between what a user wants to prioritize.
//! How an event-bus handles the lagging-consumer problem should be left to the user to decide
//! through configuration.
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
//!
//!

mod eventador;
mod event;
mod futures;
mod ring_buffer;
mod sequence;
mod subscriber;

pub use crate::futures::{AsyncPublisher, AsyncSubscriber};
pub use eventador::Eventador;
pub use event::EventRead;
pub use subscriber::Subscriber;
pub use ::futures::{SinkExt, StreamExt};
