# seam-channel

This crate provides a message queue abstraction over traditional network I/O.

By using a message queue, crate users can focus on sending and receiving messages between computers instead of low-level networking and failure recovery.

## Future Goals

- Custom serialization/deserialization
- Configurable policies for handling of non-registered (unexpected) message types

## Feature Status

| Feature                                             	| Status 	|
|-----------------------------------------------------	|--------	|
| [TCP Client](examples/tcp-client)      	            |    ✓   	|
| [TCP Server](examples/tcp-echo-server) 	            |    ✓   	|
| UDP Client                                          	|        	|
| UDP Server                                          	|        	|
| [TLS Client](examples/tls-client)      	            |    ✓   	|
| [TCP Server](examples/tls-echo-server) 	            |    ✓   	|
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