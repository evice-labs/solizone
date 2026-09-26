# Solizone EVM - Development Roadmap

> **Status:** Active development  
> **Branch:** `feat/second-phase`  
> **Project:** Independent research prototype for an EVM-compatible Sovereign Zone on Logos

---

## 1. Goal

Solizone is building an EVM execution environment where:

```text
Ethereum developer tooling
        ↓
Solizone
  ├── transaction handling
  ├── REVM execution
  ├── EVM state
  ├── block production
  ├── persistence
  └── canonical chain history
        ↓
Logos publication
        ↓
ordering / consensus / finality
```

The architectural rule is simple:

```text
Solizone owns execution and canonical EVM history.

Logos provides the shared publication,
ordering, data-availability, and finality layer.
```

REVM should not depend directly on Logos APIs or on one specific storage backend.

---

## 2. Completed foundation

### Logos research and publication path ✅

The initial Logos-facing experiments proved:

- Zone SDK connectivity
- `ZoneSequencer` startup and recovery
- ordered channel publication
- publication checkpoint persistence
- restart / resume behavior
- publication-size characterization
- canonical `SZB1` Solizone block encoding
- publication of exact canonical block bytes
- Logos block inclusion
- LIB / finality observation

The active implementation also has a reusable `BlockPublisher` boundary and Logos publisher adapter.

---

### EVM execution core ✅

Implemented with REVM:

- EOA value transfer
- generic transaction execution
- Solidity contract deployment
- contract calls
- contract storage mutation
- read-only execution
- shared state across multiple transactions
- nonce and balance continuity
- success / revert / halt receipts
- logs and gas accounting
- ordered block execution

---

### Deterministic state and commitments ✅

Implemented:

- deterministic Solizone protocol state root
- transaction root
- receipt root
- deterministic state snapshots
- snapshot JSON encoding / decoding
- deterministic snapshot commitment
- snapshot reconstruction into fresh REVM state
- contract bytecode and storage continuity after restore

The protocol state root and persistence snapshot commitment remain separate concepts.

---

### Persistent EVM state ✅

Implemented:

```text
MemoryState
    ↓
StateSnapshot
    ↓
StateBackend
    ├── FileStateBackend
    └── future storage backend
```

A fresh process can reconstruct the EVM world from persisted state and continue execution without redeploying contracts.

---

### Parent-linked block production ✅

Implemented `BlockProducer`:

```text
transactions
    ↓
REVM execution
    ↓
state root + receipts
    ↓
canonical Solizone block
    ↓
advance height + parent hash
```

Proven chain progression:

```text
Block #0
   ↓
Block #1
   ↓
Block #2
```

with each block referencing the hash of its canonical parent.

`BlockProducer::resume(...)` restores the next height and parent hash after restart.

---

### Full execution checkpoint recovery ✅

Implemented:

```text
SolizoneCheckpoint
├── StateSnapshot
├── next_height
└── parent_hash
```

with file-backed persistence through `FileCheckpointBackend`.

Proven restart flow:

```text
running node
    ↓
persist checkpoint
    ↓
process stops
    ↓
restore MemoryState
    ↓
resume BlockProducer
    ↓
continue execution
```

---

### Persistent canonical block history ✅

Implemented storage boundary:

```text
BlockStore
├── MemoryBlockStore
└── FileBlockStore
```

Current capabilities:

- insert canonical block
- retrieve by height
- retrieve by hash
- canonical replacement at the same height
- disk persistence
- history recovery after restart

File-backed blocks are stored as canonical `.szb` bytes:

```text
0.szb
1.szb
2.szb
...
```

---

### Full durable node restart ✅

The current integration test proves recovery of:

```text
EVM state
+
BlockProducer chain head
+
canonical block history
```

Flow:

```text
PROCESS A

produce #0
produce #1
persist checkpoint
persist 0.szb / 1.szb

        ↓ complete shutdown

PROCESS B

restore state
restore producer at height 2
recover block #0 / #1
produce #2
persist 2.szb
```

Current test status on `feat/second-phase`:

```text
29 passed
0 failed
```

---

## 3. Current phase - Storage and node lifecycle

### Milestone 14 - Logos Storage backend experiment

**Status: NEXT**

Goal:

> Determine whether Logos Storage can replace or augment Solizone's local persistence backend while preserving deterministic state and canonical block-history recovery.

The existing local interfaces become the reference boundary:

```text
State / checkpoint persistence
        │
        ├── File backend          ✅
        └── Logos Storage         ⏳

Canonical block history
        │
        ├── FileBlockStore        ✅
        └── Logos Storage         ⏳
```

Initial questions:

1. Can a Solizone checkpoint be stored and retrieved reliably through Logos Storage?
2. Can canonical `.szb` block bytes be stored and recovered by deterministic identifiers?
3. Can a fresh Solizone process reconstruct state and block history from Logos Storage?
4. Should Logos Storage act as:
   - primary persistence,
   - replicated persistence,
   - historical archive,
   - or content-addressed block storage?
5. What latency / size / consistency assumptions affect block production?

Exit criteria:

```text
persist checkpoint
persist canonical blocks
        ↓
process restart
        ↓
recover through Logos Storage
        ↓
restore EVM state
restore chain history
        ↓
produce next valid block
```

Important:

```text
Logos Storage != Logos blockchain publication

Logos Storage
    → persistence / retrieval

Logos blockchain
    → ordering / consensus / finality
```

---

### Milestone 15 - Long-running Solizone node

Turn the currently tested components into one runtime lifecycle:

```text
startup
   ↓
load checkpoint
   ↓
recover block history
   ↓
initialize REVM state
   ↓
resume BlockProducer
   ↓
accept transactions
   ↓
produce blocks
   ↓
persist
   ↓
publish
```

Goals:

- one node entry point
- coordinated startup / shutdown
- automatic recovery
- periodic / block-boundary checkpointing
- persistence failure handling
- publication worker separation
- clean configuration

Exit criteria:

```text
start node
→ produce blocks
→ stop node
→ restart node
→ continue from previous chain head
```

without test-only orchestration.

---

## 4. Ethereum compatibility phase

### Milestone 16 - Raw Ethereum transaction support

Current Solizone transactions are internal deterministic envelopes.

Add:

```text
raw signed Ethereum transaction
        ↓
decode typed transaction
        ↓
validate chain ID
        ↓
recover sender
        ↓
validate signature
        ↓
validate nonce / balance / fee
        ↓
convert into Solizone execution transaction
        ↓
REVM
```

Initial scope:

- EIP-2718 envelope handling
- RLP decoding
- secp256k1 recovery
- Ethereum transaction hash
- chain-id validation
- nonce admission
- balance admission
- basic fee handling

Exit criteria:

> A signed transaction produced by standard Ethereum tooling can be decoded, sender-recovered, validated, and executed by Solizone.

---

### Milestone 17 - Solizone transaction pool

Add a distinct EVM-side pending transaction layer:

```text
Ethereum transaction
        ↓
Solizone mempool
        ↓
BlockProducer
        ↓
Solizone block
        ↓
Logos publication transaction
        ↓
Logos mempool
```

Initial rules:

- FIFO baseline
- sender nonce ordering
- duplicate rejection
- invalid transaction filtering
- capacity limits

Later:

- replacement policy
- fee ordering
- expiry rules

---

### Milestone 18 - Ethereum-compatible JSON-RPC

Expose the Solizone runtime through familiar Ethereum APIs.

Initial methods:

```text
web3_clientVersion

eth_chainId
eth_blockNumber

eth_getBalance
eth_getTransactionCount
eth_getCode

eth_sendRawTransaction
eth_getTransactionByHash
eth_getTransactionReceipt

eth_call
eth_estimateGas

eth_getBlockByNumber
eth_getBlockByHash
eth_getLogs
```

Target tooling:

- Foundry
- ethers.js
- viem
- Hardhat
- MetaMask-compatible wallets

Major developer milestone:

```text
forge create Counter.sol
        ↓
Solizone RPC
        ↓
REVM deployment
        ↓
canonical Solizone block
        ↓
persistent history
        ↓
Logos publication
```

---

## 5. Validation and reliability phase

### Milestone 19 - Full block re-execution validation

Current block validation checks canonical structure and the transaction commitment.

Add:

```text
decode block
    ↓
validate transaction root
    ↓
re-execute transactions
    ↓
recompute state root
    ↓
recompute receipts root
    ↓
compare header commitments
```

Exit criteria:

> A second process can independently verify a Solizone block from previous state plus canonical transactions.

---

### Milestone 20 - Crash-consistent persistence

Current file persistence is intentionally simple.

Improve:

- atomic checkpoint writes
- atomic canonical block writes
- coordinated block + checkpoint commit semantics
- corruption detection
- recovery from partial writes
- versioned persisted formats

Also replace linear file scanning for hash lookup with a persistent index when required.

---

### Milestone 21 - Durable Logos publisher service

Turn the current publication harness into a node service.

Responsibilities:

- queue produced Solizone blocks
- publish through Logos Zone SDK
- persist publication state
- safely resume pending publication after restart
- track:
  - requested
  - mempool accepted
  - included
  - finalized
- avoid duplicate publication
- apply backpressure

Keep execution and publication asynchronous but explicitly coordinated.

---

## 6. End-to-end MVP

The first developer-facing Solizone MVP is:

```text
Ethereum wallet / Foundry
        ↓
signed Ethereum transaction
        ↓
JSON-RPC
        ↓
Solizone mempool
        ↓
BlockProducer
        ↓
REVM
        ↓
persistent EVM state
        ↓
canonical Solizone block
        ↓
BlockStore / storage backend
        ↓
Logos publisher
        ↓
Logos blockchain
        ↓
inclusion
        ↓
finality
```

MVP success means:

> A developer can deploy and interact with a Solidity contract using familiar Ethereum tooling, restart the Solizone node without losing state or history, and observe the resulting canonical Solizone blocks being published and finalized through Logos.

---

## 7. Later research

These are deliberately **not immediate milestones**:

- independent replay / verifier nodes
- reorg policy
- bridge and asset accounting
- cross-Zone messaging
- decentralized sequencing
- fraud proofs
- validity proofs
- Ethereum-equivalent MPT roots
- zkEVM proving
- production economic security

They should be addressed only after the execution, persistence, transaction, and RPC layers are stable.

---

## 8. Current priority order

```text
1. Logos Storage experiment
        ↓
2. Long-running Solizone node
        ↓
3. Raw Ethereum transactions
        ↓
4. Solizone mempool
        ↓
5. Ethereum JSON-RPC
        ↓
6. Full block re-execution validation
        ↓
7. Crash-consistent persistence
        ↓
8. Durable Logos publication service
        ↓
9. End-to-end developer MVP
```

---

## 9. Engineering principles

1. **Execution remains independent from Logos publication.**
2. **Persistence remains replaceable behind clean interfaces.**
3. **Canonical block bytes must remain deterministic.**
4. **Protocol state roots and persistence fingerprints remain separate.**
5. **A block can exist locally before Logos finalizes its publication.**
6. **Solizone mempool and Logos mempool are different systems.**
7. **Correctness and restartability come before throughput optimization.**
8. **Do not add proof systems or bridge complexity before the execution chain is stable.**

---

## 10. Current implementation summary

```text
REVM execution                            ✅
Solidity deployment / calls              ✅
Persistent EVM state                     ✅
Deterministic snapshots                  ✅
State / receipt / transaction roots      ✅
Canonical SZB1 blocks                    ✅
Parent-linked BlockProducer              ✅
Producer restart / resume                ✅
Persistent canonical block history       ✅
Full state + chain restart               ✅
Logos publication / finality proof       ✅

Logos Storage backend                    ⏳
Long-running node                        ⏳
Raw Ethereum transactions                ⏳
Solizone mempool                         ⏳
Ethereum JSON-RPC                        ⏳
Full re-execution validation             ⏳
Production durability                    ⏳
Production publisher service             ⏳
```
