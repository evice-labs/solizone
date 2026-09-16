# Solizone EVM

**Solizone EVM** is the active implementation of an **EVM-compatible Sovereign Zone on Logos**.

It is an independent research project exploring a simple architectural question:

> **What would an Ethereum-compatible execution environment look like if the Zone owned execution and state, while Logos provided the shared ordering and finality foundation for the Zone's published history?**

The project is currently in active development.

The Logos-facing R&D phase is complete, and development has now moved into the actual EVM execution layer.

---

## Current Status

```text
Logos Zone publication                     ✅
Checkpoint recovery                        ✅
Ordered channel progression                ✅
Publication-size testing                   ✅
Canonical Solizone block format            ✅
Canonical block publication                ✅
Logos finalization of canonical block      ✅

REVM integrated                            ✅
First EVM value transfer                   ✅

Contract deployment                        ⏳
Contract calls / storage                   ⏳
Persistent EVM state                       ⏳
Real EVM-derived Solizone blocks           ⏳
Ethereum JSON-RPC                          ⏳
End-to-end Solizone → Logos pipeline       ⏳
```

The current implementation uses **REVM** as the EVM execution engine.

The first execution milestone has already succeeded:

```text
fund Alice
    ↓
construct EVM transfer
    ↓
execute with REVM
    ↓
observe resulting account state
```

This is where Solizone moved from synthetic block transactions into real EVM execution.

---

## What Solizone EVM Is

Solizone EVM is intended to become:

- an EVM execution environment implemented as a Logos Zone,
- a place where Ethereum-style transactions can execute,
- an environment where Solidity contracts can be deployed and called,
- a Zone that maintains its own account and contract state,
- a Zone that produces its own canonical blocks,
- a system that publishes those blocks through the Logos Zone stack,
- and an Ethereum-compatible developer surface over a Logos-native publication architecture.

The target developer experience is:

```text
Solidity developer
      ↓
Foundry / Hardhat / wallet / viem / ethers
      ↓
Ethereum JSON-RPC
      ↓
Solizone EVM
```

The developer should not need to understand Mantle internals to deploy a Solidity contract.

---

## What Solizone EVM Is Not

Solizone EVM is not:

- an EVM running directly inside Bedrock,
- a Solidity interpreter inside Mantle,
- a smart contract deployed to Logos,
- a replacement for Logos consensus,
- a replacement for the Logos Zone SDK,
- an official Logos project,
- a production-ready blockchain today.

The EVM executes **inside Solizone**.

Logos does not need to interpret EVM bytecode or maintain Solizone's full EVM state.

---

## Architecture

The intended architecture is:

```text
Ethereum User / Developer
          ↓
Ethereum-Compatible JSON-RPC
          ↓
Solizone Transaction Pool
          ↓
Solizone Block Producer
          ↓
REVM
          ↓
Solizone EVM State
          ↓
Canonical Solizone Block
          ↓
Solizone Logos Publisher
          ↓
Logos Zone SDK
          ↓
Mantle Channel
          ↓
Logos Blockchain
```

Another way to view it is by responsibility:

```text
┌─────────────────────────────────────────┐
│               SOLIZONE                  │
│                                         │
│  Ethereum RPC                           │
│  transaction validation                 │
│  transaction pool                       │
│  block production                       │
│  EVM execution                          │
│  account state                          │
│  contract bytecode                      │
│  contract storage                       │
│  receipts                               │
│  logs                                   │
│  block hashes                           │
│  state commitments                      │
└──────────────────┬──────────────────────┘
                   │
                   │ canonical block bytes
                   ▼
┌─────────────────────────────────────────┐
│            LOGOS ZONE STACK             │
│                                         │
│  Zone SDK                               │
│  Mantle channel                         │
│  publication                            │
│  shared ordering                        │
│  data availability                      │
│  consensus / finality                   │
└─────────────────────────────────────────┘
```

The design goal is to keep these boundaries explicit.

---

## Why an EVM Zone?

Logos Zones can define their own state and execution environments.

Solizone explores whether that model can host an execution environment that feels familiar to Ethereum developers.

Conceptually:

```text
Ethereum developer experience
              +
Logos Sovereign Zone model
```

This lets Solizone make decisions about:

- EVM execution,
- transaction admission,
- block production,
- local state,
- gas policy,
- block commitments,
- sequencer behavior,
- RPC compatibility,

without requiring Logos itself to become EVM-aware.

---

## The Foundation Experiments

Before starting `solizone-evm`, three experiments were completed.

They remain under `experiments/` as the R&D record behind the current architecture.

### Experiment 001 - Minimal Zone Publication

Location:

```text
../experiments/minimal-zone/
```

Experiment 001 proved that a standalone Rust process can:

- connect to a Logos node,
- initialize a `ZoneSequencer`,
- recover/backfill chain state,
- reach `Ready`,
- publish opaque bytes,
- fund the publication transaction,
- observe mempool acceptance,
- observe the Zone channel advance,
- persist a checkpoint,
- restart,
- preserve ordered history,
- and observe finalization.

The key result was:

> Solizone can keep its execution model separate while treating the Logos Zone SDK as its publication boundary.

---

### Experiment 002 - Publication Limits

Location:

```text
../experiments/experiment-002-publication-limits/
```

Experiment 002 tested publication payloads from:

```text
128 B → 1 MiB
```

No payload-size rejection was observed in that tested range.

For early development, the working guidance is:

```text
soft block publication target:
≤ 256 KiB
```

That lets Solizone begin with:

```text
one Solizone block
      ↓
one canonical payload
      ↓
one Logos Zone publication
```

without introducing chunking immediately.

---

### Experiment 003 - Canonical Solizone Block

Location:

```text
../experiments/experiment-003-canonical-block/
```

Experiment 003 defined the first canonical Solizone block.

The header contains:

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

The first canonical header was:

```text
170 bytes
```

The full test block was:

```text
245 bytes
```

with magic:

```text
SZB1
```

and block identity:

```text
Keccak256(canonical_header_bytes)
```

The experiment proved:

- deterministic serialization,
- deterministic decoding,
- deterministic block hash,
- ordered transaction commitment,
- body tamper detection,
- publication of the exact canonical bytes,
- Logos mempool acceptance,
- channel progression,
- and finalization after Logos LIB passed the publication slot.

This established the central architecture:

> **Solizone defines and understands the block. Logos does not need to.**

---

## From Experiment 003 to Real Execution

Experiment 003 used synthetic transactions:

```text
alice -> bob : 10
bob -> charlie : 4
charlie -> alice : 1
```

Its state root and receipt root were placeholders.

`solizone-evm` now replaces those synthetic pieces with actual execution:

```text
Experiment 003
synthetic transaction body
placeholder state root
placeholder receipt root

        ↓

Solizone EVM
real Ethereum transaction
REVM execution
real state transition
real receipt
real gas usage
execution-derived commitments
```

The canonical block work was intentionally completed first so the EVM has a clear output target.

---

## Current EVM Runtime

Solizone currently uses **REVM**.

REVM is the execution engine, not the entire Solizone architecture.

Conceptually:

```text
Solizone
   │
   ├── transaction validation
   ├── state
   ├── block production
   ├── persistence
   ├── RPC
   ├── publication
   │
   └── execution engine
          ↓
         REVM
```

This distinction matters because Solizone still owns chain-level behavior around EVM execution.

---

## First EVM Execution

The first local milestone was intentionally small:

```text
Alice starts funded
      ↓
Alice sends value to Bob
      ↓
REVM executes transaction
      ↓
REVM produces state changes
      ↓
Alice and Bob resulting state is inspected
```

The goal was to answer:

> Can the new Solizone implementation perform a genuine EVM state transition?

That now works.

The next execution milestone is contract deployment and storage mutation.

---

## Near-Term EVM Target

Use a minimal Solidity contract:

```solidity
contract Counter {
    uint256 public count;

    function increment() external {
        count += 1;
    }
}
```

Target flow:

```text
compile Counter.sol
        ↓
deployment bytecode
        ↓
Solizone transaction
        ↓
REVM deployment execution
        ↓
contract account created
        ↓
call increment()
        ↓
storage changes
        ↓
receipt produced
```

Once this works, Solizone will have proven the minimum path required for Solidity execution.

---

## State Model

Solizone must maintain its own EVM state.

That includes:

```text
accounts
├── address
├── balance
├── nonce
├── code
└── storage

execution history
├── transactions
├── receipts
├── logs
└── gas usage

chain history
├── blocks
├── parent relationships
└── state commitments
```

Logos does not need to store this internal state in an EVM-aware form.

Solizone maintains and verifies it.

The published canonical block is the bridge between Solizone's state history and Logos' ordered history.

---

## Block Model

The Experiment 003 block format is the current starting point:

```rust
struct SolizoneBlockHeader {
    version: u16,
    chain_id: u64,
    height: u64,
    parent_hash: [u8; 32],
    timestamp: u64,
    state_root: [u8; 32],
    transactions_root: [u8; 32],
    receipts_root: [u8; 32],
    gas_limit: u64,
    gas_used: u64,
}

struct SolizoneBlock {
    header: SolizoneBlockHeader,
    transactions: Vec<Vec<u8>>,
}
```

The real implementation will replace placeholder fields with execution-derived values:

```text
transactions_root
    ← real ordered signed Ethereum transactions

state_root
    ← resulting Solizone EVM state

receipts_root
    ← real execution receipts

gas_used
    ← actual block execution
```

The final v1 format should only be frozen after these real structures are defined.

---

## Two Different Parent Relationships

Solizone has its own chain history:

```text
Block #100
    ↓ parent_hash
Block #101
    ↓ parent_hash
Block #102
```

The Logos channel also has its own ordered message relationship.

They are related, but they are not the same thing.

```text
Solizone parent hash
    = relationship between Solizone blocks

Logos channel parent/tip
    = relationship between Zone publications
```

Solizone should never use the Logos message ID as its block hash.

---

## Two Different Mempools

There will also be two different pending-transaction layers:

```text
Ethereum user
      ↓
Solizone transaction pool
      ↓
Solizone block producer
      ↓
canonical Solizone block
      ↓
Logos publication transaction
      ↓
Logos mempool
```

The Solizone mempool contains EVM transactions.

The Logos mempool contains the transaction carrying a Zone publication.

These should never be represented as one internal concept.

---

## Publication and Finality

Publication should be modeled explicitly:

```text
PRODUCED
   ↓
PUBLICATION_REQUESTED
   ↓
MEMPOOL_ACCEPTED
   ↓
INCLUDED
   ↓
FINALIZED
```

Experiment 003 showed why these states matter.

A block can exist locally before Logos has finalized its publication.

So Solizone may eventually track:

```text
local head
published head
included head
finalized head
```

and these can temporarily point to different blocks.

---

## EVM Compatibility vs Ethereum Equivalence

The current goal is **EVM compatibility**, not immediate byte-for-byte replication of every Ethereum protocol structure.

The initial target is:

- EVM bytecode compatibility,
- Solidity contract execution,
- Ethereum-style accounts,
- signed Ethereum transactions,
- familiar JSON-RPC,
- familiar developer tooling.

Solizone may still use its own:

- block format,
- block hash,
- state commitment format,
- sequencing policy,
- publication lifecycle.

If exact Ethereum-equivalent roots or other protocol structures become necessary later, they can be evaluated deliberately.

---

## Planned Components

The codebase is expected to grow into:

```text
ExecutionEngine
    └── REVM implementation

StateBackend
    ├── in-memory implementation
    └── persistent implementation

TransactionDecoder / Validator

SolizoneMempool

BlockProducer

CanonicalBlockCodec

ChainStore

BlockPublisher
    ├── LogosPublisher
    └── MockPublisher

Ethereum JSON-RPC

Indexer / Query Layer

Replay / Verification Node
```

These should be added incrementally, not scaffolded all at once.

---

## Possible Code Structure

```text
solizone-evm/
├── Cargo.toml
├── README.md
├── DEVELOPMENT_ROADMAP.md
│
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── execution/
│   │   └── revm_engine.rs
│   ├── state/
│   │   ├── memory.rs
│   │   ├── persistent.rs
│   │   └── commitment.rs
│   ├── transaction/
│   │   ├── decode.rs
│   │   └── validate.rs
│   ├── mempool/
│   │   └── mod.rs
│   ├── block/
│   │   ├── header.rs
│   │   ├── codec.rs
│   │   ├── commitment.rs
│   │   └── producer.rs
│   ├── chain/
│   │   └── storage.rs
│   ├── publisher/
│   │   ├── logos.rs
│   │   └── mock.rs
│   ├── rpc/
│   │   └── eth.rs
│   └── indexer/
│       └── mod.rs
│
└── tests/
```

This is a direction, not a requirement to create every module immediately.

---

## End-to-End Target

The first complete Solizone MVP should support:

```text
1. User signs an Ethereum transaction.

2. Transaction is submitted to Solizone JSON-RPC.

3. Solizone validates it.

4. Transaction enters the Solizone mempool.

5. Block producer selects it.

6. REVM executes it.

7. State changes are committed.

8. Receipt is generated.

9. Canonical Solizone block is constructed.

10. Block is persisted locally.

11. Exact canonical block bytes are published through the Logos Zone SDK.

12. Logos orders the publication.

13. Solizone observes inclusion.

14. Logos LIB passes the publication.

15. Solizone marks the block finalized.
```

For a developer, that should eventually look like an ordinary EVM transaction.

---

## Developer Experience Goal

A future developer should be able to run something close to:

```bash
forge create src/Counter.sol:Counter   --rpc-url http://localhost:8545   --private-key ...
```

and then:

```bash
cast send <contract> "increment()"   --rpc-url http://localhost:8545   --private-key ...
```

without manually dealing with:

- Mantle inscriptions,
- Logos funding notes,
- Zone checkpoints,
- channel message construction,
- Logos transaction construction.

Those belong behind the Solizone infrastructure layer.

---

## Logos Integration Boundary

The proven Experiment 003 publication logic should eventually sit behind an interface like:

```rust
trait BlockPublisher {
    async fn publish(
        &mut self,
        block: &CanonicalBlock,
    ) -> Result<PublicationId, PublishError>;

    async fn status(
        &mut self,
        publication: &PublicationId,
    ) -> Result<PublicationStatus, PublishError>;
}
```

This allows:

```text
unit / block tests
        ↓
MockPublisher
```

and:

```text
real integration
        ↓
LogosPublisher
```

without coupling REVM to Logos APIs.

---

## Security Model - Current

The current project is a research prototype.

The initial model can assume:

```text
single trusted block producer
```

while still enforcing deterministic execution and reproducible state.

That is not the intended final trust model.

A longer-term progression may be:

```text
independent replay nodes
        ↓
multi-node verification
        ↓
challenge / fraud proofs
and/or
validity proofs
        ↓
decentralized sequencing
```

The proof system should not be fixed before the execution and state architecture matures.

---

## Current Non-Goals

This phase is not trying to solve all of these immediately:

- decentralized sequencer committees,
- production bridge security,
- zkEVM proving,
- full Ethereum RPC parity,
- maximum throughput,
- permissionless sequencing,
- every Ethereum transaction type,
- production economic security,
- mainnet readiness.

The immediate priorities are correctness, deterministic execution, and clean boundaries.

---

## Repository Context

```text
solizone/
├── README.md
├── dev-roadmap/
├── research/
│
├── experiments/
│   ├── minimal-zone/
│   ├── experiment-002-publication-limits/
│   └── experiment-003-canonical-block/
│
└── solizone-evm/
    ├── Cargo.toml
    ├── README.md
    ├── DEVELOPMENT_ROADMAP.md
    └── src/
```

`experiments/` preserves the R&D history.

`solizone-evm/` is the real implementation.

---

## Immediate Next Steps

From the currently working REVM transfer:

```text
1. Move execution behind an ExecutionEngine module.

2. Replace benchmark addresses with explicit test accounts.

3. Convert the working value transfer into an automated test.

4. Deploy a minimal Solidity contract through REVM.

5. Execute a contract call and verify storage.

6. Capture a real transaction receipt.

7. Add persistent state behind a StateBackend abstraction.

8. Begin replacing Experiment 003 placeholder block values
   with real EVM-derived values.
```

See:

```text
DEVELOPMENT_ROADMAP.md
```

for the full milestone plan.

---

## Project Thesis

Solizone explores this separation:

```text
Execution belongs to the Zone.

Canonical execution history belongs to the Zone.

Logos provides the shared foundation underneath
for publishing, ordering, and finalizing that history.
```

If this separation continues to hold as the implementation grows, Ethereum developers can get a familiar execution environment while the system remains native to the Logos Zone architecture.

---

## Disclaimer

Solizone is an independent research prototype and is **not an official Logos project**.
