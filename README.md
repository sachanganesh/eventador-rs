# raptor

This crate provides a lock-free Pub/Sub event-bus based on the Disruptor pattern from LMAX. Event-buses ease the development burden of concurrent programs by enabling subsystems to subscribe to specific message types and publish at will.

Both sync and async APIs are available.

## Design Considerations

Internally, *raptor* uses a ring buffer to store published events. A sequencer atomically assigns an event to an index in the ring buffer on publishing of an event.

Subscribers internally have their own sequencer to determine their last read event in the ring buffer. On receiving a subscribed message, the sequencer is atomically updated.

### Future Goals

The async implementation can be made more efficient by using the Waker pattern.

The default strategy when a subscriber is lagging (thus preventing new events from being published), is to make publishers wait until the subscriber starts catching up. This may not be the best design choice for all use cases. The Wait Strategy must be made configurable to make *raptor* versatile to any use case. Some example wait-strategies are wait-for-all (default), no-wait, wait-for-duration, wait-for-n-publishers, etc.

Fault tolerance can be partially achieved through event sourcing. By storing deltas (sequences of events) on disk and periodically squashing them to save space, a new *raptor* instance can rebuild the ring buffer from prior to the crashed state.

Documentation, testing, and most importantly, benchmarking, are not fully realized.

## Feature Status

| Feature                                             	| Status 	|
|-----------------------------------------------------	|--------	|
| Sync MPMC Pub/Sub 	                                |     ✓  	|
| Async MPMC Pub/Sub 	                                |     ✓  	|
| Wait Strategies                                       |       	|
| Event Sourcing                                        |       	|
