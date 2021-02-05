# eventador-rs

[![Crates.io][crates-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]

[crates-badge]: https://img.shields.io/crates/v/eventador.svg
[crates-url]: https://crates.io/crates/eventador
[docs-badge]: https://docs.rs/eventador/badge.svg
[docs-url]: https://docs.rs/eventador

This crate provides a lock-free Pub/Sub event-bus based on the Disruptor pattern from LMAX.

Both sync and async APIs are available.

## Examples

Basic sync usage:

```rust
let eventbus = Eventador::new(4)?;
let subscriber = eventbus.subscribe::<usize>();

let mut i: usize = 1234;
eventbus.publish(i);

let mut msg = subscriber.recv().unwrap();
assert_eq!(i, *msg);
```

Basic async usage:

```rust
let eventbus = Eventador::new(4)?;

let subscriber = eventbus.async_subscriber::<usize>();
let mut publisher: AsyncPublisher<usize> = eventbus.async_publisher();

let mut i: usize = 1234;
publisher.send(i).await?;

let mut msg = subscriber.recv().await.unwrap();
assert_eq!(i, *msg);
```

Please use the provided [example programs](#) for a more thorough approach on how to use this
crate.

## Why?

Event-buses ease the development burden of concurrent programs by enabling concurrent
application subroutines to interact and affect other subroutines through events. Of course,
a poor implementation can become a serious bottleneck depending on the application's needs.

Eventador supports the Rust model of *Choose Your Guarantees &trade;* by presenting
configuration options for how to handle event publishing when consumers are lagging.
Providing this configurable interface is currently a work in progress.

## Design Considerations

### Ring Buffer

Like Eventador, most event-bus implementations use some form of ring buffer for the underlying
data structure to store published events. As such, an Eventador instance cannot indefinitely
grow to accommodate events, unlike a [`Vec`]. In the strictest model (and Eventador's default
approach), new events must overwrite the oldest event that has already been read by all its
subscribers. In other words, publishers cannot publish an event to the ring buffer until all
subscribers for the next overwrite-able event have consumed it. This model favors the
subscribers so that no event is lost or overwritten without first being handled by every
concerned party.

Other implementations, like [bus-queue](https://github.com/filipdulic/bus-queue), solve this
problem by ignoring lagging subscribers, and treating publishers as first-class operators. This
is the opposite extreme to Eventador's default.

Ultimately, there should not have to be a compromise between what a user wants to prioritize.
How an event-bus handles the lagging-consumer problem should be left to the user to decide
through configuration.

### LMAX Disruptor

The LMAX Disruptor serves as a basis for a lot of event-bus implementations, though the
contemporary architecture of the Disruptor looks very different from the one presented in the
outdated LMAX white-paper. Eventador draws from the principles of the current Disruptor
architecture, but the similarities stop there.

A sequencer atomically assigns an event to an index in the ring buffer on publishing of an
event.

Subscribers internally have their own sequencer to determine their last read event in the ring
buffer. On receiving a subscribed message, the sequencer is atomically updated to reflect that
it can now receive the next event.

### Lock-free

Eventador has the potential to be a high-contention (aka bottlenecking) structure to a given
concurrent program, so the implementation needs to handle contention as effectively as possible.
Atomic CAS operations are generally faster than locking, and is the preferred approach to handle
contention.

### TypeId
This crate relies on the use of `TypeId` to determine what type an event is, and what types of
events a subscriber subscribes to.

## Future Goals

The async implementation can be made more efficient by using the Waker
pattern.

The default strategy when a subscriber is lagging (thus preventing new
events from being published), is to make publishers wait until the
subscriber starts catching up. This may not be the best design choice
for all use cases. The Wait Strategy must be made configurable to make
Eventador versatile to any use case. Some example wait-strategies are
wait-for-all (default), no-wait, wait-for-duration,
wait-for-n-publishers, etc.

Fault tolerance can be partially achieved through event sourcing. By
storing deltas (sequences of events) on disk and periodically squashing
them to save space, a new *raptor* instance can rebuild the ring buffer
from prior to the crashed state.

Testing, and most importantly, benchmarking, are not
fully realized.

## Feature Status

| Feature                                             	| Status 	|
|-----------------------------------------------------	|--------	|
| Sync MPMC Pub/Sub 	                                |     ✓  	|
| Async MPMC Pub/Sub 	                                |     ✓  	|
| Wait Strategies                                       |       	|
| Event Sourcing                                        |       	|
