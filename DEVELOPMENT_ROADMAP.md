# Solizone EVM - Development Roadmap

> **Status:** Active development  
> **Current stage:** EVM execution core  
> **Foundation experiments:** 3/3 complete  
> **Project type:** Independent research prototype for an EVM-compatible Sovereign Zone on Logos

## 1. Purpose

This is the implementation roadmap for `solizone-evm`.

The project deliberately began with three small Logos-facing experiments before adding an EVM. Those experiments are now complete, so future work moves into the actual `solizone-evm/` implementation rather than continuing as `experiment-004`, `experiment-005`, etc.

The target architecture is:

```text
Ethereum Developer / User
          ↓
Ethereum-Compatible JSON-RPC
          ↓
Solizone Transaction Pool
          ↓
Solizone Block Producer
          ↓
EVM Runtime
          ↓
Solizone State
          ↓
Canonical Solizone Block
          ↓
Logos / Bedrock Publisher
          ↓
Logos Zone SDK
          ↓
Mantle Channel
          ↓
Logos Blockchain
```

The core separation is:

```text
Solizone
  owns execution
  owns EVM state
  owns transaction semantics
  owns block construction
  owns block identity
  owns local chain history

Logos
  provides the shared consensus foundation
  orders Zone publications
  provides data availability for published Zone history
  provides finality for those publications
  provides the interoperability substrate
```

Bedrock does not need to execute or interpret EVM transactions.

---

## 2. Foundation Phase - COMPLETE

### Experiment 001 - Minimal Logos Zone Publication ✅

Location:

```text
experiments/minimal-zone/
```

Proved that an independent Solizone process can:

```text
connect to a Logos node
        ↓
initialize ZoneSequencer
        ↓
backfill / recover channel state
        ↓
reach Ready
        ↓
publish opaque bytes
        ↓
observe Logos mempool acceptance
        ↓
observe channel progression
        ↓
persist checkpoint
        ↓
restart
        ↓
continue ordered history
        ↓
observe finalization
```

**Architectural outcome:** the Logos Zone SDK can be treated as Solizone's publication boundary while the execution model remains separate.

---

### Experiment 002 - Publication & Inscription Constraints ✅

Location:

```text
experiments/experiment-002-publication-limits/
```

Successful publication was tested from:

```text
128 B
1 KiB
4 KiB
16 KiB
32 KiB
64 KiB
128 KiB
256 KiB
512 KiB
1 MiB
```

No payload-size rejection was observed in the tested range.

Working design guidance:

```text
Largest tested payload:
1 MiB

Initial soft operating target:
≤ 256 KiB

Immediate chunking requirement:
No

Funding / fee management:
Required

Mempool acceptance:
Not equivalent to finality
```

**Architectural outcome:** the first Solizone implementation can use one canonical block per Zone publication without premature chunking.

The 1 MiB result is a tested value, not a protocol maximum.

---

### Experiment 003 - Canonical Solizone Block ✅ FINALIZED

Location:

```text
experiments/experiment-003-canonical-block/
```

Experiment 003 defined a deterministic Solizone-native block:

```text
SolizoneBlockHeader
├── version
├── chain_id
├── height
├── parent_hash
├── timestamp
├── state_root
├── transactions_root
├── receipts_root
├── gas_limit
└── gas_used

SolizoneBlock
├── header
└── transactions
```

It established:

```text
header size:     170 bytes
full test block: 245 bytes
magic:           SZB1
block hash:      Keccak256(canonical_header_bytes)
```

The experiment proved:

- deterministic encoding and decoding,
- deterministic block identity,
- transaction-body commitment,
- tamper rejection,
- exact artifact publication,
- Logos mempool acceptance,
- wallet state transition,
- channel-tip progression,
- and finalization after Logos LIB passed the publication slot.

**Architectural outcome:** Solizone can own the semantics and identity of its blocks while Logos handles publication, ordering, and finality.

---

## 3. Current Development State

The R&D phase has transitioned into implementation:

```text
Logos publication path                 ✅
Publication-size characterization      ✅
Canonical Solizone block               ✅
Canonical block finalization           ✅

REVM integration                       ✅
First EVM value transfer               ✅

Contract deployment                    ⏳
Persistent EVM state                   ⏳
Raw Ethereum transaction support       ⏳
Real EVM-derived Solizone blocks       ⏳
Ethereum JSON-RPC                      ⏳
Full end-to-end pipeline               ⏳
```

The first real EVM execution now works locally:

```text
fund Alice
    ↓
construct EVM value transfer
    ↓
execute through REVM
    ↓
observe resulting account state
```

This is the first point where Solizone performs actual EVM execution instead of using synthetic transaction strings.

---

## 4. Milestone 1 - EVM Execution Core

**Status: IN PROGRESS**

Solizone currently uses **REVM** as the EVM execution engine.

### Immediate goals

1. Move REVM execution behind a clean `ExecutionEngine` abstraction.
2. Replace benchmark addresses with explicit Solizone test accounts.
3. Turn the working value transfer into an automated test.
4. Support contract deployment.
5. Support contract calls and storage mutation.
6. Capture success/revert, gas usage, logs, and receipts.

Suggested boundary:

```rust
pub trait ExecutionEngine {
    fn execute_transaction(...);
    fn execute_block(...);
}
```

Possible module:

```text
src/execution/
├── mod.rs
└── revm_engine.rs
```

### Contract milestone

Use a minimal contract:

```solidity
contract Counter {
    uint256 public count;

    function increment() external {
        count += 1;
    }
}
```

Target:

```text
compile Counter.sol
      ↓
deployment bytecode
      ↓
REVM deployment
      ↓
contract account created
      ↓
call increment()
      ↓
storage changes
      ↓
receipt produced
```

### Exit criteria

- EOA transfers,
- contract deployment,
- contract calls,
- success and revert paths,
- balance changes,
- nonce changes,
- contract bytecode,
- contract storage,
- gas accounting,
- logs,
- receipts.

---

## 5. Milestone 2 - Persistent Solizone State

A VM call is not yet a chain. State must survive restarts.

Solizone needs to persist:

```text
accounts
balances
nonces
contract bytecode
contract storage
receipts
logs
block metadata
transaction metadata
```

Use an abstraction such as:

```rust
trait StateBackend {
    fn account(...);
    fn storage(...);
    fn commit(...);
}
```

Potential structure:

```text
src/state/
├── mod.rs
├── memory.rs
├── persistent.rs
└── commitment.rs
```

Start with a simple deterministic embedded database. The concrete database should remain replaceable behind the interface.

### State commitment

The first requirement is determinism:

```text
same previous state
+
same ordered transactions
=
same resulting state commitment
```

Exact Ethereum state-trie equivalence is not an initial requirement unless the compatibility target later demands it.

### Exit criteria

Restarting Solizone preserves:

- balances,
- nonces,
- bytecode,
- storage,
- receipts,
- and deterministic state commitments.

---

## 6. Milestone 3 - Real Ethereum Transaction Layer

The current prototype builds REVM transaction environments directly.

Normal Ethereum users submit signed serialized transactions.

Solizone therefore needs:

```text
raw signed transaction
        ↓
decode transaction envelope
        ↓
recover sender
        ↓
validate chain ID
        ↓
validate signature
        ↓
validate nonce / fee fields
        ↓
convert to REVM execution environment
```

Support only the transaction types needed for the MVP first, then expand deliberately.

### Exit criteria

A raw transaction produced by normal Ethereum tooling can be decoded, validated, sender-recovered, executed, and assigned its normal Ethereum transaction hash.

---

## 7. Milestone 4 - Solizone Transaction Pool

The Solizone mempool and Logos mempool are different systems:

```text
Ethereum user transaction
          ↓
Solizone mempool
          ↓
Solizone block
          ↓
Logos publication transaction
          ↓
Logos mempool
```

Start with FIFO, then add:

- sender nonce ordering,
- duplicate detection,
- invalid transaction filtering,
- replacement rules,
- fee ordering,
- capacity limits,
- expiry policy.

### Exit criteria

Multiple Ethereum transactions can remain pending and be selected deterministically for block production.

---

## 8. Milestone 5 - Real Solizone Block Production

This is where Experiment 003 and the EVM runtime converge.

Target:

```text
Solizone mempool
      ↓
select ordered transactions
      ↓
execute through REVM
      ↓
commit resulting state
      ↓
produce receipts
      ↓
calculate commitments
      ↓
build canonical Solizone block
      ↓
persist block
```

The Experiment 003 header remains the starting point:

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

Replace placeholders with real execution-derived values:

```text
transactions_root ← ordered signed transactions
state_root        ← resulting Solizone EVM state
receipts_root     ← execution receipts
gas_used          ← actual execution
```

Before freezing block-format v1, review:

- versioning,
- transaction representation,
- receipt representation,
- commitment algorithms,
- sequencer/block signatures,
- future-extension strategy.

### Parent relationships

Keep these separate:

```text
Solizone block.parent_hash
    = Solizone chain relationship

Logos channel parent/tip
    = Zone publication relationship
```

### Exit criteria

Produce a deterministic sequence:

```text
Block #0 → Block #1 → Block #2
```

with execution-derived state, transaction, receipt, and gas commitments.

---

## 9. Milestone 6 - Local Chain Storage

A Solizone block should exist locally before Logos finalizes its publication.

Persist:

```text
block hash
height
parent hash
timestamp
transactions
receipts
state commitment
publication status
Logos transaction hash
Logos channel MsgId
Logos inclusion slot
Logos finalization slot
```

Model publication explicitly:

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

Possible failure states:

```text
PUBLICATION_FAILED
DROPPED
ORPHANED
RETRY_PENDING
```

Avoid a single `confirmed: bool`.

---

## 10. Milestone 7 - Reusable Logos Publisher

Experiment 003 already contains the working integration path.

Refactor it into a production component:

```rust
trait BlockPublisher {
    async fn publish(&mut self, block: &CanonicalBlock)
        -> Result<PublicationId, PublishError>;

    async fn status(&mut self, id: &PublicationId)
        -> Result<PublicationStatus, PublishError>;
}
```

Implement:

```text
LogosPublisher
MockPublisher
```

The real publisher should handle:

- Logos node connection,
- ZoneSequencer initialization,
- checkpoint restore,
- backfill and `Ready`,
- canonical block publication,
- funding,
- checkpoint persistence,
- mempool events,
- channel state,
- inclusion mapping,
- LIB finalization,
- safe retry after restart.

Add backpressure:

```text
max_pending_publications
```

Use Experiment 002's initial soft target:

```text
≤ 256 KiB per publication
```

Only add chunking or external-DA patterns if real block size requires it.

---

## 11. Milestone 8 - Ethereum-Compatible JSON-RPC

Once execution, state, blocks, and publication work, expose an Ethereum-compatible interface.

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

```text
Foundry
Hardhat
viem
ethers.js
MetaMask-compatible wallets
```

Major milestone:

```text
forge create Counter.sol
      ↓
Solizone RPC
      ↓
deployment executes
      ↓
canonical Solizone block
      ↓
Logos publication
      ↓
Logos finality
```

---

## 12. Milestone 9 - End-to-End MVP

The first complete MVP is:

```text
Ethereum wallet
      ↓
signed transaction
      ↓
JSON-RPC
      ↓
Solizone mempool
      ↓
block producer
      ↓
REVM
      ↓
state commit
      ↓
receipt
      ↓
canonical Solizone block
      ↓
local chain storage
      ↓
Logos publisher
      ↓
Mantle channel
      ↓
Logos inclusion
      ↓
LIB finalization
```

### MVP definition

A developer can deploy and interact with a Solidity contract using normal Ethereum tooling, while the resulting Solizone block history is reliably published and finalized through Logos.

---

## 13. Milestone 10 - Indexer and Query Layer

Build readable history without forcing clients to replay the Zone channel.

Index:

```text
blocks
transactions
receipts
logs
contracts
addresses
publication state
finality state
```

Maintain mappings:

```text
Ethereum transaction hash
        ↔
Solizone block hash
        ↔
Logos channel MsgId
        ↔
Logos transaction hash
```

This supports explorers, debugging, monitoring, and verification.

---

## 14. Milestone 11 - Independent Replay Node

Before decentralized sequencing, build a non-producing verification node.

It should:

```text
read canonical Solizone blocks
        ↓
decode transactions
        ↓
re-execute with REVM
        ↓
recompute receipts
        ↓
recompute state
        ↓
recompute commitments
        ↓
verify block
```

This is the first major trust-reduction milestone.

---

## 15. Milestone 12 - Deposit / Withdrawal Prototype

Only start bridge work after the execution chain is stable.

Deposit:

```text
Logos / Mantle-side asset
        ↓
finalized deposit
        ↓
Solizone observes deposit
        ↓
credit corresponding EVM representation
```

Withdrawal:

```text
Solizone withdrawal request
        ↓
debit / burn EVM representation
        ↓
publish withdrawal state
        ↓
release Logos-side asset
```

Invariant:

```text
total withdrawable Solizone representation
<=
assets actually controlled for Solizone
```

Do not use meaningful value until accounting, replay, restart, and failure behavior are tested.

---

## 16. Milestone 13 - Cross-Zone Messaging

Expose Logos interoperability through a clean EVM-facing abstraction rather than raw Mantle internals.

Possible design:

```text
Solidity contract
      ↓
Solizone system contract / precompile
      ↓
outgoing Zone message
      ↓
Logos
      ↓
destination Zone
```

Incoming:

```text
other Zone
      ↓
Logos
      ↓
Solizone inbox
      ↓
verified message
      ↓
EVM-visible event / execution
```

Research:

- replay protection,
- destination addressing,
- ordering,
- failure semantics,
- acknowledgements,
- cross-Zone asset handling.

---

## 17. Milestone 14 - Verification and Trust Reduction

Initial MVP:

```text
single trusted block producer
+
deterministic execution
```

Then:

```text
independent replay nodes
        ↓
multi-node verification
        ↓
challenge/fraud-proof system
and/or
validity proofs
```

Do not commit to a proof system before the execution/state architecture stabilizes.

---

## 18. Milestone 15 - Decentralized Sequencing

This comes significantly later.

Start:

```text
1 block producer / sequencer
```

Then evaluate:

```text
sequencer committee
```

and whatever sequencing mechanisms are actually supported and appropriate in the Logos stack at that time.

Research areas:

- writer authorization,
- leader selection,
- writer predictability,
- censorship resistance,
- duplicate proposals,
- conflicting blocks,
- Solizone fork choice,
- publication races,
- channel configuration changes,
- sequencer key rotation,
- failed-writer recovery.

Keep block production and Logos publication as separate interfaces so this remains evolvable.

---

## 19. Milestone 16 - Production Hardening

### Testing

- deterministic execution tests,
- state transition vectors,
- malformed transaction tests,
- block encoding vectors,
- tamper tests,
- persistence/restart tests,
- publisher restart tests,
- publication retry tests,
- Logos reorg tests,
- long-running synchronization tests,
- RPC compatibility tests,
- fuzz tests.

### Operations

- structured logs,
- metrics,
- health endpoints,
- backups,
- database migrations,
- crash recovery,
- key management,
- rate limits,
- transaction limits,
- resource limits.

### Security review

Review:

- EVM configuration,
- chain ID behavior,
- transaction validation,
- fee accounting,
- state commitment,
- block encoding,
- replay protection,
- bridge accounting,
- sequencer authentication,
- RPC exposure,
- key handling.

---

## 20. Suggested Codebase Evolution

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
    ├── execution.rs
    ├── block_vectors.rs
    ├── persistence.rs
    └── end_to_end.rs
```

Do not scaffold all of this immediately. Add boundaries when the corresponding milestone requires them.

---

## 21. Recommended Development Order From Today

```text
[COMPLETE]
1. Logos Zone publication proof

[COMPLETE]
2. Publication-size characterization

[COMPLETE]
3. Canonical Solizone block + Logos finalization

[CURRENT]
4. REVM execution core
   └── EVM value transfer already proven

[NEXT]
5. Contract deployment + contract call + receipt

6. Persistent state backend

7. Raw Ethereum transaction decoding / validation

8. Solizone mempool

9. Real EVM-derived canonical Solizone blocks

10. Local Solizone chain persistence

11. Extract reusable Logos publisher

12. End-to-end block → Logos finality

13. Ethereum JSON-RPC

14. Foundry / wallet developer experience

15. Indexer + replay node

16. Bridge prototype

17. Cross-Zone messaging

18. Verification / proof research

19. Decentralized sequencing

20. Production hardening
```

---

## 22. Near-Term Definition of Success

The most important target is:

```text
real signed Ethereum transaction
        ↓
Solizone
        ↓
REVM execution
        ↓
persistent state transition
        ↓
real receipt
        ↓
canonical Solizone block
        ↓
Logos publication
        ↓
Logos finalization
```

Once that loop works reliably, Solizone has moved from connected research components into a functional EVM execution Zone.

---

## 23. Long-Term Product Goal

From the developer side:

```text
Solidity
Foundry / Hardhat
MetaMask-compatible wallet
ethers / viem
        ↓
Solizone JSON-RPC
```

Underneath:

```text
EVM execution
      ↓
Solizone state
      ↓
Solizone blocks
      ↓
Logos Zone publication
      ↓
Logos shared consensus / ordering / finality
```

The core thesis is:

> **Execution and canonical execution history belong to the Zone. Logos provides the shared foundation underneath for publishing, ordering, and finalizing that history.**
