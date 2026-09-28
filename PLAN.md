
# DropShip

> A peer-to-peer file transfer system built in Rust with encrypted, chunked, resumable, parallel transfers and LAN peer discovery.

## 0. Project Goal

Build a production-style P2P file transfer tool rather than a simple TCP file sender.

The final system should support:

- Direct peer-to-peer transfers
- LAN peer discovery
- Chunked file transfers
- Parallel chunk downloads
- Per-chunk integrity verification
- End-to-end encryption
- Resumable interrupted transfers
- Transfer progress
- Multiple simultaneous transfers
- Connection failure/recovery
- CLI-based operation
- Transfer benchmarking

Target usage:

```bash
dropship discover
dropship listen

dropship send ./movie.mp4 <peer>
dropship receive <transfer-id>
dropship status
```

---

# 1. Tech Stack

## Core

- Rust
- Tokio — asynchronous networking/runtime
- Serde — serialization
- SHA-256 — file/chunk integrity
- AEAD encryption — authenticated encryption
- Clap — CLI
- Tracing — structured logging

## Later

- LAN discovery
- Concurrent transfer workers
- Benchmarking
- Property-based testing
- Optional QUIC transport

### Initial transport

Start with **TCP**.

Do not introduce QUIC immediately. The first objective is to understand and implement the transfer protocol ourselves.

---

# 2. Architecture

Initial architecture:

```text
                  ┌─────────────────┐
                  │     CLI         │
                  └────────┬────────┘
                           │
                  ┌────────▼────────┐
                  │ Transfer Manager│
                  └────────┬────────┘
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
        Connection      Chunking       Crypto
          Manager         Engine        Layer
             │             │             │
             └─────────────┼─────────────┘
                           │
                    ┌──────▼──────┐
                    │ TCP / Tokio │
                    └──────┬──────┘
                           │
                      Remote Peer
```

Eventually:

```text
                         DropShip
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
         Discovery      Transfer        Storage
             │           Manager           │
             │              │               │
             ▼       ┌──────┼──────┐        ▼
            LAN      ▼      ▼      ▼     Manifest
                   Worker Worker Worker
                      │      │      │
                      └──────┼──────┘
                             │
                         TCP / QUIC
```

---

# 3. Milestone 1 — Rust + TCP Foundation

## Objective

Get two DropShip processes communicating.

### Sender

```bash
dropship send ./hello.txt 127.0.0.1:7878
```

### Receiver

```bash
dropship listen --port 7878
```

Expected flow:

```text
Sender                     Receiver

  │                           │
  │──── CONNECT ─────────────>│
  │                           │
  │──── HELLO ───────────────>│
  │                           │
  │──── FILE_INFO ───────────>│
  │                           │
  │──── FILE_DATA ───────────>│
  │                           │
  │──── TRANSFER_COMPLETE ───>│
  │                           │
```

### Tasks

- [x] Create Cargo project
- [x] Add Tokio
- [x] Create CLI with Clap
- [x] Implement TCP listener
- [x] Implement TCP client
- [x] Establish connection
- [x] Send/receive basic messages
- [x] Add structured logging

### Success criteria

Two local processes can establish a connection and exchange a message reliably.

---

# 4. Milestone 2 — Protocol Design

Do not send arbitrary strings once the basic connection works.

Create an explicit application protocol.

## Message types

```text
HELLO
FILE_INFO
CHUNK_REQUEST
CHUNK_DATA
CHUNK_ACK
TRANSFER_STATUS
TRANSFER_COMPLETE
TRANSFER_ERROR
```

Example:

```text
HELLO
{
    protocol_version,
    peer_id
}
```

```text
FILE_INFO
{
    transfer_id,
    filename,
    file_size,
    chunk_size,
    total_chunks,
    file_hash
}
```

```text
CHUNK_REQUEST
{
    transfer_id,
    chunk_id
}
```

```text
CHUNK_DATA
{
    transfer_id,
    chunk_id,
    chunk_hash,
    payload
}
```

## Tasks

- [x] Define protocol enums
- [x] Define message structures
- [x] Serialize with Serde
- [x] Implement message framing
- [x] Add protocol version
- [ ] Add transfer IDs
- [ ] Handle malformed messages
- [ ] Handle unsupported protocol versions

### Important

TCP is a byte stream, not a message protocol.

Implement proper framing rather than assuming one `read()` equals one message.

---

# 5. Milestone 3 — File Chunking

Split files into fixed-size chunks.

Initial target:

```text
1 MiB per chunk
```

Example:

```text
4.5 MiB file

chunk 0 → 1 MiB
chunk 1 → 1 MiB
chunk 2 → 1 MiB
chunk 3 → 1 MiB
chunk 4 → 0.5 MiB
```

## Tasks

- [ ] Implement chunk iterator
- [ ] Generate chunk IDs
- [ ] Calculate chunk hashes
- [ ] Calculate complete file hash
- [ ] Stream chunks rather than loading entire file
- [ ] Handle very large files

### Success criteria

A 10 GB file can be transferred without loading the entire file into memory.

---

# 6. Milestone 4 — Basic File Transfer

Implement:

```text
Sender
  │
  ├── FILE_INFO
  │
  ├── CHUNK_DATA 0
  ├── CHUNK_DATA 1
  ├── CHUNK_DATA 2
  ├── ...
  │
  └── TRANSFER_COMPLETE
```

Receiver writes chunks to disk.

## Receiver storage

Use a temporary transfer directory:

```text
~/.dropship/
└── transfers/
    └── <transfer-id>/
        ├── manifest.json
        ├── chunk-000000
        ├── chunk-000001
        └── ...
```

After successful verification:

```text
chunks
  ↓
reassemble
  ↓
final file
```

## Tasks

- [ ] Create transfer directory
- [ ] Write chunks to disk
- [ ] Maintain manifest
- [ ] Verify chunk hashes
- [ ] Reassemble file
- [ ] Verify final file hash
- [ ] Atomically rename completed file

---

# 7. Milestone 5 — Resumable Transfers

This is one of the core features.

If a transfer stops at:

```text
67%
```

DropShip should not start over.

The receiver maintains:

```json
{
    "transfer_id": "...",
    "file_size": 4287348271,
    "chunk_size": 1048576,
    "completed_chunks": [
        0,
        1,
        2,
        3,
        4
    ]
}
```

When reconnecting:

```text
Receiver → Sender

I already have:
0,1,2,3,4,8,9

Send:
5,6,7,10,...
```

## Tasks

- [ ] Persist transfer manifest
- [ ] Detect existing transfers
- [ ] Track completed chunks
- [ ] Validate existing chunks
- [ ] Request only missing chunks
- [ ] Resume after process restart
- [ ] Resume after connection loss

### Failure tests

Kill the sender at:

- [ ] 10%
- [ ] 50%
- [ ] 90%

Then restart and confirm the transfer resumes.

---

# 8. Milestone 6 — Parallel Transfers

Move from:

```text
chunk 1
   ↓
chunk 2
   ↓
chunk 3
   ↓
chunk 4
```

to:

```text
chunk 1 ──┐
chunk 2 ──┤
chunk 3 ──┼──→ Receiver
chunk 4 ──┤
chunk 5 ──┘
```

Use Tokio tasks/channels.

Initial configuration:

```text
4 concurrent chunks
```

Later benchmark:

```text
1
2
4
8
16
```

## Tasks

- [ ] Implement transfer worker pool
- [ ] Implement concurrent chunk requests
- [ ] Prevent duplicate chunk requests
- [ ] Handle out-of-order chunks
- [ ] Apply backpressure
- [ ] Tune concurrency

---

# 9. Milestone 7 — Encryption

Use an established authenticated-encryption library.

Do NOT implement cryptography ourselves.

Desired model:

```text
plaintext chunk
      ↓
authenticated encryption
      ↓
ciphertext
      ↓
network
      ↓
ciphertext
      ↓
decryption
      ↓
plaintext chunk
```

Encryption should provide:

- confidentiality
- integrity
- authentication

## Key exchange

Eventually establish a session key using a modern key-agreement mechanism.

Potential architecture:

```text
Peer A                     Peer B

  │──── public key ──────────>│
  │<─── public key ───────────│
  │                           │
  │    derive shared secret   │
  │                           │
  │──── encrypted transfer ──>│
```

## Tasks

- [ ] Establish peer identity
- [ ] Implement key exchange
- [ ] Derive session key
- [ ] Encrypt chunks
- [ ] Authenticate ciphertext
- [ ] Reject tampered chunks
- [ ] Rotate session keys between transfers if appropriate

---

# 10. Milestone 8 — LAN Peer Discovery

Remove the requirement to manually enter IP addresses.

Desired experience:

```bash
dropship discover
```

Output:

```text
Discovered peers:

1. MacBook-Air
   ID: 7e42...
   Address: 192.168.1.42:7878

2. MacBook-Pro
   ID: a18f...
   Address: 192.168.1.51:7878
```

Potential approach:

- UDP broadcast initially
- multicast/service discovery later

## Tasks

- [ ] Peer identity
- [ ] Discovery announcements
- [ ] Discovery listener
- [ ] Peer expiration
- [ ] Duplicate peer handling
- [ ] CLI discovery output

---

# 11. Milestone 9 — Connection Recovery

Make network failure a normal event.

Scenarios:

```text
Wi-Fi drops
   ↓
TCP connection dies
   ↓
detect timeout
   ↓
persist state
   ↓
retry connection
   ↓
resume missing chunks
```

Implement:

- connection timeout
- read/write timeout
- retry policy
- exponential backoff
- dead-peer detection
- transfer recovery

Do not blindly retry forever.

---

# 12. Milestone 10 — Transfer UX

CLI should feel like an actual tool.

Example:

```bash
dropship send movie.mp4 sharmistha-mac
```

Output:

```text
Sending movie.mp4

████████████████░░░░  78%

Transferred: 3.42 GB / 4.38 GB
Speed:       84.2 MB/s
ETA:         11s
Chunks:      3264 / 4182
Peers:       1
```

Receiver:

```text
Receiving movie.mp4

████████████████░░░░  78%

Speed: 84.2 MB/s
ETA:   11s
```

---

# 13. Milestone 11 — Multiple Peers

Eventually allow:

```text
             ┌──────────┐
             │ Sender A │
             └────┬─────┘
                  │
          ┌───────┼────────┐
          ▼       ▼        ▼
        Peer 1  Peer 2   Peer 3
```

Potential future feature:

A receiver can obtain different chunks from different peers.

```text
Peer 1 → chunks 0–999
Peer 2 → chunks 1000–1999
Peer 3 → chunks 2000–2999
```

This moves DropShip toward a genuinely distributed file-transfer system.

---

# 14. Testing Strategy

Do not rely only on manually transferring files.

## Unit tests

Test:

- chunk boundaries
- hashing
- manifest handling
- protocol encoding
- protocol decoding
- encryption/decryption
- missing chunk detection

## Integration tests

Test:

```text
sender ↔ receiver
```

with:

- tiny files
- empty files
- binary files
- large files
- filenames with Unicode
- interrupted transfers

## Failure tests

Intentionally:

- kill sender
- kill receiver
- close socket
- corrupt chunk
- delete chunk
- modify manifest
- restart process
- interrupt transfer repeatedly

---

# 15. Benchmarking

Measure:

### Throughput

```text
MB/s
```

### Latency

```text
connection setup
chunk request latency
```

### CPU

```text
sender CPU
receiver CPU
```

### Memory

```text
RSS
peak memory
```

### Recovery

```text
time to resume
```

### Concurrency

Compare:

```text
1 worker
2 workers
4 workers
8 workers
16 workers
```

Create a benchmark table:

```text
Workers | Throughput | CPU | Memory
--------|------------|-----|-------
1       |            |     |
2       |            |     |
4       |            |     |
8       |            |     |
16      |            |     |
```

---

# 16. Security Requirements

Threat model at minimum:

### Attacker can

- observe network traffic
- modify packets
- replay messages
- send malformed protocol messages
- impersonate an unknown peer
- corrupt transferred chunks

### Protect against

- plaintext file exposure
- tampered chunks
- malformed messages
- path traversal
- arbitrary file overwrite
- unlimited resource consumption

Never trust:

```text
filename
file size
chunk ID
peer ID
manifest
network input
```

Validate everything.

Especially prevent:

```text
../../some-important-file
```

from becoming a destination path.

---

# 17. Project Structure

Target structure:

```text
dropship/
│
├── Cargo.toml
├── Cargo.lock
├── README.md
├── BUILD.md
├── LICENSE
│
├── src/
│   ├── main.rs
│   ├── cli.rs
│   │
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── messages.rs
│   │   └── framing.rs
│   │
│   ├── network/
│   │   ├── mod.rs
│   │   ├── connection.rs
│   │   └── discovery.rs
│   │
│   ├── transfer/
│   │   ├── mod.rs
│   │   ├── sender.rs
│   │   ├── receiver.rs
│   │   ├── chunk.rs
│   │   └── manifest.rs
│   │
│   ├── crypto/
│   │   ├── mod.rs
│   │   ├── keys.rs
│   │   └── encryption.rs
│   │
│   └── error.rs
│
├── tests/
│   ├── protocol.rs
│   ├── transfer.rs
│   ├── resume.rs
│   └── failure.rs
│
└── benches/
    └── transfer.rs
```

---

# 18. Final target architecture:

```text
                         DropShip
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
          Discovery      Protocol      Transfer
              │             │             │
              ▼             ▼             ▼
             LAN          Framing       Chunking
                            │             │
                            ▼             ▼
                         Transport    Parallelism
                            │             │
                       ┌────┴────┐        ▼
                       ▼         ▼     Resume
                      TCP       QUIC      │
                       │                  ▼
                       └────────────── Encryption
                                           │
                                           ▼
                                      Verification
```