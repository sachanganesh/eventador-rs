# seam-channel

This crate provides a distributed message queue abstraction over network streams, Kafka, SQS, etc.

By using a message queue, crate users can focus on sending and receiving messages between computers instead of low-level networking and failure recovery.

## Here be Dragons

This crate uses the `std::Any::TypeId` structure to map incoming messages to their appropriate Rust and user-defined message types. This is because Rust does not have convenient run-time type inferences utilities at the time of writing.

The `TypeId` has a severe limitation in that the value may differ between different compiler versions. Thus, a server program compiled on v1.44.0 may not be able to handle messages sent from a client program compiled on v1.45.0.

Please keep this limitation in mind when building binaries that depend on this crate. In the future, this dependency on `TypeId` will hopefully be resolved, and a stable API presented instead.

## Future Goals

- Use the `futures` crate to reduce dependency on `async-std` runtime
- Documentation
- Stable TypeId evaluation
- Connection pool (+ accounting for ordering of messages)
- Custom serialization/deserialization
- Configurable policies for handling of non-registered (unexpected) message types
- Testing
- Benchmarking

## Feature Status

| Feature                                             	| Status 	|
|-----------------------------------------------------	|--------	|
| [TCP Client](examples/tcp-client)      	            |    ✓   	|
| [TCP Server](examples/tcp-echo-server) 	            |    ✓   	|
| UDP Client                                          	|        	|
| UDP Server                                          	|        	|
| [TLS Client](examples/tls-client)      	            |    ✓   	|
| [TLS Server](examples/tls-echo-server) 	            |    ✓   	|
| QUIC Client                                         	|        	|
| QUIC Server                                         	|        	|
| SCTP Client                                         	|        	|
| SCTP Server                                         	|        	|
| DTLS-SCTP Client                                    	|        	|
| DTLS-SCTP Server                                    	|        	|
| Kafka Client                                        	|        	|
| RMQ Client                                          	|        	|
| SQS Client                                          	|        	|
| NSQ Client                                          	|        	|