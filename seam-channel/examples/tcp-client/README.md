# seam-channel tcp-client example

This example program will:

1. Establish a connection with a TCP server
2. Send a `String` message to the server
3. Wait for a `String` message reply from the server

## Usage

```
export RUST_LOG=info
cargo run <ip-address-to-connect-to>
```

## Example Usage

```
export RUST_LOG=info
cargo run localhost:5678
```