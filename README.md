# eventador-rs

[![Crates.io][crates-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]

[crates-badge]: https://img.shields.io/crates/v/eventador.svg
[crates-url]: https://crates.io/crates/eventador
[docs-badge]: https://docs.rs/eventador/badge.svg
[docs-url]: https://docs.rs/eventador

This crate provides a lock-free Pub/Sub event-bus inspired by the LMAX Disruptor.

Users can configure how publishers handle slow subscribers through `WaitStrategy`
policies.

Both sync and async APIs are available.

## Examples

Please use the provided [example programs](https://github.com/sachanganesh/eventador-rs/tree/main/examples)
for a more thorough approach on how to use this crate.

Basic sync usage:

````rust
use eventador::Eventador;

let eventbus = Eventador::new(4).unwrap();
let subscriber = eventbus.subscribe::<usize>();

let i: usize = 1234;
eventbus.publish(i);

let mut publisher = eventbus.publisher();
publisher.send(i + 1111);

let mut msg = subscriber.recv();
assert_eq!(i, *msg);

msg = subscriber.recv();
assert_eq!(i + 1111, *msg);
````

Basic async usage:

````rust
use eventador::{Eventador, SinkExt, StreamExt};

let eventbus = Eventador::new(4).unwrap();

let mut subscriber = eventbus.async_subscriber::<usize>();
let mut publisher = eventbus.async_publisher::<usize>(4);

let i: usize = 1234;
publisher.send(i).await?;

let msg = subscriber.next().await?;
assert_eq!(i, *msg);
````

## Why?

Event-buses ease the development burden of concurrent programs by enabling concurrent
application subroutines to interact and affect other subroutines through events.

Eventador embraces the Rust model of *Choose Your Guarantees &trade;* by offering different
policies for publishing when subscribers are lagging. These are represented as
[WaitStrategies](https://docs.rs/eventador/latest/eventador/enum.WaitStrategy.html), with the
default being to wait for all subscribers to read an event before it is overwritten.

## Design Considerations

A general overview of the architecture of the library can be found [here](ARCHITECTURE.md).

### Ring Buffer

Like Eventador, most event-bus implementations use some form of ring buffer for the underlying
data structure to store published events. As such, an Eventador instance cannot indefinitely
grow to accommodate events, unlike a `Vec`. Publishers require configurable policies to decide
how and when to overwrite old data in the ring.

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
events a subscriber is subscribed to.

Unfortunately, due to the limitations of Rust reflection tools, an Enum will have a different
TypeId than an Enum variant. This means that a subscriber must subscribe to the Enum type and
ignore any variants it's not interested in that it still receives. Likewise, the publisher must
publish events as the Enum type and not the variant in order to maintain that consistency.

## Feature Status

Testing, and most importantly, benchmarking, are not fully realized.

| Feature                                             	| Status 	|
|-----------------------------------------------------	|--------	|
| Sync MPMC Pub/Sub 	                                |     ✓  	|
| Async MPMC Pub/Sub 	                                |     ✓  	|
| Wait Strategies                                       |     ✓ 	|
