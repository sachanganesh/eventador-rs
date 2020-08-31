# seam-channel tcp-echo-server example

This example program will:

1. Bind to an IP address
2. Accept any number of TCP connections
3. Handle each connection by:
    1. Waiting for `String` messages to be received
    2. Echoing the `String` message back to the source

## Usage

```
export RUST_LOG=info
cargo run <ip-address-to-bind-to>
```

## Example Usage

```
export RUST_LOG=info
cargo run localhost:5678
```