# solizone-evm

**The active Rust implementation of the Solizone execution layer and its current Logos publication path.**

`solizone-evm` executes Solidity contracts through REVM, packages the resulting execution data into a canonical Solizone block, publishes that block as a Logos inscription, and observes the publication reaching Logos finality.

The project has moved beyond the original REVM value-transfer proof.

> ⚠️ **Disclaimer:** Solizone is an independent research prototype and is **not an official Logos project**.

---

## Table of contents

- [What works today](#what-works-today)
- [Status](#status)
- [Current end-to-end flow](#current-end-to-end-flow)
- [Architecture boundary](#architecture-boundary)
- [Proven execution-backed publication](#proven-execution-backed-publication)
- [Logos terminology](#logos-terminology)
- [Checkpoint reconciliation](#checkpoint-reconciliation)
- [Canonical Solizone block](#canonical-solizone-block)
- [Transaction model](#transaction-model)
- [Receipts](#receipts)
- [State root](#state-root)
- [Solidity contract](#solidity-contract)
- [Repository structure](#repository-structure)
- [Module responsibilities](#module-responsibilities)
- [Dependencies](#dependencies)
- [Build, run, test](#build-run-test)
- [Block publisher CLI](#block-publisher-cli)
- [Publication lifecycle](#publication-lifecycle)
- [Relationship to earlier experiments](#relationship-to-earlier-experiments)
- [Current limitations](#current-limitations)
- [Development principle](#development-principle)
- [Roadmap](#roadmap)
- [Useful commands](#useful-commands)
- [Related documentation](#related-documentation)

---

## What works today

The current prototype can:

- execute Solidity contracts through REVM
- derive execution-backed Solizone transactions
- produce receipts
- compute state / transaction / receipt commitments
- build a canonical Solizone block
- serialize it into the `SZB1` block format
- publish that block through the Logos Zone SDK
- persist publication checkpoints
- resume interrupted publication state
- observe the resulting Logos transaction reaching finality

---

## Status

### Implemented and proven

| Area | Capability | |
| --- | --- | --- |
| Execution | REVM integration | ✅ |
| Execution | In-memory EVM state | ✅ |
| Execution | Generic EVM value transfer | ✅ |
| Execution | Solidity contract deployment | ✅ |
| Execution | Contract call | ✅ |
| Execution | Contract storage mutation | ✅ |
| Receipts | Success execution receipt | ✅ |
| Receipts | Revert execution receipt | ✅ |
| Receipts | Halt / gas-limit execution receipt | ✅ |
| Receipts | Receipt encoding | ✅ |
| Transactions | Canonical Solizone transaction encoding | ✅ |
| Transactions | Transaction hashing | ✅ |
| Transactions | Transactions root | ✅ |
| Commitments | Deterministic prototype state root | ✅ |
| Commitments | Receipts root | ✅ |
| Block | Canonical Solizone block format | ✅ |
| Block | Canonical block encoding / decoding | ✅ |
| Block | Transaction-root block validation | ✅ |
| Block | Block builder | ✅ |
| Publication | Publisher abstraction | ✅ |
| Publication | Logos publisher integration | ✅ |
| Publication | Logos Zone SDK connection | ✅ |
| Publication | Sequencer checkpoint persistence | ✅ |
| Publication | Sequencer resume / recovery | ✅ |
| Finality | Execution-backed block publication to Logos | ✅ |
| Finality | Logos block inclusion | ✅ |
| Finality | Logos LIB / finality proof | ✅ |
| Finality | Checkpoint cleanup after finalization | ✅ |

### Not implemented yet

| Area | Capability | |
| --- | --- | --- |
| State | Persistent EVM state across restarts | ⏳ |
| Ethereum tx layer | Raw signed Ethereum transactions | ⏳ |
| Ethereum tx layer | Ethereum signature recovery | ⏳ |
| Ethereum tx layer | Ethereum fee validation | ⏳ |
| Ethereum tx layer | Transaction admission rules | ⏳ |
| Node surface | Ethereum JSON-RPC | ⏳ |
| Node surface | Transaction pool | ⏳ |
| Chain | Persistent Solizone chain storage | ⏳ |
| Chain | Multi-block parent linkage | ⏳ |
| Chain | Long-running block producer | ⏳ |
| Validation | Full state re-execution validation | ⏳ |
| Validation | Full receipt re-execution validation | ⏳ |
| Compatibility | Ethereum-compatible MPT state roots | ⏳ |
| Operations | Production publisher service | ⏳ |

### What the current milestone proves

Solizone can execute EVM state transitions locally, package the resulting execution data into its own canonical block, publish that block as a Logos inscription, and observe that publication reaching Logos finality.

---

## Current end-to-end flow

```text
Forge-built Solidity bytecode
compiled by Solc 0.8.20
        ↓
REVM deployment
        ↓
Counter contract created
        ↓
increment() executed
        ↓
contract storage changes
        ↓
Solizone transactions produced
        ↓
execution receipts produced
        ↓
transactions_root
receipts_root
state_root
        ↓
canonical Solizone block
        ↓
SZB1 canonical bytes
        ↓
Logos ZoneSequencer
        ↓
Mantle channel inscription
        ↓
Logos block inclusion
        ↓
LIB / finality
```

The EVM executes **inside Solizone**. Logos does not execute the Solidity contract or interpret Solizone's EVM state.

**Solizone owns:** execution, accounts, balances, contract bytecode, contract storage, transactions, receipts, gas accounting, state commitments, block semantics.

**Logos currently provides:** publication, shared ordering, channel sequencing, consensus, data availability, finality — for the canonical Solizone block bytes.

---

## Architecture boundary

The intended architecture remains:

```text
Ethereum tooling
      ↓
Ethereum JSON-RPC
      ↓
Solizone transaction pool
      ↓
Solizone block producer
      ↓
REVM
      ↓
Solizone state
      ↓
canonical Solizone block
      ↓
BlockPublisher interface
      ↓
Logos publisher
      ↓
Logos Zone SDK
      ↓
Mantle channel
      ↓
Logos ordering / finality
```

The important design rule:

> **Keep EVM execution and Logos publication separate.**
> REVM should not know how Logos works.
> The publisher should not determine EVM execution semantics.

---

## Proven execution-backed publication

The currently reproduced canonical Solizone block:

| Field | Value |
| --- | --- |
| Solizone block height | `0` |
| Transactions | `2` |
| Gas used | `223126` |
| Payload size | `975` bytes |

**Solizone block hash**

```text
0xaafe6fe8137374388925d6fc2038da9f36b88cb7ae31c51f7f92ae3e6247e540
```

**Execution commitments**

```text
state_root
0xc98e732117ebb7054dc0ac4b79ae64095bc8b79c1bdffc2fb6e2453e0ee142b7

transactions_root
0x9edd2a6e48d3c88679a390e39dafd06aab7fb437aad4b1c657e725cc0740e77d

receipts_root
0x5f8eefd86c5f2218ae4282cc6a9b434bd7aec83a7fd7c6b374a2a68870871d9
```

**Logos Mantle transaction carrying the block**

```text
d8c72874c1df3d3a5533f32c9e404874c9541131ae9b84b588df2b2739abda5b
```

**Included in Logos block**

```text
block id
d05e25841617681c23c2114950de5ad4ef7d3ca5864a79f32f43ce175ab3cb49

slot
1318018
```

That block later became the Logos LIB, so the publication reached Logos finality.

### Proof that the actual Solizone block was published

The Logos transaction inscription begins with:

```text
53 5a 42 31
```

which is ASCII `SZB1` — the canonical Solizone block magic.

The inscription also contains the same execution commitments produced locally (`state_root`, `transactions_root`, `receipts_root`). This verifies that the Logos transaction contains the execution-backed canonical Solizone block, rather than an unrelated publication test payload.

---

## Logos terminology

### Slot

A Logos slot is a consensus-time position or opportunity in which a block may be produced:

```text
slot 1
slot 2
slot 3
slot 4
...
```

A block does not necessarily have to exist for every possible slot.

### Block number / block height

Block height counts blocks that were actually produced:

```text
block 100
block 101
block 102
```

So:

```text
slot         = consensus-time position
block height = produced-block sequence
```

They are not the same concept.

### LIB

LIB means **Last Irreversible Block** — the latest Logos block considered irreversible / finalized by the protocol.

For the proven Solizone publication:

```text
transaction d8c728...
        ↓
included in block d05e258...
        ↓
block slot 1318018
        ↓
that block became LIB
        ↓
publication finalized
```

The LIB later advanced beyond slot `1318018`. That is expected — once LIB moves forward, previously finalized blocks remain part of finalized history.

---

## Checkpoint reconciliation

`ZoneSequencer` stores a local checkpoint containing fields such as `last_msg_id`, `pending_txs`, `lib`, and `lib_slot`.

The file used by `solizone-evm` is:

```text
.state/sequencer-checkpoint.json
```

On restart:

```text
load checkpoint
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

For the first execution-backed publication the checkpoint initially contained `pending_txs = 1`, because the publication had been created locally. After Logos finalized the transaction and the sequencer caught up:

```text
lib_slot          = 1318294
pending_txs_count = 0
```

The local sequencer state successfully caught up with Logos and removed the already-finalized transaction from its pending set. That process is what we refer to here as **checkpoint reconciliation**.

### Why the LIB slot later became 1318294

The Solizone transaction finalized in the block at slot `1318018`. Later the Logos LIB advanced to `1318294`. This does not change the finality of the earlier transaction:

```text
1318018 finalized
      ↓
Logos continued producing/finalizing blocks
      ↓
LIB moved forward
      ↓
1318018 remains finalized history
```

---

## Canonical Solizone block

The block implementation lives in `src/block.rs`.

The canonical block begins with the magic `SZB1`, followed by:

- a 170-byte fixed-width header
- transaction count
- length-prefixed canonical transactions

### Block header fields

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

The block body is committed through `transactions_root`, while execution results are committed through `state_root` and `receipts_root`.

### Block validation

The current `decode_and_validate()` flow checks:

- canonical encoding
- transaction decoding
- trailing bytes
- transactions root

It does **not** yet independently re-execute transactions, so it does not yet fully prove `state_root` and `receipts_root` during import.

Full block validation will eventually become:

```text
decode block
      ↓
validate canonical structure
      ↓
validate tx commitment
      ↓
re-execute transactions
      ↓
recompute state root
      ↓
recompute receipt root
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

Transaction kind is currently `Create` or `Call(address)`.

Canonical transaction encoding begins with `SZT1`, and the transaction hash is:

```text
Keccak256(canonical transaction bytes)
```

### Example transactions

The proven block contains two Solizone transactions.

```text
contract deployment
0x0d050d4bb4e349c3091f32034787da0def0858b1fc47fa55ee94548a411bbae5

Counter increment() call
0x5a46aeb245b5957b4d817b248c8b99c04043ead20d5e6064a8c6f1804a607a2c
```

### Important limitation

These are **not** yet raw Ethereum signed transactions. Solizone currently creates execution transactions internally, and the REVM sender is provided by Solizone execution code.

Missing Ethereum transaction-layer work:

- EIP-2718 typed transaction support
- RLP decoding
- raw transaction parsing
- chain-id validation
- secp256k1 signature recovery
- sender derivation
- gas-price / fee fields
- EIP-1559 fields
- access lists
- nonce admission
- balance admission
- fee validation

---

## Receipts

Receipt implementation lives in `src/execution/receipt.rs`.

Statuses: `Success`, `Revert`, `Halt`.

A receipt contains:

```text
status
gas_used
output
contract_address
logs
```

Receipt encoding begins with `SZR1`, and receipt hashes are committed into `receipts_root`.

### Receipt root

Receipt commitment logic lives in `src/execution/receipts_root.rs`. The current root uses deterministic Solizone-specific hashing domains. This is a prototype commitment model and is **not** intended to claim Ethereum receipt-trie compatibility.

---

## State root

State commitment logic lives in `src/execution/state_root.rs`.

The current state root is a deterministic flat Solizone commitment over REVM state, committing:

- accounts
- balances
- nonces
- code hashes
- storage

It is **not** an Ethereum Merkle Patricia Trie root. That is intentional at this stage — the current priority is determinism, reproducibility, and auditability before introducing more complex proof structures.

---

## Solidity contract

The current execution milestone uses:

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

The contract is built through Forge using Solc 0.8.20 — described as *Forge-built Solidity bytecode, compiled by Solc 0.8.20*.

### Counter execution

```text
deployer funded
      ↓
Counter creation bytecode loaded
      ↓
REVM CREATE transaction
      ↓
contract deployed
      ↓
increment() calldata submitted
      ↓
REVM CALL
      ↓
count changes from 0 → 1
      ↓
event emitted
      ↓
result committed
```

Current deployed test contract:

```text
0x8F7a45eBDe059392E46A46DCc14AB24681A961Ea
```

Current known gas usage:

| Operation | Gas |
| --- | --- |
| deploy | 178299 |
| increment | 44827 |
| read | 23466 |
| revert | 21492 |
| halt | 25100 |

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
│
├── contracts-out/
│
└── src/
    ├── lib.rs
    ├── main.rs
    │
    ├── block.rs
    ├── block_builder.rs
    │
    ├── publisher.rs
    ├── logos_publisher.rs
    │
    ├── bin/
    │   └── publish.rs
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
| `src/main.rs` | Small local REVM execution example; currently demonstrates a generic EVM value transfer. |
| `src/execution/revm_engine.rs` | Main REVM integration: EOA value transfer, contract creation, contract calls, Solidity Counter execution, storage mutation, revert behavior, halt behavior, receipt generation, state inspection. |
| `src/execution/transaction.rs` | Defines the current canonical Solizone transaction model. |
| `src/execution/transactions_root.rs` | Computes deterministic transaction commitments. |
| `src/execution/receipt.rs` | Defines execution receipts and canonical receipt encoding. |
| `src/execution/receipts_root.rs` | Computes the receipt commitment. |
| `src/execution/state_root.rs` | Computes the current deterministic prototype state commitment. |
| `src/block.rs` | Defines `SolizoneBlock`, `SolizoneBlockHeader`, canonical encoding/decoding, block hash, transaction-root validation. |
| `src/block_builder.rs` | Builds a canonical block from transactions, receipts, execution state, and gas information. |
| `src/publisher.rs` | Defines the generic publication boundary: `SolizoneBlock → PublicationPayload → BlockPublisher`. |
| `src/logos_publisher.rs` | Logos-specific publisher adapter; keeps Logos-specific behavior outside the EVM execution engine. |
| `src/bin/publish.rs` | End-to-end publication harness (see below). |

The publication harness flow:

```text
executes Counter
      ↓
builds transactions
      ↓
builds receipts
      ↓
computes roots
      ↓
builds Solizone block
      ↓
serializes SZB1 bytes
      ↓
connects to Logos
      ↓
publishes inscription
      ↓
persists checkpoint
      ↓
drives ZoneSequencer
```

---

## Dependencies

```toml
revm = "43.0.2"
hex = "0.4.3"
serde_json = "1"

tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

The Logos integration currently uses local path dependencies:

```text
../../logos-blockchain-basecamp-compat/zone-sdk
../../logos-blockchain-basecamp-compat/services/key-management-system
../../logos-blockchain-basecamp-compat/zk/groth16
```

This compatibility checkout exists because the currently installed Basecamp node API differs from newer Logos source revisions. It is development infrastructure, not a final Solizone distribution strategy.

---

## Build, run, test

```bash
cd solizone-evm

cargo build        # build
cargo check        # quick compile check
cargo run          # run the simple generic transfer example in src/main.rs
```

### Tests

```bash
cargo test

# Solidity Counter execution test
cargo test executes_solidity_counter -- --nocapture
```

Current execution coverage includes:

- value transfer
- contract deployment
- contract call
- storage mutation
- receipt construction
- receipt hashing
- receipt roots
- state commitment
- transaction encoding
- transaction roots
- canonical block encoding
- canonical block validation

---

## Block publisher CLI

The current publisher harness is `src/bin/publish.rs`. It supports several modes.

### Dry run

```bash
cargo run --bin publish
```

Executes the Counter flow, builds the canonical Solizone block, and prints block data — without connecting to Logos.

### Connect only

```bash
cargo run --bin publish -- --connect
```

```text
loads checkpoint
      ↓
connects to Basecamp Logos node
      ↓
backfills
      ↓
waits until ZoneSequencer Ready
      ↓
receives live checkpoint
      ↓
exits
```

No new block is published.

### Send

```bash
cargo run --bin publish -- --send
```

The send lifecycle:

```text
build Solizone block
      ↓
convert bytes → Logos Inscription
      ↓
ZoneSequencer handle().publish()
      ↓
transaction accepted locally
      ↓
save checkpoint immediately
      ↓
continue driving next_event()
      ↓
network POST progresses
      ↓
MempoolPending
```

> **Important:** `publish()` returning does not by itself mean the Logos node has accepted the transaction. The `ZoneSequencer` drive loop must continue being polled.

### Resume

```bash
RUST_LOG=warn cargo run --bin publish -- --resume

# deeper SDK logging
RUST_LOG=debug cargo run --bin publish -- --resume
```

`--resume` does not create another inscription. It restores the existing checkpoint and continues driving the sequencer. Useful after process restart, network interruption, local application exit, or pending transaction recovery.

### Publication safety

Before creating a new publication, the harness checks whether the checkpoint already contains pending transactions:

```text
pending tx exists
        ↓
refuse --send
        ↓
use --resume
```

This prevents accidental duplicate publication while an older transaction is unresolved.

### Publisher checkpoint

```text
.state/sequencer-checkpoint.json
```

The publisher can also use the older Experiment 003 checkpoint as an initial migration source if a local checkpoint does not exist. The checkpoint stores publication continuity information such as `last_msg_id`, `pending_txs`, `lib`, `lib_slot`.

### Logos channel and node

The current prototype publishes through the same research channel established during the earlier experiments. The publisher expects the channel credentials under:

```text
../experiments/experiment-003-canonical-block/.secrets/
```

> 🔐 **The sequencer private key must never be committed.**

The local Basecamp node is currently expected at:

```text
http://127.0.0.1:8080
```

---

## Publication lifecycle

```text
Solizone block built
        ↓
Logos transaction created
        ↓
AcceptedLocally
        ↓
PendingMempool
        ↓
OnChain
        ↓
Finalized
```

These are not the same state.

| Stage | Meaning |
| --- | --- |
| **Accepted locally** | The Zone SDK has created, funded, signed and tracked the transaction. This does not yet prove node acceptance. |
| **Pending mempool** | The transaction has been successfully posted to the Logos node. |
| **On-chain** | The transaction appears in a canonical Logos block. |
| **Finalized** | The containing Logos block reaches irreversible Logos history. |

For the execution-backed Solizone block, the final state has been proven.

### Current finalized checkpoint state

After restarting and allowing the `ZoneSequencer` to reconcile with Logos history, checkpoint pending transactions and `pending_publish_txs` were both `0`, and the saved checkpoint contained:

```json
{
  "lib_slot": 1318294,
  "pending_txs_count": 0,
  "pending_tx_hashes": []
}
```

This is the expected post-finalization state.

---

## Relationship to earlier experiments

The earlier experiments remain important research evidence.

### Experiment 001

Validated Logos Zone SDK connectivity, channel behavior, sequencer identity, checkpoint behavior, opaque publication, ordering continuity.

### Experiment 002

Validated publication-size behavior and explored inscription constraints. The important protocol-side bound discovered from Logos source was:

```text
MAX_BLOCK_TRANSACTIONS_SIZE = 2,097,152 bytes

MAX_BYTES = MAX_BLOCK_TRANSACTIONS_SIZE * 7 / 8
          = 1,835,008 bytes
```

for the inscription upper bound in that implementation. The successful tests did **not** establish 1 MiB as a protocol maximum.

### Experiment 003

Validated the canonical Solizone block format, block publication, Logos inclusion, and finality. The original Experiment 003 block still used synthetic transaction data, a placeholder state root, and a placeholder receipt root.

### Difference between Experiment 003 and current solizone-evm

```text
before:
synthetic txs
placeholder state
placeholder receipts

now:
REVM execution
      ↓
real Solizone txs
      ↓
real execution receipts
      ↓
execution-derived state commitment
      ↓
execution-backed canonical block
```

This is the important bridge between the research experiments and the active execution implementation.

---

## Current limitations

**EVM state is still temporary.** Current REVM state exists in memory. Restarting the process does not restore the Solizone EVM state. Persistent state is one of the next major milestones.

**The published block is currently a demonstration block.** It uses `height = 0` and a zero `parent_hash`. A real chain needs:

```text
Block N
      ↓
hash
      ↓
Block N+1.parent_hash
```

**Transactions are not Ethereum-wallet transactions.** There is not yet support for MetaMask, Foundry `cast send`, or `eth_sendRawTransaction` against Solizone.

**State root is prototype-specific.** It provides deterministic commitment but not Ethereum state-trie compatibility.

**Receipts are Solizone-specific.** They use Solizone's canonical encoding and are not yet Ethereum JSON-RPC receipt objects.

**Logos publication is still an R&D harness.** `src/bin/publish.rs` is not yet a daemon or production block-publishing service.

---

## Development principle

Keep the following boundaries separate, even if one process currently performs all of them:

- Execution engine
- Block producer
- Publisher
- Logos integration

Long-term:

```text
BlockProducer
      ↓
produces SolizoneBlock

BlockPublisher
      ↓
publishes SolizoneBlock
```

The block producer should not care whether publication eventually uses Logos, a mock publisher, test storage, or another DA layer.

---

## Roadmap

### Immediate next steps

1. Persist Solizone EVM state across restarts
2. Add persistent Solizone chain storage
3. Add parent-linked multi-block production
4. Introduce a real `BlockProducer` abstraction
5. Introduce a transaction pool
6. Accept raw signed Ethereum transactions
7. Implement secp256k1 signature recovery
8. Add nonce and balance admission rules
9. Add Ethereum fee handling
10. Add Ethereum JSON-RPC
11. Re-execute imported blocks
12. Verify `state_root` during import
13. Verify `receipts_root` during import
14. Turn Logos publication into a durable publisher service
15. Add restart / reorg / multi-block integration tests

### Next major target

Move from a single deterministic demo block to a persistent Solizone chain:

```text
Block 0
  hash H0
      ↓
Block 1
  parent_hash = H0
  hash H1
      ↓
Block 2
  parent_hash = H1
  hash H2
```

with state advancing:

```text
State 0
   ↓ execute txs
State 1
   ↓ execute txs
State 2
```

and every resulting canonical block published independently to Logos.

### Future Ethereum tooling flow

```text
Foundry / MetaMask / ethers / viem
        ↓
Ethereum JSON-RPC
        ↓
eth_sendRawTransaction
        ↓
Solizone tx decoder
        ↓
signature recovery
        ↓
mempool
        ↓
block producer
        ↓
REVM
        ↓
persistent Solizone state
        ↓
canonical Solizone block
        ↓
Logos publication
        ↓
Logos finality
```

### Current goal

The current research proof is:

```text
Solidity
      ↓
REVM
      ↓
Solizone execution
      ↓
canonical Solizone block
      ↓
Logos inscription
      ↓
Logos finality
```

The next phase is:

```text
Ethereum signed transaction
        ↓
Solizone admission / mempool
        ↓
REVM execution
        ↓
persistent state
        ↓
parent-linked Solizone blocks
        ↓
durable Logos publication
        ↓
Logos finality
```

That moves Solizone from a reproducible execution-and-publication prototype toward an actual EVM-compatible Sovereign Zone runtime.

---

## Useful commands

```bash
# enter the crate
cd solizone-evm

# compile / build
cargo check
cargo build

# run local EVM example
cargo run

# tests
cargo test
cargo test executes_solidity_counter -- --nocapture

# build canonical publication block locally
cargo run --bin publish

# connect without publishing
cargo run --bin publish -- --connect

# publish a new block
cargo run --bin publish -- --send

# resume existing publication
RUST_LOG=warn cargo run --bin publish -- --resume

# debug Logos SDK behavior
RUST_LOG=debug cargo run --bin publish -- --resume

# formatting and lint
cargo fmt
cargo fmt -- --check
cargo clippy --all-targets
```

Inspect the checkpoint:

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
| Architecture / EVM design | `../SOLIZONE_EVM_README.md` |
| Development roadmap | `../DEVELOPMENT_ROADMAP.md` |
| Experiment 001 | `../experiments/minimal-zone/` |
| Experiment 002 | `../experiments/experiment-002-publication-limits/` |
| Experiment 003 | `../experiments/experiment-003-canonical-block/` |

---

*Solizone is an independent research prototype and is not an official Logos project.*