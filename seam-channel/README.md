# seam-channel

This crate provides a message queue abstraction over traditional network I/O.

By using a message queue, crate users can focus on sending and receiving messages between computers instead of low-level networking and failure recovery.

## Roadmap

- Allow users to specify their own serialization/deserialization formats
- Allow users to configure handling of non-registered (unexpected) message types