# DropShip

Rust-based peer-to-peer file transfer system.

## Requirements

- Rust stable toolchain, including `cargo`
- Git

Install Rust on macOS or Linux with [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Verify the installation:

```bash
rustc --version
cargo --version
```

## Setup

Clone the repository and enter the project directory:

```bash
git clone https://github.com/SharmisthaAlike/dropship.git
cd dropship
```

Build the project and run its tests:

```bash
cargo build
cargo test --all-features
```

## Run The TCP Handshake Demo

Start a receiver in one terminal:

```bash
cargo run -- listen --port 7878
```

Then connect from a second terminal:

```bash
printf 'hello' > hello.txt
cargo run -- send ./hello.txt 127.0.0.1:7878
```

The receiver listens on all interfaces at the selected port. The sender connects
to the address, sends a framed `HELLO` protocol message, and waits for the
receiver response. Press `Ctrl-C` to stop the listener.

To see the available commands and options:

```bash
cargo run -- --help
cargo run -- listen --help
cargo run -- send --help
```

## Current Scope

The current implementation provides asynchronous TCP connections, structured
logging, length-prefixed JSON protocol messages, and streaming file chunking with
SHA-256 hashes. The `send` path currently performs the initial handshake only;
file transfer, resumable storage, encryption, peer discovery, and parallel
transfers remain tracked in `PLAN.md`.
