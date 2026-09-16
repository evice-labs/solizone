# Solizone - Tentative Development Roadmap

**Solizone** is an independent research project exploring an **EVM-compatible Sovereign Zone on Logos Blockchain**.

The objective is to provide an Ethereum-like execution and developer experience while using Logos as the underlying shared blockchain infrastructure for Zone publication, ordering, data availability, and interoperability.

The fundamental architecture is:

```text
Ethereum Developer / User
          ↓
Ethereum-Compatible JSON-RPC
          ↓
Solizone Transaction Pool
          ↓
Solizone Block Producer / Sequencer
          ↓
EVM Runtime
          ↓
Solizone State
          ↓
Canonical Solizone Block
          ↓
Bedrock Publisher
          ↓
Logos Zone SDK
          ↓
Mantle Channel
          ↓
Logos Blockchain
```

The most important separation of responsibilities is:

```text
Solizone executes Solidity.
Solizone owns EVM state.
Mantle transports and orders Zone messages.
Logos provides the shared blockchain foundation.
Bedrock does not execute or interpret EVM transactions.
```

---

# 1. Development Philosophy

Solizone should be developed through **small experiments that remove architectural uncertainty before large components are built**.

Instead of:

```text
build complete EVM chain
        ↓
integrate Logos
        ↓
discover assumptions were wrong
```

the project follows:

```text
understand Logos
      ↓
prove publication
      ↓
define canonical Zone block
      ↓
add execution
      ↓
connect execution to publication
      ↓
add Ethereum compatibility
      ↓
expand interoperability/security
```

Each major stage should answer one concrete question.

---

# 2. Current R&D Status

```text
Experiment 001
Minimal Logos Zone lifecycle
        ✅ COMPLETE

Experiment 002
Publication & inscription constraints
        ✅ COMPLETE FOR DESIGN GATE

Experiment 003
Canonical Solizone block
        ⏭ NEXT

Experiment 004
Minimal EVM runtime
        ⏳

Experiment 005
EVM state + deterministic execution
        ⏳

Experiment 006
Solizone block production
        ⏳

Experiment 007
Ethereum-compatible RPC
        ⏳
```

The numbering can evolve, but the dependency order should remain deliberate.

---

# 3. What Has Already Been Proven

## Experiment 001 - Minimal Zone Lifecycle ✅

### Goal

Prove that an independent application can interact with the Logos Zone SDK correctly.

### Proven

```text
Connect to Logos node
        ↓
Initialize ZoneSequencer
        ↓
Recover / backfill channel
        ↓
Reach Ready
        ↓
Publish inscription
        ↓
Observe mempool
        ↓
Track channel state
        ↓
Persist checkpoint
        ↓
Restart
        ↓
Continue from previous message
```

This proved the fundamental Solizone → Logos publication path.

### Architectural Outcome

The Logos integration layer is no longer theoretical.

Solizone should therefore build around a real abstraction such as:

```text
BedrockPublisher
```

rather than treating Logos integration as something to bolt on at the end.

---

# 4. Experiment 002 - Publication Constraints ✅

### Goal

Determine the practical publication envelope before defining a Solizone block format.

### Tested

Successful mempool publication from:

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

No payload-size rejection was encountered within the tested 1 MiB range.

### Important Findings

The experiment established:

```text
Largest tested payload:
1 MiB

Initial recommended operating target:
≤ 256 KiB

Chunking required immediately:
No

Multiple pending publications:
Supported by checkpoint state

Funding:
Needs explicit management

Mempool acceptance:
Must not be confused with finalization
```

It also demonstrated that publication cost increases significantly with payload size.

### Design Consequence

Experiment 003 can initially use:

```text
1 Solizone block
        ↓
1 canonical serialization
        ↓
1 Logos inscription
```

rather than introducing chunking before it is necessary.

---

# 5. Experiment 003 - Canonical Solizone Block

**Status: NEXT**

## Goal

Define exactly what a **Solizone block** is before introducing EVM execution.

We should be able to serialize, hash, sign, publish, decode, and verify a fake Solizone block containing deterministic test data.

## Initial Structure

Something similar to:

```text
SolizoneBlock
├── version
├── chain_id
├── height
├── parent_hash
├── timestamp
├── state_root
├── transactions_root
├── receipts_root
├── gas_used
├── body
└── sequencer_signature
```

The exact structure should be determined experimentally.

## Questions to Answer

### Block Identity

What produces the canonical block hash?

```text
hash(header)
```

or:

```text
hash(full serialized block)
```

### Parent Linkage

Every block must explicitly reference:

```text
parent_block_hash
```

This is different from the Mantle channel's own parent-message relationship.

Solizone needs its **own blockchain history**.

### Serialization

Choose a deterministic binary representation.

Avoid canonicalizing through ordinary JSON.

Candidates could include:

```text
custom binary encoding
SSZ-like encoding
RLP-inspired encoding
other deterministic Rust-friendly format
```

The important requirement is:

```text
same block
   ↓
same bytes
   ↓
same hash
```

on every implementation.

### Block Body

Experiment 003 should determine whether Logos receives:

```text
header + complete transactions
```

or:

```text
header + transaction representations
```

For the first prototype, full block publication is reasonable while keeping the soft publication target below approximately 256 KiB.

## Exit Criteria

Experiment 003 is complete when:

- a deterministic `SolizoneBlock` type exists,
- blocks can be serialized/deserialized,
- block hashes are deterministic,
- parent relationships are validated,
- invalid chains are rejected,
- a block can be published through the proven Zone SDK path,
- the same block can be reconstructed from the inscription,
- publication state can be mapped back to the Solizone block.

---

# 6. Experiment 004 - Minimal EVM Runtime

Only after we know what execution must eventually produce should we introduce the EVM.

## Goal

Execute simple Ethereum-compatible transactions locally.

Initial preference remains a Rust EVM such as:

```text
revm
```

## First Target

Do not begin with complete Ethereum compatibility.

Use one simple flow:

```text
Create funded EVM account
        ↓
Deploy Counter.sol
        ↓
call increment()
        ↓
execute with EVM
        ↓
read resulting storage
        ↓
produce receipt
```

## Minimum Functionality

Support:

- EOA accounts,
- balances,
- nonces,
- contract deployment,
- bytecode execution,
- contract storage,
- gas accounting,
- transaction success/revert,
- logs,
- receipts.

## Exit Criteria

```text
Solidity contract
      ↓
compile
      ↓
EVM bytecode
      ↓
local Solizone execution
      ↓
deterministic resulting state
```

No Logos integration is required inside the EVM runtime itself.

---

# 7. Experiment 005 - Solizone State Layer

## Goal

Turn isolated EVM execution into persistent deterministic Zone state.

Solizone must own:

```text
accounts
balances
nonces
contract bytecode
contract storage
receipts
logs
block metadata
```

## MVP Storage

Start simple.

```text
key-value state
```

is acceptable for the prototype.

Do not make Ethereum's exact state trie a hard requirement unless Ethereum-equivalent roots become necessary.

## Important Interface

The EVM runtime should operate against something conceptually similar to:

```rust
trait StateBackend {
    fn account(...);
    fn storage(...);
    fn commit(...);
}
```

The execution engine should not be tightly coupled to a database implementation.

## State Commitment

Every deterministic state transition should eventually produce:

```text
previous_state_root
        ↓
transactions
        ↓
execution
        ↓
new_state_root
```

## Exit Criteria

Restarting Solizone should restore the same EVM state and continue execution deterministically.

---

# 8. Experiment 006 - Solizone Block Producer

This is where Experiments 003–005 converge.

## Goal

Turn EVM transactions into actual Solizone blocks.

```text
Transactions
     ↓
Transaction pool
     ↓
Block producer
     ↓
EVM execution
     ↓
State transition
     ↓
Receipts
     ↓
Canonical SolizoneBlock
```

## Responsibilities

The block producer should:

- select transactions,
- establish ordering,
- execute them,
- enforce block limits,
- calculate gas used,
- commit state,
- calculate transaction root,
- calculate receipt root,
- build canonical header,
- hash block,
- sign block,
- persist block.

## Important Separation

Even if the first implementation uses one process:

```text
Block Producer
```

and:

```text
Bedrock Publisher
```

should remain separate interfaces.

A block can exist locally before its publication state changes.

---

# 9. Publication Lifecycle

Solizone should explicitly model:

```text
PRODUCED
   ↓
PUBLICATION_REQUESTED
   ↓
MEMPOOL_ACCEPTED
   ↓
ADOPTED / INCLUDED
   ↓
FINALIZED
```

Never collapse these into one `confirmed` state.

Experiment 002 demonstrated why this distinction matters.

## Backpressure

The initial producer should enforce a configurable limit such as:

```text
max_pending_publications
```

instead of creating unlimited pending blocks.

This prevents the funding-note pressure observed during Experiment 002.

---

# 10. Real Logos Publisher

Unlike the old roadmap, we do **not** need a large mock-Mantle phase followed much later by real integration.

The real SDK pipeline already works.

The architecture should instead wrap the working integration behind:

```rust
trait BlockPublisher {
    async fn publish(block: CanonicalBlock) -> PublicationId;
    async fn status(id: PublicationId) -> PublicationStatus;
}
```

One implementation can use:

```text
Logos Zone SDK
```

while tests can use:

```text
MockPublisher
```

This gives us both realistic integration and easy unit testing.

---

# 11. Ethereum-Compatible JSON-RPC

Once blocks and state work locally, expose them through Ethereum APIs.

## MVP Goal

A normal Ethereum developer should ideally not need to understand Logos to use Solizone.

```text
Foundry
MetaMask-compatible wallet
ethers.js
viem
Hardhat
      ↓
Ethereum JSON-RPC
      ↓
Solizone
```

## Initial RPC Methods

Start with:

```text
eth_chainId
eth_blockNumber
eth_getBalance
eth_getTransactionCount
eth_sendRawTransaction
eth_getTransactionByHash
eth_getTransactionReceipt
eth_call
eth_estimateGas
eth_getCode
eth_getBlockByNumber
eth_getLogs
```

Do not implement the entire Ethereum RPC specification immediately.

## Main Milestone

This should work:

```bash
forge create Counter.sol ...
```

followed by a transaction calling:

```solidity
increment()
```

through the Solizone RPC.

---

# 12. Ethereum Transaction Pool

The future Solizone transaction pool is distinct from the Logos node mempool tested in Experiment 002.

```text
Ethereum user transaction
          ↓
Solizone EVM mempool
          ↓
Solizone block
          ↓
Logos publication transaction
          ↓
Logos mempool
```

These must remain separate concepts.

## Responsibilities

The Solizone mempool should eventually handle:

- sender nonce,
- signatures,
- fee ordering,
- replacement transactions,
- invalid transaction filtering,
- transaction expiry/policy,
- block selection.

For the MVP, a simple FIFO queue is sufficient.

---

# 13. Indexer and Query Layer

## Goal

Provide readable application history without requiring clients to replay every Zone message.

The indexer should track:

```text
Solizone blocks
transactions
receipts
logs
contracts
publication status
Logos message IDs
Logos transaction hashes
finalization state
```

## Important Mapping

Maintain:

```text
Solizone block hash
        ↔
Logos channel MsgId
        ↔
Logos transaction hash
```

This becomes extremely useful for debugging, explorers, and verification.

---

# 14. Bridge Prototype

Do this only after native EVM execution and block publication are stable.

## Goal

Transfer value between the Logos side and EVM-side state.

Conceptually:

```text
Bedrock/Mantle asset
        ↓
Zone deposit
        ↓
Solizone detects finalized deposit
        ↓
credit corresponding EVM balance
```

Withdrawal:

```text
EVM balance
        ↓
withdrawal request
        ↓
debit/burn inside Solizone
        ↓
publish withdrawal commitment
        ↓
Mantle/Bedrock withdrawal
```

## Core Invariant

```text
total withdrawable Solizone representation
≤
assets actually controlled/deposited by the Zone
```

This needs strong accounting before real value is involved.

---

# 15. Cross-Zone Messaging

This should remain post-MVP.

## Goal

Allow EVM contracts to interact with other Logos Zones.

A possible model:

```text
Solidity contract
      ↓
Solizone system contract / precompile
      ↓
outgoing Zone message
      ↓
Mantle / Logos
      ↓
destination Zone
```

Incoming:

```text
other Zone
    ↓
Mantle message
    ↓
Solizone
    ↓
system inbox
    ↓
EVM-visible event / execution
```

Do not expose raw Mantle internals directly to normal Solidity contracts.

A system contract or precompile would create a cleaner abstraction.

---

# 16. Correctness and Verification

The first prototype can use:

```text
single honest sequencer
```

but the architecture must not assume that model forever.

The progression could be:

### Stage A

```text
trusted sequencer
+
deterministic local execution
```

### Stage B

```text
independent Solizone nodes
+
re-execution
```

### Stage C

Potentially:

```text
challenge / fraud proof
```

or:

```text
validity proof / ZK-EVM
```

Do not commit to fraud proofs or ZK proofs until the execution/state architecture is stable.

---

# 17. Independent Solizone Node

Before decentralized sequencing, build a read/replay node.

It should be able to:

```text
read canonical Solizone blocks
        ↓
re-execute transactions
        ↓
recompute state
        ↓
verify roots
        ↓
compare against published commitment
```

This is an important trust-minimization milestone.

It also gives Solizone a real blockchain-node architecture instead of only a sequencer service.

---

# 18. Decentralized Sequencing

This should happen significantly later.

Initial MVP:

```text
1 sequencer
```

Then:

```text
sequencer committee
```

Then evaluate the sequencing mechanisms actually supported by the Logos channel stack at that time.

Possible directions may include:

```text
Round Robin
First Writer Wins
```

but the project should not hardcode either one before the Logos implementation stabilizes.

## Areas Requiring Research

- writer authorization,
- sequencer rotation,
- timeout handling,
- liveness,
- malicious sequencer removal,
- forks,
- conflicting Zone blocks,
- reorg handling,
- writer identity predictability,
- MEV,
- key rotation,
- committee updates.

This is particularly important given the writer-predictability concern around public round-robin scheduling.

---

# 19. Developer Tooling

Once the chain works, make it pleasant to use.

Possible tooling:

```text
solizone node
solizone status
solizone block <height>
solizone tx <hash>
solizone publish-status
```

Provide:

- Foundry example project,
- Hardhat example,
- ethers example,
- viem example,
- wallet configuration guide,
- local development network,
- faucet,
- contract deployment tutorial.

Eventually the goal should be:

```text
Developer writes Solidity
        ↓
points existing Ethereum tooling at Solizone RPC
        ↓
deploys normally
```

rather than requiring a custom Solidity framework.

---

# 20. Explorer

A lightweight explorer can come after the indexer.

Display:

```text
block number
block hash
parent hash
transactions
gas usage
state root
receipts
contracts
logs
publication status
Logos MsgId
Logos tx hash
```

A particularly useful concept would be showing both:

```text
Solizone status
```

and:

```text
Logos publication/finality status
```

for every block.

---

# 21. Reliability and Crash Recovery

Checkpoint behavior proven in Experiments 001 and 002 should become a permanent architectural requirement.

Test:

- process crashes before publication,
- crash after publication request,
- crash after mempool acceptance,
- crash after adoption,
- crash after finalization,
- duplicate block publication,
- RPC restart,
- state DB restart,
- node restart.

Expected invariant:

```text
restart
   ↓
recover local chain
   ↓
recover Logos publication state
   ↓
never silently duplicate canonical blocks
```

---

# 22. Security Hardening

Before any public-value network:

- fuzz block decoding,
- fuzz transaction handling,
- fuzz RPC,
- check malformed inscriptions,
- test replay attacks,
- validate chain ID,
- enforce nonce rules,
- cap block size,
- cap gas,
- bound pending queues,
- protect sequencer keys,
- separate funding keys,
- handle malicious channel messages,
- document trust assumptions.

The sequencer key and publication funding key should eventually be treated as separate operational responsibilities.

---

# 23. Performance Benchmarking

The Experiment 002 numbers are preliminary single observations.

Later benchmark:

```text
transactions / second
EVM execution latency
block production latency
serialization time
publication latency
Logos inclusion latency
finalization latency
state DB growth
RPC throughput
replay speed
```

Run multiple iterations and report distributions rather than one measurement.

---

# 24. Suggested Repository Architecture

As the prototype grows, move from isolated experiments toward a Rust workspace:

```text
solizone/
│
├── crates/
│   ├── solizone-types/
│   ├── solizone-block/
│   ├── solizone-evm/
│   ├── solizone-state/
│   ├── solizone-mempool/
│   ├── solizone-sequencer/
│   ├── solizone-publisher/
│   ├── solizone-rpc/
│   ├── solizone-indexer/
│   ├── solizone-bridge/
│   └── solizone-node/
│
├── experiments/
│   ├── minimal-zone/
│   ├── experiment-002-publication-limits/
│   ├── experiment-003-canonical-block/
│   └── ...
│
├── examples/
│   └── counter/
│
├── research/
├── docs/
└── dev-roadmap/
```

The experiment directories should remain as historical R&D evidence rather than being turned directly into production code.

---

# 25. Revised Milestone Sequence

The overall path should now look like:

```text
PHASE A - LOGOS FOUNDATION

Experiment 001
Minimal Zone lifecycle
✅

Experiment 002
Publication constraints
✅


PHASE B - BLOCKCHAIN FORMAT

Experiment 003
Canonical Solizone block
⏭

Block hashing
Serialization
Parent linkage
Publication mapping


PHASE C - EXECUTION

Experiment 004
Minimal EVM runtime

Experiment 005
Persistent EVM state


PHASE D - LOCAL SOLIZONE CHAIN

Transaction pool
Block producer
Receipts
State commitments
Local chain persistence


PHASE E - LOGOS-CONNECTED CHAIN

Bedrock Publisher
One block → one inscription
Publication lifecycle
Recovery/backpressure


PHASE F - ETHEREUM DEVELOPER EXPERIENCE

Ethereum JSON-RPC
Foundry
ethers / viem
Wallet compatibility
Solidity deployment


PHASE G - READ INFRASTRUCTURE

Indexer
Explorer
Independent replay node


PHASE H - INTEROPERABILITY

Asset bridge
Withdrawals
Cross-Zone messaging


PHASE I - TRUST MINIMIZATION

Independent verification
Re-execution
Fraud/ZK research


PHASE J - DECENTRALIZATION

Sequencer committee
Rotation
Liveness
MEV mitigation
Governance/key updates


PHASE K - HARDENING

Testing
Benchmarking
Security
Developer tooling
Public testnet
```

---

# 26. MVP Definition

The **first real MVP** is complete when:

```text
1. User has an Ethereum private key.

2. User connects standard Ethereum tooling to Solizone RPC.

3. User deploys Counter.sol.

4. Transaction enters Solizone mempool.

5. Solizone executes it using the EVM.

6. Solizone updates persistent state.

7. Sequencer creates a canonical Solizone block.

8. Block is serialized deterministically.

9. Block is published as one Logos channel inscription.

10. Solizone tracks its publication state.

11. User receives an Ethereum-compatible transaction receipt.

12. After restart, Solizone reconstructs the same chain/state.
```

At that point we have actually demonstrated:

> **A working EVM-compatible execution Zone using Logos as its underlying blockchain publication infrastructure.**

Bridge, multi-sequencer operation, cross-Zone messaging, and advanced proofs can all come afterward.

---

# 27. Near-Term Roadmap

The immediate next milestones should therefore be:

```text
NOW
│
├── Experiment 003
│   Canonical Solizone block
│
├── Experiment 004
│   Minimal REVM execution
│
├── Experiment 005
│   Persistent state transition
│
├── Experiment 006
│   Produce blocks from EVM transactions
│
├── Connect block publisher to proven Zone SDK
│
└── Ethereum JSON-RPC + Counter.sol
```

We should **not** work on bridges, decentralized sequencers, ZK proofs, or cross-Zone messaging before this path works end-to-end.

---

# 28. Final Architectural Principle

The project should preserve this separation all the way through development:

```text
Ethereum compatibility
        │
        ▼
Solizone RPC
        │
        ▼
Solizone transaction pool
        │
        ▼
Block Producer
        │
        ▼
EVM Runtime
        │
        ▼
Solizone State
        │
        ▼
Canonical Solizone Block
        │
        ▼
Bedrock Publisher
        │
        ▼
Logos Zone SDK / Mantle
        │
        ▼
Logos Blockchain
```

Each layer should have a narrow responsibility.

That gives us the ability to change the EVM engine, database, sequencer strategy, publication strategy, or RPC implementation without redesigning the entire system.

---

# Updated Status

```text
Solizone

Research foundation       ✅
Real Zone SDK lifecycle   ✅
Publication benchmark     ✅

Canonical block           🧪 NEXT
EVM runtime               ⏳
Persistent EVM state      ⏳
Block producer            ⏳
Ethereum RPC              ⏳
Indexer                    ⏳
Bridge                     ⏳
Cross-Zone messaging      ⏳
Verification              ⏳
Decentralized sequencing  ⏳
```

The biggest change from the original roadmap is that **we no longer need to postpone Logos integration until after building the EVM**.

The real Logos publication path is already experimentally proven.

So from here, the R&D path becomes much cleaner:

> **Logos publication proven → define what we publish → build the EVM that produces it → expose Ethereum tooling on top.**
