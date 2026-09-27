# Solizone EVM

**A stateful, restartable EVM execution environment for Logos.**

Solizone executes Solidity/EVM transactions with REVM, maintains recoverable state, produces parent-linked canonical blocks, publishes those blocks to Logos for ordering/finality, and is now experimenting with Logos Storage for checkpoint and historical block durability.

> **Status:** active research prototype on `feat/second-phase`  
> **Note:** Solizone is an independent project and is not an official Logos project.

---

## Why Solizone?

```text
Ethereum developer experience
        ↓
Solizone EVM execution
        ↓
canonical Solizone blocks
        ↓
Logos
```

Solizone owns EVM execution and state.

Logos provides the shared infrastructure around it:

```text
Logos blockchain
→ publication
→ ordering
→ consensus
→ finality

Logos Storage
→ durable/shared content storage
→ checkpoint and historical-block experiments
```

---

## What works today

### EVM execution

- generic REVM transaction execution
- Solidity contract deployment and calls
- mutable contract storage and read-only calls
- persistent balances, nonces, bytecode, and storage
- ordered transaction-batch execution
- receipts and deterministic transaction/receipt/state commitments

### State and recovery

- deterministic `StateSnapshot`
- snapshot → fresh REVM reconstruction
- JSON and file-backed persistence
- `SolizoneCheckpoint`
- checkpoint contains EVM state + `next_height` + `parent_hash`
- complete process restart and continued execution
- protocol state-root continuity after restore

### Canonical chain

- canonical `SZB1` block format
- parent-linked block production
- `BlockProducer`
- producer resume after restart
- `BlockStore`
- `MemoryBlockStore`
- `FileBlockStore`
- lookup by height or hash
- durable local canonical history

### Logos publication

- generic publisher boundary
- Logos publisher adapter
- `ZoneSequencer` publication
- canonical Solizone bytes published to Logos
- Logos block inclusion observed
- LIB / finality observed
- sequencer checkpoint and resume flow

### Logos Storage experiment

The first network-level Storage roundtrip is now working:

```text
local file
   ↓
logosctl
   ↓
storage_module v2.1.2
   ↓
Logos Storage
   ↓
CID
   ↓
download by CID
   ↓
original bytes recovered
```

Proven so far:

- Storage module installed and started through the Logos runtime
- node connected to the `logos.test` network
- local file uploaded successfully
- CID returned
- file downloaded successfully by CID
- downloaded content matched the original

The actual Solizone checkpoint/block Storage backend is **not implemented yet**.

---

## Current architecture

```text
Solidity / Solizone tx
        ↓
       REVM
        ↓
   MemoryState
        ↓
receipts + state_root
        ↓
  BlockProducer
        ↓
canonical Solizone block
        │
        ├──────────── local recovery ────────────┐
        │                                         │
        ↓                                         ↓
SolizoneCheckpoint                           BlockStore
        ↓                                         ↓
CheckpointBackend                         FileBlockStore
        ↓                                   recent .szb
FileCheckpointBackend
        ↓
checkpoint.json

canonical block
        ↓
BlockPublisher
        ↓
LogosPublisher
        ↓
ZoneSequencer
        ↓
Logos blockchain
        ↓
ordering / inclusion / finality
```

Persistence direction:

```text
CheckpointBackend
├── FileCheckpointBackend
└── Logos Storage adapter       ← next

Block history
├── FileBlockStore              → recent local cache
└── Logos Storage               → older shared history
```

---

## Recovery model we are targeting

A normal Solizone node should **not** need to keep the entire chain forever.

```text
current head = 1,000,000

local node keeps:
├── current reconstructed EVM state
├── latest checkpoint
├── recent block window
└── small head / CID metadata

Logos Storage keeps:
└── older checkpoint and block history
```

When a node rejoins:

```text
load latest checkpoint
        ↓
discover canonical head
        ↓
fetch missing blocks
        ↓
verify parent linkage
        ↓
re-execute missing blocks
        ↓
reach current state
        ↓
keep only the recent window locally
```

Fetched historical blocks do not need to remain on disk forever.

A remaining design problem is **canonical discovery**:

```text
latest checkpoint → which CID?
current head       → which block?
height N           → which CID?
```

Logos Storage is content-addressed, so Solizone still needs a reliable head/index mechanism.

---

## Important distinctions

### Protocol state root

```text
execution state
→ deterministic Solizone state_root
→ committed into block header
```

Used for execution/block correctness.

### Snapshot commitment

```text
serialized deterministic snapshot
→ Keccak256
```

Used as a persistence fingerprint.

It does **not** replace the protocol state root.

### Logos blockchain vs Logos Storage

```text
Logos blockchain
→ canonical publication
→ ordering
→ consensus
→ finality

Logos Storage
→ content storage
→ retrieval by CID
→ checkpoint / history durability experiment
```

Storage does not replace blockchain finality.

---

## Current checkpoint boundary

```rust
pub trait CheckpointBackend {
    type Error;

    fn save(
        &self,
        checkpoint: &SolizoneCheckpoint,
    ) -> Result<(), Self::Error>;

    fn load(
        &self,
    ) -> Result<Option<SolizoneCheckpoint>, Self::Error>;
}
```

Today:

```text
CheckpointBackend
└── FileCheckpointBackend
```

Next:

```text
CheckpointBackend
├── FileCheckpointBackend
└── LogosStorageCheckpointBackend
```

---

## Current block storage

```text
BlockStore
├── MemoryBlockStore
└── FileBlockStore
```

`FileBlockStore` persists canonical `.szb` files by height.

This is correct for local recovery, but keeping every block forever would create unbounded disk growth. The intended direction is:

```text
recent blocks → local FileBlockStore
older blocks  → Logos Storage
```

---

## Test status

Current Rust suite:

```text
29 passed
0 failed
```

Coverage includes value transfer, Solidity deployment/calls, mutable state, receipts, deterministic roots, snapshots, checkpoint persistence, parent-linked blocks, producer resume, block storage, block lookup, canonical replacement, and full state + producer + history restart.

---

## Current limitations

Not implemented yet:

- raw signed Ethereum transactions
- EIP-2718 / RLP decoding
- secp256k1 sender recovery
- Ethereum fee handling
- transaction pool
- Ethereum JSON-RPC
- long-running production node
- full imported-block re-execution validation
- Ethereum MPT state roots
- atomic checkpoint + block commits
- persistent block hash index
- production publisher service
- Logos Storage checkpoint adapter
- Logos Storage historical-block adapter
- canonical head / height → CID discovery
- explicit Storage replication policy

Logos Storage has been proven at the runtime/network level, but it is **not yet wired into Solizone persistence**.

---

## Next development steps

### 1. Store a real `SolizoneCheckpoint` in Logos Storage

```text
SolizoneCheckpoint
        ↓
encode_json()
        ↓
checkpoint file
        ↓
storage_module
        ↓
CID
        ↓
download
        ↓
decode_json()
        ↓
restore EVM state
```

### 2. Implement a Logos Storage checkpoint adapter

```text
CheckpointBackend
├── FileCheckpointBackend
└── LogosStorageCheckpointBackend
```

### 3. Archive canonical block history

```text
recent .szb blocks
→ local FileBlockStore

older .szb blocks
→ Logos Storage
```

### 4. Design canonical discovery

```text
latest checkpoint CID
latest canonical head
height → block hash → CID
```

### 5. Continue toward an EVM developer surface

```text
Foundry / MetaMask / ethers / viem
        ↓
Ethereum JSON-RPC
        ↓
raw signed transactions
        ↓
transaction pool
        ↓
BlockProducer
        ↓
REVM
        ↓
Solizone state
        ↓
canonical block
        ↓
Logos
```

---

## Repository structure

```text
solizone-evm/
├── contracts/
├── contracts-out/
├── src/
│   ├── block.rs
│   ├── block_builder.rs
│   ├── block_producer.rs
│   ├── block_store.rs
│   ├── memory_block_store.rs
│   ├── file_block_store.rs
│   ├── publisher.rs
│   ├── logos_publisher.rs
│   ├── bin/
│   │   ├── publish.rs
│   │   ├── state_writer.rs
│   │   └── state_reader.rs
│   ├── state/
│   │   ├── memory.rs
│   │   ├── snapshot.rs
│   │   ├── commitment.rs
│   │   ├── backend.rs
│   │   ├── checkpoint.rs
│   │   ├── checkpoint_backend.rs
│   │   ├── file_backend.rs
│   │   └── file_checkpoint_backend.rs
│   └── execution/
│       ├── revm_engine.rs
│       ├── transaction.rs
│       ├── receipt.rs
│       ├── state_root.rs
│       ├── transactions_root.rs
│       └── receipts_root.rs
└── README.md
```

---

## Useful commands

```bash
cargo fmt
cargo check
cargo test

cargo test resumes_full_node_from_disk_checkpoint -- --nocapture

cargo run --bin publish
cargo run --bin publish -- --send
RUST_LOG=warn cargo run --bin publish -- --resume
```

### Logos Storage experiment

```bash
logosctl module load storage_module

logosctl call storage_module init "$(cat storage-config.json)"

logosctl call storage_module start

logosctl call storage_module peerId

logosctl call storage_module manifests
```

---

## Design rule

Keep the boundaries explicit:

```text
execution
   ↓
state / persistence
   ↓
block production
   ↓
canonical history
   ↓
publication
```

REVM should not know how Logos publication works.

REVM should not depend directly on Logos Storage.

Storage and blockchain finality are different layers.

That separation lets Solizone evolve without coupling execution to one persistence or publication mechanism.

---

**Solizone is an independent research prototype and is not an official Logos project.**
