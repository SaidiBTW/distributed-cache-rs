# Distributed State Cache

A high-performance, distributed state cache built from the ground up in Rust, focusing on low-level primitives and minimal abstractions. This project aims to implement complex distributed systems concepts using only the Rust standard library (`std`) to provide a clear understanding of the underlying mechanics.

## Project Tags

`Distributed Systems` `Networking` `State Reconciliation` `Rust` `Invertible Bloom Lookup Tables` `Caching`

---

## Vision

The goal of this project is to build a robust distributed cache that transitions from a simple concurrent key-value store to a fully consistent, distributed system with efficient state reconciliation. By avoiding high-level frameworks, we maintain complete control over memory allocation, network protocols, and consensus logic.

## Possible Improvements at the Moment

- **Implementing using DashMap**: At the moment the cache is locked behind a single `RwLock`. If readers do not pause or execute indefinetly, this means that writes will suffer from starvation. `DashMap` should solve this, as it implements mutliple `RwLocks`.
- **Implementing a trait based implementation of return protocols**: Consider using a Respond trait and a response enum with all possible return types.
- **Thread Management**: We should implement thread-pooling to reduce the risk of creating multiple OS-level threads that could consume memory when they scale to the thousands. We could also use tokio although it is not considered an `std` library hence falls short of the requirements of the project.
- **Graceful Shutdown**
- **Error Handling** - Reducing the use of `unwrap()` to prevent preventable panicking.

## Roadmap & Progress

### 🟢 Phase 1: Baseline (Current)

- **Concurrent Key-Value Store**: Built using `HashMap`, `Arc`, and `RwLock`.
- **TCP Server**: A multi-threaded TCP server using `std::net::TcpListener` and `std::thread`.
- **Basic Protocol**: A binary protocol for `GET` and `SET` operations with length-prefixed keys and values.

### 🟡 Phase 2: Custom Memory Arena

- **Efficient Allocation**: Replacing standard `HashMap` allocations with a custom memory arena.
- **Buffer Management**: Using `Vec<u8>` as a pre-allocated buffer to reduce fragmentation and improve cache locality.
- **Manual Lifetime Management**: Implementing low-level memory layout for cached entries.

### 🔴 Phase 3: Consensus (Raft)

- **Distributed Protocol**: Implementing the Raft Consensus protocol from scratch.
- **Leader Election**: Handling node failures and ensuring a single leader.
- **Log Replication**: Ensuring all servers reach a consistent state before acknowledging writes.

### 🔴 Phase 4: State Reconciliation & Persistence

- **IBLTs (Invertible Bloom Lookup Tables)**: Implementing IBLTs for efficient Write-Ahead Log (WAL) reconciliation between nodes.
- **Persistence Layer**: Adding a disk-backed storage engine using `std::fs`.
- **Recovery**: Restoring state from the WAL after a crash or restart.

### 🔴 Phase 5: Zero-Copy Serialization

- **Performance Optimization**: Implementing a zero-copy binary protocol to minimize data copying between the network buffer and the cache.
- **Custom Deserialization**: Using `unsafe` (where necessary and safe) to map byte buffers directly to internal structures.

### 🔴 Phase 6: Observability & Diagnostics

- **Metrics**: Implementing hit/miss ratios, latency tracking, and memory usage statistics using standard primitives.
- **Diagnostic API**: Exposing a secondary TCP port for health checks and cluster status.

---

## Getting Started

### Prerequisites

- Rust (Latest Stable)

### Running the Server

Currently, the baseline implementation is located in the `cache` directory.

```bash
cargo run --bin cache
```

The server listens on `127.0.0.1:7878` by default.

### Running the Test Client

cargo run --bin client

```

### Protocol Specification (Phase 1)

The server uses a binary protocol over TCP. All multi-byte integers are encoded in **Big-Endian**.

#### Request Format
| Offset | Field | Size | Description |
|--------|-------|------|-------------|
| 0 | Command | 1 byte | `1` for GET, `2` for SET |
| 1 | Key Length | 4 bytes | Length of the key in bytes (u32) |
| 5 | Key | N bytes | Raw bytes of the key |
| 5+N | Value Length* | 4 bytes | Length of the value in bytes (only for SET) |
| 9+N | Value* | M bytes | Raw bytes of the value (only for SET) |

#### Response Format
| Offset | Field | Size | Description |
|--------|-------|------|-------------|
| 0 | Status | 1 byte | `0x00` (Ok), `0x01` (NotFound), `0xFF` (Err) |
| 1 | Body Length | 4 bytes | Length of the body in bytes (u32) |
| 5 | Body | L bytes | Raw bytes of the response body |

#### Status Codes
- `0x00 (Ok)`: Operation successful. For `GET`, body contains the value. For `SET`, body is empty.
- `0x01 (NotFound)`: Key does not exist in the cache.
- `0xFF (Err)`: A server-side or protocol error occurred.

## Philosophy

- **No Dependencies**: Where possible, only `std` is used.
- **Explicit over Implicit**: No magic macros or heavy abstractions.
- **Performance First**: Prioritizing memory efficiency and low-latency networking.
```

- **Performance First**: Prioritizing memory efficiency and low-latency networking.
