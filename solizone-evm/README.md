# Solizone EVM

**A stateful, restartable EVM execution environment for Logos.**

Solizone executes Solidity / EVM transactions with REVM, maintains recoverable state, produces parent-linked canonical blocks, publishes those blocks through the Logos Zone SDK, and now uses Logos Storage for execution checkpoints and historical block persistence.

> **Status:** active research implementation on `feat/second-phase`  
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

The Logos stack provides shared infrastructure around it:

```text
Logos blockchain
→ publication
→ ordering
→ consensus
→ finality

Logos Storage
→ execution checkpoint persistence
→ historical canonical block storage
→ content-addressed retrieval
```

The long-term goal is to give Ethereum developers a familiar Solidity / EVM environment while remaining native to the Logos architecture.

---

## What works today

### EVM execution

- generic REVM transaction execution
- Solidity contract deployment and calls
- mutable contract storage and read-only calls
- persistent balances, nonces, bytecode, and storage
- ordered transaction-batch execution
- receipts and deterministic transaction / receipt / state commitments

### State and recovery

- deterministic `StateSnapshot`
- snapshot → fresh REVM reconstruction
- JSON and file-backed persistence
- `SolizoneCheckpoint`
- checkpoint contains EVM state + `next_height` + `parent_hash`
- `FileCheckpointBackend`
- `LogosStorageCheckpointBackend`
- full process restart and continued execution
- protocol state-root continuity after restore
- remote checkpoint recovery through Logos Storage

### Canonical chain

- canonical `SZB1` block format
- parent-linked block production
- `BlockProducer`
- producer resume after restart
- `BlockStore`
- `MemoryBlockStore`
- `FileBlockStore`
- `HybridBlockStore`
- lookup by height or hash
- recent canonical history retained locally
- historical canonical blocks archived into Logos Storage
- automatic pruning with a configurable retention window
- recovered historical blocks decoded, validated, and hash-checked

### Long-running node runtime

- continuous block production
- checkpoint persistence after completed blocks
- automatic local retention enforcement
- automatic archival of older blocks to Logos Storage
- automatic startup recovery from the latest checkpoint
- recovered chain-head verification
- continued block production from the recovered height

### Logos publication

- generic publisher boundary
- Logos publisher adapter
- `ZoneSequencer` publication
- canonical Solizone bytes published to Logos
- Logos block inclusion observed
- LIB / finality observed
- sequencer checkpoint and resume flow

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
        ├──────────────── persistence ────────────────┐
        │                                             │
        ↓                                             ↓
SolizoneCheckpoint                             HybridBlockStore
        │                                      │             │
        ↓                                      ↓             ↓
CheckpointBackend                        recent blocks    older blocks
        │                                  local          Logos Storage
        ├── FileCheckpointBackend                             │
        └── LogosStorageCheckpointBackend                    ↓
                                                    height → hash → CID

canonical Solizone block
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

The execution, persistence, storage, and publication boundaries are intentionally separate.

---

## Hybrid Storage Layer

Solizone now includes a hybrid storage model for canonical block history.

```text
BlockStore
├── MemoryBlockStore
├── FileBlockStore
└── HybridBlockStore
      ├── recent blocks → local FileBlockStore
      └── older blocks  → Logos Storage
                          via height → hash → CID
```

A node can keep only a recent local block window while moving older canonical history into Logos Storage.

Example with a retention window of `3`:

```text
canonical history:
0 1 2 3 4 5 6

local:
4 5 6

Logos Storage:
0 1 2 3
```

When the local window is exceeded:

```text
insert new block
      ↓
persist locally
      ↓
retention exceeded?
      ↓
archive oldest block
      ↓
upload to Logos Storage
      ↓
persist height + hash + CID
      ↓
prune local .szb
```

The archive index is persisted before the local canonical block is removed.

A caller does not need to know where the block physically lives:

```rust
store.get_by_height(8)?;
store.get_by_height(2)?;
```

`HybridBlockStore` checks local storage first and falls back to Logos Storage when the block is archived.

Recovered blocks are:

```text
downloaded
    ↓
decoded
    ↓
validated
    ↓
hash-checked against canonical metadata
    ↓
returned
```

---

## Logos Storage integration

The Storage integration is now part of Solizone persistence rather than a standalone runtime experiment.

### Checkpoints

```text
SolizoneCheckpoint
        ↓
encode
        ↓
LogosStorageCheckpointBackend
        ↓
Logos Storage
        ↓
CID
```

The latest checkpoint CID is recorded locally as discovery metadata.

On restart:

```text
checkpoint CID
      ↓
Logos Storage
      ↓
download checkpoint
      ↓
restore EVM state
      ↓
restore next_height
      ↓
restore parent_hash
```

### Historical blocks

Canonical `.szb` blocks can be archived individually.

```text
canonical .szb
      ↓
Logos Storage
      ↓
CID
      ↓
archive index
height → block hash → CID
```

The Storage layer stores and retrieves content. It does **not** decide which Solizone block is canonical.

---

## Long-running node runtime

Run:

```bash
cargo run --bin solizone_node
```

The runtime continuously owns:

```text
MemoryState
+
BlockProducer
+
HybridBlockStore
+
LogosStorageCheckpointBackend
```

Runtime flow:

```text
execute transaction
      ↓
produce block
      ↓
insert into HybridBlockStore
      ↓
archive / prune old history if needed
      ↓
snapshot EVM state
      ↓
persist checkpoint to Logos Storage
      ↓
continue
```

If no checkpoint exists, the node starts fresh.

If a previous checkpoint exists:

```text
existing checkpoint detected
        ↓
download from Logos Storage
        ↓
restore EVM state
        ↓
resume BlockProducer
        ↓
verify recovered chain head
        ↓
continue block production
```

A full restart path has been verified with the node stopping at one height and continuing the same parent-linked chain from the next height after recovery.

---

## Recovery model

A normal Solizone node does not need to keep the entire chain locally forever.

```text
local node keeps:
├── current reconstructed EVM state
├── latest checkpoint metadata
├── recent block window
└── archive / head metadata

Logos Storage keeps:
├── execution checkpoints
└── older canonical block history
```

After restart:

```text
load latest checkpoint
        ↓
restore state + chain head
        ↓
verify local head
        ↓
continue execution
```

For historical lookups:

```text
request block N
      ↓
check local store
      ↓
not found
      ↓
archive index
      ↓
CID
      ↓
Logos Storage
      ↓
validate + hash-check
```

Longer-term synchronization still requires a network-level canonical discovery mechanism for:

```text
latest checkpoint CID
current canonical head
height → block hash → CID
```

The current implementation uses local metadata for that discovery boundary.

---

## Important distinctions

### Protocol state root

```text
execution state
→ deterministic Solizone state_root
→ committed into block header
```

Used for execution and block correctness.

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
→ checkpoint persistence
→ historical block persistence
```

Storage does not replace blockchain finality.

---

## Checkpoint boundary

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

Implemented backends:

```text
CheckpointBackend
├── FileCheckpointBackend
└── LogosStorageCheckpointBackend
```

This keeps REVM independent from the persistence mechanism.

---

## Block storage boundary

```text
BlockStore
├── MemoryBlockStore
├── FileBlockStore
└── HybridBlockStore
```

`HybridBlockStore` adds:

- automatic local retention
- Logos Storage archival
- archive index lookup
- transparent local / remote reads
- historical block validation
- canonical hash verification

---

## Current limitations

Not implemented yet:

- raw signed Ethereum transactions
- EIP-2718 / RLP decoding
- secp256k1 sender recovery
- Ethereum fee handling
- transaction pool
- Ethereum JSON-RPC
- full imported-block re-execution validation
- Ethereum MPT state roots
- network-distributed canonical head / CID discovery
- explicit Storage replication policy
- atomic crash-safe checkpoint + block commit protocol
- production-grade graceful shutdown
- production publisher service

These are intentionally deferred to the next development phase.

---

## Next development steps

### 1. Raw Ethereum transaction ingestion

```text
signed Ethereum transaction
        ↓
decode transaction envelope
        ↓
recover sender
        ↓
admission validation
        ↓
Solizone transaction
```

### 2. Transaction pool

```text
incoming transactions
        ↓
Solizone mempool
        ↓
BlockProducer
        ↓
REVM
```

### 3. Ethereum-compatible JSON-RPC

```text
Foundry / MetaMask / ethers / viem
        ↓
Ethereum JSON-RPC
        ↓
Solizone node
```

### 4. Independent block validation

A second execution path should be able to consume canonical blocks, re-execute transactions, and independently verify resulting commitments.

### 5. Production hardening

- crash-safe persistence
- graceful shutdown
- archive-index durability
- distributed CID / head discovery
- replication strategy
- durable publisher service

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
│   ├── hybrid_block_store.rs
│   ├── logos_storage_client.rs
│   ├── publisher.rs
│   ├── logos_publisher.rs
│   ├── bin/
│   │   ├── solizone_node.rs
│   │   ├── publish.rs
│   │   ├── hybrid_node_restart.rs
│   │   └── ...
│   ├── state/
│   │   ├── memory.rs
│   │   ├── snapshot.rs
│   │   ├── commitment.rs
│   │   ├── backend.rs
│   │   ├── checkpoint.rs
│   │   ├── checkpoint_backend.rs
│   │   ├── file_backend.rs
│   │   ├── file_checkpoint_backend.rs
│   │   └── logos_storage_checkpoint_backend.rs
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

### Rust

```bash
cargo fmt
cargo check
cargo test
```

Run the long-running Solizone node:

```bash
cargo run --bin solizone_node
```

Reset the local runtime state:

```bash
cargo run --bin solizone_node -- reset
```

Run the Logos publication harness:

```bash
cargo run --bin publish
```

### Logos Storage

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

That separation lets Solizone evolve without coupling execution to a single persistence or publication mechanism.

---

**Solizone is an independent research implementation and is not an official Logos project.**
