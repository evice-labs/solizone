# solizone-evm

**The active Rust implementation of the Solizone EVM execution layer, persistent local chain state, and current Logos publication path.**

`solizone-evm` executes Solidity/EVM transactions through REVM, maintains recoverable EVM state, produces parent-linked canonical Solizone blocks, stores canonical block history locally, publishes canonical block bytes through the Logos Zone SDK, and can observe publication reaching Logos finality.

The current `feat/second-phase` branch has moved beyond the original single-execution proof into a **stateful, restartable, block-producing execution prototype**.

> ⚠️ **Disclaimer:** Solizone is an independent research prototype and is **not an official Logos project**.

---

## Table of contents

- [What works today](#what-works-today)
- [Status](#status)
- [Current architecture](#current-architecture)
- [Execution and state](#execution-and-state)
- [Persistent state and recovery](#persistent-state-and-recovery)
- [Block production and canonical history](#block-production-and-canonical-history)
- [Canonical Solizone block](#canonical-solizone-block)
- [Transaction model](#transaction-model)
- [Receipts](#receipts)
- [State commitments](#state-commitments)
- [Logos blockchain publication](#logos-blockchain-publication)
- [Logos publication checkpoint](#logos-publication-checkpoint)
- [Logos Storage direction](#logos-storage-direction)
- [Repository structure](#repository-structure)
- [Module responsibilities](#module-responsibilities)
- [Build, run, test](#build-run-test)
- [Publisher CLI](#publisher-cli)
- [Current limitations](#current-limitations)
- [Development principles](#development-principles)
- [Roadmap](#roadmap)
- [Useful commands](#useful-commands)
- [Related documentation](#related-documentation)

---

## What works today

The current prototype can:

- execute generic EVM transactions through REVM
- deploy and call compiled Solidity contracts
- mutate and read contract storage
- preserve accounts, balances, nonces, bytecode, and storage across transactions
- execute ordered transaction batches as a block
- produce execution receipts
- compute deterministic state, transaction, and receipt commitments
- build canonical `SZB1` Solizone blocks
- produce parent-linked multi-block chains
- persist deterministic EVM-state snapshots
- restore a fresh REVM state from persisted snapshots
- persist a combined execution checkpoint containing state + next block height + parent hash
- resume block production from a persisted chain head
- store canonical block history in memory or on disk
- retrieve persisted blocks by height or hash
- recover EVM state + block producer + canonical block history after a complete process restart
- continue execution with the next correctly linked block after restart
- publish an execution-backed canonical Solizone block through the Logos Zone SDK
- persist and resume Logos `ZoneSequencer` publication state
- observe a published Solizone block reaching Logos finality

At the latest second-phase milestone, the full Rust test suite reports:

```text
29 passed
0 failed
```

---

## Status

### Implemented and proven

| Area | Capability | Status |
| --- | --- | --- |
| Execution | REVM integration | ✅ |
| Execution | Generic EVM value transfer | ✅ |
| Execution | Solidity contract deployment | ✅ |
| Execution | Contract calls | ✅ |
| Execution | Contract storage mutation | ✅ |
| Execution | Read-only contract execution | ✅ |
| Execution | Ordered transaction-batch execution | ✅ |
| State | Shared in-memory EVM state | ✅ |
| State | Deterministic state snapshot | ✅ |
| State | Snapshot → fresh REVM reconstruction | ✅ |
| State | JSON snapshot round trip | ✅ |
| State | File-backed snapshot persistence | ✅ |
| State | State-root continuity after restore | ✅ |
| Recovery | `SolizoneCheckpoint` | ✅ |
| Recovery | File-backed checkpoint save/load | ✅ |
| Recovery | Full EVM-state restart recovery | ✅ |
| Recovery | Continue execution after restart | ✅ |
| Transactions | Canonical Solizone transaction encoding | ✅ |
| Transactions | Transaction hashing | ✅ |
| Transactions | Transactions root | ✅ |
| Receipts | Success / revert / halt receipts | ✅ |
| Receipts | Canonical receipt encoding | ✅ |
| Receipts | Receipts root | ✅ |
| Commitments | Deterministic Solizone protocol state root | ✅ |
| Commitments | Deterministic snapshot commitment | ✅ |
| Block | Canonical Solizone block format | ✅ |
| Block | Canonical encoding / decoding | ✅ |
| Block | Transaction-root validation | ✅ |
| Block | Block builder | ✅ |
| Chain | `BlockProducer` abstraction | ✅ |
| Chain | Parent-linked multi-block production | ✅ |
| Chain | Producer resume from chain head | ✅ |
| Chain | `BlockStore` abstraction | ✅ |
| Chain | `MemoryBlockStore` | ✅ |
| Chain | `FileBlockStore` | ✅ |
| Chain | Block lookup by height | ✅ |
| Chain | Block lookup by hash | ✅ |
| Chain | Canonical replacement semantics | ✅ |
| Chain | Durable block-history recovery | ✅ |
| Integration | Full restart: state + producer + block history | ✅ |
| Publication | Generic publisher abstraction | ✅ |
| Publication | Logos publisher adapter | ✅ |
| Publication | Logos Zone SDK connection | ✅ |
| Publication | Sequencer checkpoint persistence | ✅ |
| Publication | Sequencer resume / reconciliation | ✅ |
| Finality | Execution-backed block publication to Logos | ✅ |
| Finality | Logos block inclusion | ✅ |
| Finality | Logos LIB / finality proof | ✅ |

### Not implemented yet

| Area | Capability | Status |
| --- | --- | --- |
| Storage | Logos Storage backend | ⏳ |
| Ethereum tx layer | Raw signed Ethereum transactions | ⏳ |
| Ethereum tx layer | EIP-2718 / RLP transaction decoding | ⏳ |
| Ethereum tx layer | secp256k1 signature recovery | ⏳ |
| Ethereum tx layer | Ethereum fee handling | ⏳ |
| Ethereum tx layer | Transaction admission rules | ⏳ |
| Node surface | Ethereum JSON-RPC | ⏳ |
| Node surface | Transaction pool | ⏳ |
| Node runtime | Long-running block-production service | ⏳ |
| Validation | Full state re-execution validation | ⏳ |
| Validation | Full receipt re-execution validation | ⏳ |
| Compatibility | Ethereum MPT state roots | ⏳ |
| Durability | Atomic checkpoint + block commit | ⏳ |
| Chain indexing | Persistent hash index | ⏳ |
| Operations | Production publisher service | ⏳ |

### What the current milestone proves

Solizone can now:

```text
execute EVM state transitions
        ↓
advance deterministic state
        ↓
produce parent-linked canonical blocks
        ↓
persist execution state + chain head
        ↓
persist canonical block history
        ↓
stop completely
        ↓
restore state + producer + history
        ↓
continue with the next correctly linked block
```

Separately, the existing Logos publication path has already proven:

```text
execution-backed Solizone block
        ↓
canonical SZB1 bytes
        ↓
Logos inscription
        ↓
Logos block inclusion
        ↓
LIB / finality
```

---

## Current architecture

The implementation now has three intentionally separated responsibilities:

1. **EVM execution**
2. **Solizone persistence / chain history**
3. **Logos publication / finality**

```text
Solidity / Solizone transaction
        ↓
REVM
        ↓
MemoryState
(accounts / balances / nonces / code / storage)
        ↓
receipts + final state
        ↓
state_root / transactions_root / receipts_root
        ↓
BlockProducer
        ↓
parent-linked canonical Solizone block
        │
        ├──────────── local persistence ─────────────┐
        │                                             │
        ↓                                             ↓
SolizoneCheckpoint                               BlockStore
state + next_height + parent_hash               canonical blocks
        │                                             │
        ↓                                             ↓
FileCheckpointBackend                          FileBlockStore
checkpoint.json                                0.szb / 1.szb / 2.szb
        │                                             │
        └──────────── restart recovery ───────────────┘
                              ↓
                       continue block N+1

canonical Solizone block
        ↓
BlockPublisher
        ↓
LogosPublisher
        ↓
Logos ZoneSequencer
        ↓
Mantle channel inscription
        ↓
Logos ordering / block inclusion
        ↓
LIB / finality
```

### Architecture boundary

**Solizone owns:**

- EVM execution
- accounts and balances
- nonces
- contract bytecode
- contract storage
- transaction semantics
- execution receipts
- gas accounting
- state commitments
- block semantics
- local execution checkpoints
- canonical Solizone block history

**Logos blockchain currently provides:**

- publication of canonical Solizone bytes
- shared ordering
- channel sequencing
- consensus
- data availability for the publication path
- finality

The EVM executes **inside Solizone**. Logos does not execute Solidity contracts or interpret Solizone's EVM state.

---

## Execution and state

The active execution engine is built on REVM.

The main execution path accepts a `SolizoneTransaction`, executes it against `MemoryState`, commits REVM changes, and returns an `ExecutionReceipt`.

For block execution:

```text
transactions[]
      ↓
execute transaction 0
      ↓
state changes
      ↓
execute transaction 1
      ↓
state changes
      ↓
...
      ↓
final MemoryState
      ↓
protocol state_root
      ↓
BlockExecutionOutcome
  ├── receipts
  └── state_root
```

### Current state owner

`MemoryState` wraps REVM's in-memory database and is the active owner of local EVM state.

Conceptually:

```text
MemoryState
│
├── EOA accounts
│   ├── balance
│   └── nonce
│
└── contract accounts
    ├── balance
    ├── nonce
    ├── runtime bytecode
    └── storage slots
```

The important change in the second phase is that this live state is no longer treated as disposable process memory. It can now be snapshotted, persisted, reconstructed, and continued.

---

## Persistent state and recovery

### Deterministic `StateSnapshot`

`MemoryState::snapshot()` produces a persistence-friendly representation containing account state, bytecode, and storage.

```text
StateSnapshot
└── accounts[]
    ├── address
    ├── balance
    ├── nonce
    ├── code
    └── storage[]
        ├── key
        └── value
```

Accounts and storage slots are sorted before snapshot output. This makes the persisted representation deterministic even if state was inserted in a different order.

A fresh process can reconstruct active REVM state through:

```text
persisted StateSnapshot
        ↓
MemoryState::from_snapshot(...)
        ↓
fresh REVM database
        ↓
continue execution
```

This has been tested with both synthetic account state and a real deployed Solidity `Counter` contract, including bytecode and contract storage continuity.

### State persistence boundary

State persistence is abstracted behind `StateBackend`.

```text
StateBackend
│
├── save(StateSnapshot)
└── load() -> StateSnapshot
```

The current concrete backend is:

```text
FileStateBackend
```

This keeps state serialization separate from the storage provider and gives future persistence experiments a clean integration boundary.

### Snapshot commitment vs protocol state root

There are two different hashes in the current implementation.

#### Protocol state root

```text
execution::state_root::compute_state_root(...)
```

This is the execution commitment used in canonical Solizone block headers.

It commits to deterministic account state including:

- account address
- balance
- nonce
- code hash
- storage

#### Snapshot commitment

```text
state::commitment::snapshot_commitment(...)
```

This hashes the deterministic persisted snapshot representation.

It is useful as a persistence fingerprint, but it does **not** replace the protocol state root.

```text
protocol state root      → block/execution commitment
snapshot commitment      → persistence snapshot fingerprint
```

### `SolizoneCheckpoint`

A state snapshot alone can reconstruct EVM state, but a running chain also needs to know where block production should resume.

`SolizoneCheckpoint` therefore stores:

```text
state
next_height
parent_hash
```

Conceptually:

```text
SolizoneCheckpoint
├── StateSnapshot
├── next_height = N + 1
└── parent_hash = hash(block N)
```

`FileCheckpointBackend` persists and reloads that checkpoint from disk.

### Full execution restart

```text
PROCESS A

MemoryState
    ↓
execute blocks #0 and #1
    ↓
state after block #1
    ↓
SolizoneCheckpoint
    ├── state snapshot
    ├── next_height = 2
    └── parent_hash = hash(block #1)
    ↓
file

──────────── process stops ────────────

PROCESS B

checkpoint file
    ↓
restore MemoryState
    ↓
BlockProducer::resume(
    next_height = 2,
    parent_hash = hash(block #1)
)
    ↓
execute next tx
    ↓
produce block #2
```

---

## Block production and canonical history

### `BlockProducer`

`BlockProducer` tracks the chain-head metadata needed for sequential block production:

```text
version
chain_id
gas_limit
next_height
parent_hash
```

For each block:

```text
transactions
     ↓
REVM execute_block(...)
     ↓
receipts + final state_root
     ↓
build_block(...)
     ↓
SolizoneBlock at next_height
     ↓
parent_hash = block.hash()
next_height += 1
```

This produces automatic chain linkage:

```text
Block #0
parent = 0x00...00
hash = H0
     ↓
Block #1
parent = H0
hash = H1
     ↓
Block #2
parent = H1
hash = H2
```

`BlockProducer::resume(...)` recreates the producer after restart using persisted `next_height` and `parent_hash`.

### `BlockStore`

Canonical block history is storage-agnostic behind:

```text
BlockStore
├── insert(block)
├── get_by_height(height)
├── get_by_hash(hash)
├── len()
└── is_empty()
```

Current backends:

```text
BlockStore
├── MemoryBlockStore
└── FileBlockStore
```

### `MemoryBlockStore`

The in-memory implementation maintains:

```text
height → block
hash   → height
```

It supports canonical replacement: replacing a block at a height removes the old hash index.

### `FileBlockStore`

The file implementation stores the canonical encoded bytes for each height:

```text
block-history/
├── 0.szb
├── 1.szb
└── 2.szb
```

`<height>.szb` contains `SolizoneBlock::encode()` bytes.

Height lookup is direct:

```text
height 7
   ↓
7.szb
   ↓
decode
   ↓
SolizoneBlock
```

Hash lookup currently scans `.szb` files, decodes them, computes block hashes, and returns the match. This is deliberately simple and correctness-first; a persistent hash index has not been implemented yet.

Writing a different block at an existing height overwrites the canonical height file. The old hash is therefore no longer returned as canonical.

### Full durable node recovery test

The current integration test combines both persistence paths:

```text
PROCESS A

transaction nonce 0
      ↓
Block #0
      ↓
0.szb

transaction nonce 1
      ↓
Block #1
      ↓
1.szb

MemoryState + chain head
      ↓
SolizoneCheckpoint
      ↓
checkpoint.json

DROP ALL IN-MEMORY OBJECTS

──────────── complete restart ────────────

PROCESS B

checkpoint.json
      ↓
restore MemoryState
restore BlockProducer at height 2

0.szb + 1.szb
      ↓
new FileBlockStore
      ↓
recover canonical blocks #0 / #1

transaction nonce 2
      ↓
Block #2
parent = hash(Block #1)
      ↓
2.szb
```

The test verifies:

```text
sender nonce       = 3
recipient balance  = 600
canonical history  = 3 blocks
```

---

## Canonical Solizone block

The block implementation lives in `src/block.rs`.

The canonical block begins with:

```text
SZB1
```

followed by:

- a 170-byte fixed-width header
- transaction count
- length-prefixed canonical transactions

### Header fields

```text
version
chain_id
height
parent_hash
timestamp
state_root
transactions_root
receipts_root
gas_limit
gas_used
```

Fixed header size: **170 bytes**.

### Block hash

```text
Keccak256(canonical block header bytes)
```

The body is committed through `transactions_root`, while execution results are committed through `state_root` and `receipts_root`.

### Current validation

Current block validation checks canonical structure and the transactions commitment.

It does **not** yet independently re-execute all transactions during import to verify `state_root` and `receipts_root`.

The future import-validation path is:

```text
decode block
      ↓
validate canonical structure
      ↓
validate transactions_root
      ↓
re-execute transactions
      ↓
recompute state_root
      ↓
recompute receipts_root
      ↓
compare header commitments
```

---

## Transaction model

Solizone currently uses its own deterministic prototype transaction envelope:

```text
sender
nonce
kind
value
data
gas_limit
```

Transaction kind is currently:

```text
Create
Call(address)
```

Canonical transaction encoding begins with:

```text
SZT1
```

Transaction hash:

```text
Keccak256(canonical transaction bytes)
```

### Important limitation

These are **not yet raw signed Ethereum transactions**.

Missing Ethereum transaction-layer work includes:

- EIP-2718 typed transaction support
- RLP decoding
- raw transaction parsing
- chain-id validation
- secp256k1 signature recovery
- sender derivation
- EIP-1559 fee fields
- access lists
- nonce admission
- balance admission
- fee validation

---

## Receipts

Receipt implementation lives in `src/execution/receipt.rs`.

Current statuses:

```text
Success
Revert
Halt
```

A receipt contains:

```text
status
gas_used
output
contract_address
logs
```

Canonical receipt encoding begins with:

```text
SZR1
```

Receipt hashes are committed through `receipts_root`.

The current receipt commitment is Solizone-specific and does **not** claim Ethereum receipt-trie compatibility.

---

## State commitments

The current protocol state root is a deterministic flat Solizone commitment over REVM state.

It commits to:

- accounts
- balances
- nonces
- code hashes
- storage

It is **not** an Ethereum Merkle Patricia Trie root.

That is intentional at this stage: the current priority is deterministic, reproducible execution and recovery before introducing Ethereum-compatible trie/proof structures.

---

## Logos blockchain publication

The existing publication path remains an important part of Solizone.

```text
canonical Solizone block
        ↓
SZB1 bytes
        ↓
PublicationPayload
        ↓
BlockPublisher
        ↓
LogosPublisher
        ↓
ZoneSequencer
        ↓
Mantle inscription
        ↓
Logos block
        ↓
LIB / finality
```

### Proven execution-backed publication

A previously reproduced execution-backed canonical Solizone block had:

| Field | Value |
| --- | --- |
| Solizone block height | `0` |
| Transactions | `2` |
| Gas used | `223126` |
| Payload size | `975` bytes |

Solizone block hash:

```text
0xaafe6fe8137374388925d6fc2038da9f36b88cb7ae31c51f7f92ae3e6247e540
```

Execution commitments:

```text
state_root
0xc98e732117ebb7054dc0ac4b79ae64095bc8b79c1bdffc2fb6e2453e0ee142b7

transactions_root
0x9edd2a6e48d3c88679a390e39dafd06aab7fb437aad4b1c657e725cc0740e77d

receipts_root
0x5f8eefd86c5f2218ae4282cc6a9b434bd7aec83a7fd7c6b374a2a68870871d9
```

Logos Mantle transaction carrying the block:

```text
d8c72874c1df3d3a5533f32c9e404874c9541131ae9b84b588df2b2739abda5b
```

Included in Logos block:

```text
block id
d05e25841617681c23c2114950de5ad4ef7d3ca5864a79f32f43ce175ab3cb49

slot
1318018
```

That block later became part of irreversible Logos history.

### Proof that the canonical Solizone bytes were published

The Logos inscription begins with:

```text
53 5a 42 31
```

which is ASCII:

```text
SZB1
```

The inscription also contained the same locally produced execution commitments, demonstrating that the published payload was the execution-backed canonical Solizone block rather than an unrelated test message.

### Logos terminology

#### Slot

A Logos slot is a consensus-time position or opportunity in which a block may be produced.

#### Block height

Block height counts blocks that were actually produced.

```text
slot         = consensus-time position
block height = produced-block sequence
```

They are not the same concept.

#### LIB

LIB means **Last Irreversible Block** — the latest Logos block considered irreversible/finalized by the protocol.

---

## Logos publication checkpoint

The execution checkpoint introduced in the second phase and the Logos publication checkpoint solve **different problems**.

### Solizone execution checkpoint

```text
SolizoneCheckpoint
├── state snapshot
├── next_height
└── parent_hash
```

Purpose:

```text
restore EVM state
+
restore local Solizone chain head
```

### Logos `ZoneSequencer` checkpoint

The publication harness uses:

```text
.state/sequencer-checkpoint.json
```

It tracks publication continuity such as:

```text
last_msg_id
pending_txs
lib
lib_slot
```

Purpose:

```text
restore Logos publication progress
+
reconcile pending local publication state
with canonical Logos history
```

On restart:

```text
load sequencer checkpoint
      ↓
connect to Logos
      ↓
backfill Logos history
      ↓
compare local pending state
with canonical Logos history
      ↓
update checkpoint
```

For the proven execution-backed publication, a pending transaction was eventually reconciled and removed after finality.

These two recovery domains should remain conceptually separate:

```text
execution checkpoint
    → Solizone execution + chain recovery

sequencer checkpoint
    → Logos publication recovery
```

A production node will eventually orchestrate both lifecycles.

---

## Logos Storage direction

**Logos Storage is not integrated into `solizone-evm` yet.**

The second-phase work intentionally creates persistence boundaries first so Logos Storage can be evaluated as a backend instead of being coupled directly into REVM.

Current reference implementations:

```text
execution persistence
        │
        ├── StateBackend
        │     └── FileStateBackend
        │
        └── FileCheckpointBackend

canonical block history
        │
        └── BlockStore
              ├── MemoryBlockStore
              └── FileBlockStore
```

The next storage experiment is:

> **Can Logos Storage replace or augment Solizone's local persistence backends while preserving deterministic EVM-state recovery and canonical block-history recovery across restart?**

Possible future adapters:

```text
checkpoint / snapshot persistence
        ├── local file backend
        └── Logos Storage backend

canonical block history
        ├── FileBlockStore
        └── Logos Storage block-history backend
```

The intended design rule is:

```text
REVM
 ↓
MemoryState
 ↓
persistence interface
 ↓
provider
 ├── local files
 └── Logos Storage
```

not:

```text
REVM → Logos Storage directly
```

### Logos Storage vs Logos blockchain

These are separate responsibilities:

```text
Logos Storage
    → durable storage / retrieval experiment

Logos blockchain / Mantle
    → publication / shared ordering / consensus / finality
```

The existing Logos blockchain publication path remains relevant even if Logos Storage becomes a persistence provider.

---

## Solidity contract

The current execution tests use a simple Counter contract:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Counter {
    uint256 public count;

    event Incremented(uint256 newCount);

    function increment() external {
        count += 1;
        emit Incremented(count);
    }

    function fail() external pure {
        revert("intentional failure");
    }
}
```

The contract is built through Forge using Solc 0.8.20.

The test suite proves deployment, mutation, read-only access, revert behavior, persistence, restart recovery, and continued execution against reconstructed state.

---

## Repository structure

```text
solizone-evm/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── foundry.toml
│
├── contracts/
├── contracts-out/
│
└── src/
    ├── lib.rs
    ├── main.rs
    │
    ├── block.rs
    ├── block_builder.rs
    ├── block_producer.rs
    ├── block_store.rs
    ├── memory_block_store.rs
    ├── file_block_store.rs
    │
    ├── publisher.rs
    ├── logos_publisher.rs
    │
    ├── bin/
    │   ├── publish.rs
    │   ├── state_writer.rs
    │   └── state_reader.rs
    │
    ├── state/
    │   ├── mod.rs
    │   ├── memory.rs
    │   ├── snapshot.rs
    │   ├── commitment.rs
    │   ├── backend.rs
    │   ├── file_backend.rs
    │   ├── checkpoint.rs
    │   └── file_checkpoint_backend.rs
    │
    └── execution/
        ├── mod.rs
        ├── receipt.rs
        ├── receipts_root.rs
        ├── revm_engine.rs
        ├── state_root.rs
        ├── transaction.rs
        └── transactions_root.rs
```

---

## Module responsibilities

| Module | Responsibility |
| --- | --- |
| `src/main.rs` | Small local REVM execution example. |
| `src/execution/revm_engine.rs` | Main REVM integration: transaction execution, contract deployment/calls, block execution, receipts, read-only execution, state updates. |
| `src/execution/transaction.rs` | Current canonical Solizone transaction model and encoding. |
| `src/execution/transactions_root.rs` | Deterministic transaction commitment. |
| `src/execution/receipt.rs` | Execution receipt model and canonical encoding. |
| `src/execution/receipts_root.rs` | Receipt commitment. |
| `src/execution/state_root.rs` | Protocol state root committed into Solizone block headers. |
| `src/state/memory.rs` | Active REVM state owner, snapshot creation/restore, state-root access. |
| `src/state/snapshot.rs` | Persistence-friendly account / bytecode / storage snapshot model. |
| `src/state/commitment.rs` | Deterministic snapshot commitment. |
| `src/state/backend.rs` | Generic state snapshot persistence boundary. |
| `src/state/file_backend.rs` | File-backed `StateBackend`. |
| `src/state/checkpoint.rs` | `SolizoneCheckpoint`: state + next height + parent hash. |
| `src/state/file_checkpoint_backend.rs` | File-backed execution checkpoint persistence. |
| `src/block.rs` | Canonical Solizone block/header, encoding/decoding, hash, tx-root validation. |
| `src/block_builder.rs` | Builds canonical blocks from transactions, receipts, state root, and gas data. |
| `src/block_producer.rs` | Executes batches, produces parent-linked blocks, tracks/resumes chain head. |
| `src/block_store.rs` | Storage-neutral canonical block-history interface. |
| `src/memory_block_store.rs` | In-memory canonical block history with height/hash lookup. |
| `src/file_block_store.rs` | File-backed `.szb` block history with restart recovery. |
| `src/publisher.rs` | Generic publication boundary: `SolizoneBlock → PublicationPayload → BlockPublisher`. |
| `src/logos_publisher.rs` | Logos-specific publisher adapter. |
| `src/bin/publish.rs` | End-to-end Logos publication harness. |
| `src/bin/state_writer.rs` | Process-A persistence experiment. |
| `src/bin/state_reader.rs` | Process-B state recovery + continued execution experiment. |

---

## Build, run, test

```bash
cd solizone-evm

cargo build
cargo check
cargo run
cargo test
```

### Current test coverage

The current suite covers:

- value transfer
- shared state across multiple transfers
- Solidity deployment
- generic contract calls
- read-only contract calls
- contract storage mutation
- success/revert/halt receipts
- transaction and receipt commitments
- protocol state-root changes
- deterministic snapshots
- snapshot JSON round trip
- file-backed state persistence
- snapshot commitments
- state-root continuity after restore
- contract bytecode/storage continuity after restore
- ordered block execution
- canonical block construction
- parent-linked block progression
- block-producer resume
- EVM-state + producer restart
- file-backed execution checkpoints
- in-memory block history
- file-backed block history
- block lookup by height
- block lookup by hash
- canonical block replacement
- full durable restart with state + producer + canonical history

Latest result:

```text
29 passed; 0 failed
```

---

## Publisher CLI

The current publisher harness is:

```text
src/bin/publish.rs
```

### Dry run

```bash
cargo run --bin publish
```

Builds the execution-backed canonical Solizone block without publishing it.

### Connect only

```bash
cargo run --bin publish -- --connect
```

Loads the publication checkpoint, connects to the Logos node, backfills history, and waits for the sequencer to become ready.

### Send

```bash
cargo run --bin publish -- --send
```

Publication lifecycle:

```text
build Solizone block
      ↓
convert bytes → Logos Inscription
      ↓
ZoneSequencer publish()
      ↓
accepted locally
      ↓
save checkpoint
      ↓
continue driving next_event()
      ↓
pending mempool
      ↓
on-chain
      ↓
finalized
```

> `publish()` returning does not itself prove Logos node acceptance. The sequencer must continue being driven until network state advances.

### Resume

```bash
RUST_LOG=warn cargo run --bin publish -- --resume
```

`--resume` restores the existing publication checkpoint and continues the outstanding publication lifecycle instead of creating a duplicate inscription.

---

## Current limitations

### No Ethereum wallet transaction layer yet

Solizone does not yet accept raw signed Ethereum transactions from MetaMask, Foundry `cast send`, ethers, or viem.

### No Ethereum JSON-RPC yet

There is no current `eth_sendRawTransaction`, `eth_getBlockByNumber`, `eth_getBalance`, or similar public RPC surface.

### No transaction pool yet

Transactions are currently supplied directly to execution/block-production code.

### State root is Solizone-specific

The protocol state root is deterministic and execution-derived but is not an Ethereum Merkle Patricia Trie root.

### Receipts are Solizone-specific

Current receipts are canonical Solizone receipts, not Ethereum JSON-RPC receipt objects.

### Block import validation is incomplete

Current validation does not yet independently re-execute imported blocks to verify both state and receipt roots.

### Local persistence is prototype persistence

`FileStateBackend`, `FileCheckpointBackend`, and `FileBlockStore` use straightforward file I/O.

They do not yet provide a transactional database or atomic crash-consistent commit spanning:

```text
state checkpoint + canonical block history
```

### File block hash lookup is linear

`FileBlockStore::get_by_hash()` scans `.szb` files. There is no persistent hash index yet.

### Logos Storage is not integrated yet

Logos Storage remains the next backend experiment. The current durable path uses local files.

### Logos publication is still an R&D harness

`src/bin/publish.rs` is not yet a continuously running production block publisher.

### Execution and publication recovery are separate

The local execution checkpoint and Logos sequencer checkpoint are intentionally distinct today. A production Solizone node still needs orchestration around both recovery domains.

---

## Development principles

Keep these boundaries separate even if one process eventually orchestrates all of them:

```text
Execution engine
      ↓
State / persistence boundary
      ↓
Block producer
      ↓
Canonical block store
      ↓
Publisher
      ↓
Logos blockchain integration
```

Important rules:

> **REVM should not know how Logos blockchain publication works.**

> **REVM should not depend directly on a particular storage provider.**

> **The block producer should produce canonical Solizone blocks regardless of how they are later published.**

> **A Logos Storage integration should sit behind a persistence abstraction rather than inside execution semantics.**

> **Logos blockchain publication and Logos Storage persistence are separate responsibilities.**

---

## Roadmap

### Completed second-phase foundation

```text
persistent EVM state                         ✅
deterministic snapshot / restore             ✅
file-backed state persistence                ✅
combined state + chain-head checkpoint       ✅
parent-linked multi-block production         ✅
BlockProducer abstraction                    ✅
producer restart / resume                    ✅
persistent canonical block history           ✅
height / hash block retrieval                ✅
full state + producer + history restart      ✅
```

### Immediate next steps

1. Prototype a **Logos Storage persistence backend** against the current persistence abstractions.
2. Determine whether Logos Storage is best used for state checkpoints, canonical block history, archival data, or a combination.
3. Introduce a long-running Solizone node lifecycle around the existing state, producer, checkpoint, and block-store components.
4. Add a transaction pool.
5. Accept raw signed Ethereum transactions.
6. Implement EIP-2718/RLP transaction decoding and secp256k1 sender recovery.
7. Add chain-id, nonce, balance, and fee admission rules.
8. Add a minimal Ethereum-compatible JSON-RPC surface.
9. Add imported-block re-execution and verify `state_root` / `receipts_root`.
10. Improve persistence crash consistency / atomicity.
11. Turn Logos publication into a durable service coordinated with local chain recovery.

### Next storage experiment

Reference baseline:

```text
Solizone execution state
        ↓
SolizoneCheckpoint
        ↓
FileCheckpointBackend

canonical Solizone blocks
        ↓
BlockStore
        ↓
FileBlockStore
```

Experimental extension:

```text
Solizone execution state
        ↓
checkpoint / snapshot abstraction
        ├── local file backend
        └── Logos Storage backend

canonical Solizone blocks
        ↓
BlockStore-style boundary
        ├── FileBlockStore
        └── Logos Storage-backed history
```

Experiment question:

> **Can Logos Storage replace or augment local persistence while preserving deterministic state recovery and canonical block-history recovery across restart?**

### Next developer-facing target

```text
Foundry / MetaMask / ethers / viem
        ↓
Ethereum JSON-RPC
        ↓
eth_sendRawTransaction
        ↓
Ethereum transaction decoding
        ↓
signature recovery + admission
        ↓
transaction pool
        ↓
BlockProducer
        ↓
REVM
        ↓
persistent Solizone state
        ↓
parent-linked canonical block
        ├────────────→ persistence backend
        │                local / Logos Storage experiment
        ↓
Logos blockchain publication
        ↓
Logos ordering / finality
```

This moves Solizone from a restartable execution-and-chain prototype toward a developer-facing EVM-compatible Sovereign Zone runtime.

---

## Useful commands

```bash
# enter crate
cd solizone-evm

# compile
cargo check
cargo build

# format
cargo fmt
cargo fmt -- --check

# full tests
cargo test

# Solidity Counter
cargo test executes_solidity_counter -- --nocapture

# durable state + producer + canonical-history restart
cargo test resumes_full_node_from_disk_checkpoint -- --nocapture

# block history persistence
cargo test persists_and_recovers_block_by_height -- --nocapture
cargo test persists_and_recovers_block_by_hash -- --nocapture

# local publication dry run
cargo run --bin publish

# connect without publishing
cargo run --bin publish -- --connect

# publish
cargo run --bin publish -- --send

# resume publication
RUST_LOG=warn cargo run --bin publish -- --resume

# SDK debugging
RUST_LOG=debug cargo run --bin publish -- --resume
```

Inspect the Logos publication checkpoint:

```bash
jq '{
  lib,
  lib_slot,
  pending_txs_count: (.pending_txs | length),
  pending_tx_hashes: [.pending_txs[][0]]
}' .state/sequencer-checkpoint.json
```

---

## Related documentation

| Document | Path |
| --- | --- |
| Repository overview | `../README.md` |
| Development roadmap | `../DEVELOPMENT_ROADMAP.md` |
| Experiment 001 | `../experiments/minimal-zone/` |
| Experiment 002 | `../experiments/experiment-002-publication-limits/` |
| Experiment 003 | `../experiments/experiment-003-canonical-block/` |
| Logos Storage research | `../research/` / relevant storage experiment documentation |

---

**Solizone is an independent research prototype and is not an official Logos project.**
