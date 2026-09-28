# DropShip

Rust-based peer-to-peer file transfer system.

## Development

Install the stable Rust toolchain, then run:

```bash
cargo test
cargo run -- listen --port 7878
```

In another terminal, perform the initial TCP handshake:

```bash
cargo run -- send ./hello.txt 127.0.0.1:7878
```

The current skeleton implements asynchronous TCP connections, structured logging,
and length-prefixed JSON protocol messages. File transfer, chunking, encryption,
resume state, and discovery are tracked in `PLAN.md`.
