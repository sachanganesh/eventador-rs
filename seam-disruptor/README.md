# seam-channel

This crate provides a Pub/Sub event-bus based on the disruptor pattern from LMAX. It can be used in a local context as well as a distributed context to source and send events to other computers.

## Future Goals

- Documentation
- Event sourcing by storing deltas (events) and periodically squashing them to save space
- Testing
- Benchmarking

## Feature Status

| Feature                                             	| Status 	|
|-----------------------------------------------------	|--------	|
| Local MPMC Pub/Sub                     	            |       	|
| Remote MPMC Pub/Sub 	                                |       	|
| Event Sourcing                                        |       	|