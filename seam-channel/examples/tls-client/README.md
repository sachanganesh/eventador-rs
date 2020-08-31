# seam-channel tls-client example

This example program will:

1. Establish a secure connection with a TLS server
2. Send a `String` message to the server
3. Wait for a `String` message reply from the server

## Usage

```
export RUST_LOG=info
cargo run <ip-address-to-connect-to> <domain-name> <ca-file>
```

## Example Usage

```
export RUST_LOG=info
cargo run 127.0.0.1:5678 localhost end.chain
```