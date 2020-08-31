# seam-rs

This crate provides tools for building reliable, fault-tolerant, distributed systems.

>In sewing, a seam is the join where two or more layers of fabric, leather, or other materials are held together with stitches.

Seam-rs utilities make "stitching" together computers together in a distributed system simple, reliable, and robust.

<hr>

All sub-modules are heavily reliant on the `async-std` crate and its runtime. This dependency may be removed in the future, but definitely not anytime soon.

- **[seam-channel](seam-channel)**: distributed message queue abstraction over network streams, Kafka, SQS, etc.

## Roadmap

- **seam-partition**: distributed hash-table algorithms to partition elements across network
- **seam-consistency**: CRDTs and methods for strongly consistent transactions
- **seam-membership**: membership algorithms to determine node joins and leaves
- **seam-failure**: failure detection algorithms to identify network partitions