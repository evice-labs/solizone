# Solizone Tentative Development Roadmap (outdated: refer to v2)

**Solizone** is a tentative research and development project for building an **EVM-compatible Sovereign Zone** on the Logos Blockchain.

The goal is to explore whether a Logos Zone can provide a familiar Ethereum-style developer experience, where developers can deploy Solidity smart contracts while the Zone relies on Logos/Bedrock for ordering, data availability, settlement context, token bridging, and cross-Zone messaging.

This document outlines a **tentative development roadmap** and highlights the key research questions that should be answered before building each major part of the system.

## Current Understanding

Solizone is based on the following architectural assumption:

```text
Solidity contracts execute inside Solizone.
Mantle does not execute Solidity.
Bedrock does not interpret EVM state.
Solizone publishes ordered state updates to Bedrock through Mantle channels.
```

The high-level model is:

```text
User / Developer
      ↓
Ethereum-compatible RPC
      ↓
Solizone Sequencer
      ↓
EVM Runtime
      ↓
Solizone State Database
      ↓
State Commitment
      ↓
Mantle Channel
      ↓
Bedrock / Logos Blockchain
```

## Development Strategy

Solizone should not begin by directly integrating everything with Logos.

The safest approach is:

```text
1. Build a local EVM Zone prototype.
2. Add a simple sequencer.
3. Generate state commitments.
4. Mock Mantle/channel submission.
5. Integrate the Logos Zone SDK.
6. Add bridge and cross-Zone logic later.
```

This keeps the project realistic and prevents the first version from becoming too complex.

## Phase 0: Research Before Building

Before writing major code, the following areas need to be researched properly.

### 1. Logos Zone SDK

Research:

* How the Zone SDK creates or connects to a channel
* How a sequencer publishes inscriptions
* How pending and finalized channel state is tracked
* How deposits and withdrawals are represented
* How sequencer checkpoints are stored
* What data format is expected for channel messages
* Whether the SDK is stable enough to build on directly

Why this matters:

The Zone SDK is likely the main integration point between Solizone and Logos. If this is not understood first, the architecture may need to be rewritten later.

### 2. Mantle Channels

Research:

* How channels are created
* How sequencer keys are registered
* How channel messages are ordered
* How inscription payloads are structured
* Whether there are payload size limits
* How finalized vs non-finalized messages are exposed
* How round-robin sequencing works
* How first-write-wins sequencing works

Why this matters:

Solizone will rely on a channel to publish EVM state updates. The channel is the bridge between Solizone execution and Bedrock ordering.

### 3. State Commitment Format

Research:

* What should be posted to the channel
* Whether Solizone should publish full transaction batches, state roots, receipt roots, or only commitments
* How users or validators can reconstruct state
* Whether the data should follow Ethereum block structure or a custom Solizone format
* How much data should be stored on Bedrock vs stored off-chain

Possible state update format:

```text
SolizoneBlock {
  block_number
  previous_state_root
  new_state_root
  tx_batch_hash
  receipt_root
  timestamp
  sequencer_signature
}
```

Why this matters:

The state commitment format becomes the core record of Solizone’s history.

### 4. EVM Runtime Choice

Research:

* Which EVM engine to use
* Whether to use Rust-based `revm`
* Whether to use an existing Ethereum execution stack
* How to manage gas accounting
* How to store account state
* How to support contract deployment
* How close Solizone should be to Ethereum compatibility

Initial preference:

```text
Use a Rust EVM implementation such as revm for the first prototype.
```

Why this matters:

The EVM runtime is the heart of Solizone. Choosing the wrong execution engine early can slow the whole project.

### 5. State Database

Research:

* How Ethereum-like account state should be stored
* Whether to use Merkle Patricia Trie compatibility
* Whether to start with a simpler key-value database
* How state roots should be generated
* How contract storage should be persisted
* How historical states should be queried

Initial MVP approach:

```text
Start with a simple local state database.
Add Ethereum-compatible state root logic later.
```

Why this matters:

Solizone needs a reliable way to track accounts, balances, nonces, contracts, and storage.

### 6. Ethereum RPC Compatibility

Research:

* Which RPC methods are required for the MVP
* What wallets/tools need to work first
* How to support `eth_sendRawTransaction`
* How to support `eth_call`
* How to support `eth_getBalance`
* How to support `eth_getTransactionReceipt`
* How to support contract deployment tools like Foundry or Hardhat

MVP target:

```text
Support enough RPC methods to deploy and call a simple Solidity contract.
```

Why this matters:

Solizone’s main value is Ethereum-like developer experience.

### 7. Token Bridging

Research:

* How Bedrock notes are deposited into a channel
* How channel balances are updated
* How withdrawals recreate notes on Bedrock
* How Solizone should mint or credit equivalent EVM assets
* How to prevent withdrawals from exceeding deposits
* Whether bridged assets should appear as native balance or ERC-20 tokens inside Solizone

Initial MVP approach:

```text
Mock bridge first.
Then integrate real Bedrock note/channel balance logic.
```

Why this matters:

Bedrock only tracks total value deposited into the Zone. Solizone must correctly track individual user balances internally.

### 8. Cross-Zone Messaging

Research:

* How arbitrary channel messages are formatted
* How another Zone identifies a message intended for it
* Whether Logos has a suggested message standard
* How atomic token transfers between Zones are represented
* How Solizone should expose cross-Zone messages to Solidity contracts

Possible future model:

```text
Solidity contract emits cross-zone request
        ↓
Solizone sequencer converts it into channel message
        ↓
Other Zone reads and reacts
```

Why this matters:

Cross-Zone messaging is not required for the first MVP, but it could become one of Solizone’s strongest features later.

### 9. Correctness and Verification

Research:

* Should Solizone start with an honest sequencer model?
* Can Solizone nodes re-execute EVM transactions?
* Should fraud proofs be introduced later?
* Is ZK proof generation for EVM execution realistic?
* What would validators verify?
* How long should a challenge window be if fraud proofs are used?

MVP approach:

```text
Start with honest sequencer + reproducible state updates.
Later explore re-execution, fraud proofs, or ZK proofs.
```

Why this matters:

A Zone needs a way to convince users that its state transitions are valid.

### 10. Relationship with LEZ and SPEL

Research:

* What architectural lessons can be reused from LEZ
* How LEZ handles programs, accounts, sequencers, validators, and indexers
* What developer-experience lessons can be learned from SPEL
* Whether SPEL patterns like IDL generation and CLI tooling can inspire Solizone tooling

Important distinction:

```text
LEZ = Logos-native execution Zone
SPEL = developer framework for LEZ-style programs
Solizone = proposed EVM-compatible execution Zone
```

Why this matters:

Solizone should not duplicate LEZ or SPEL. It should learn from them and focus specifically on EVM/Solidity compatibility.

## Tentative Development Roadmap

## Phase 1: Project Setup

Goal:

Create the initial project structure and development environment.

Tasks:

* Create repository structure
* Choose primary language, likely Rust
* Set up workspace
* Add basic documentation
* Add local test scripts
* Add formatting and linting
* Add initial architecture notes

Possible structure:

```text
solizone/
├── README.md
├── docs/
│   ├── architecture.md
│   ├── research-notes.md
│   └── roadmap.md
├── crates/
│   ├── solizone-evm/
│   ├── solizone-sequencer/
│   ├── solizone-rpc/
│   ├── solizone-state/
│   ├── solizone-channel/
│   ├── solizone-indexer/
│   └── solizone-types/
└── examples/
    └── simple-counter/
```

Research required before building:

* Confirm language/runtime choice
* Confirm EVM engine choice
* Review Logos Zone SDK structure
* Decide what should be mocked in the first version

Expected result:

A clean repository with a basic Solizone architecture skeleton.

## Phase 2: Local EVM Runtime Prototype

Goal:

Run Solidity/EVM transactions locally without Logos integration.

Tasks:

* Integrate EVM runtime
* Create basic account model
* Support contract deployment
* Support contract calls
* Store account state locally
* Generate transaction receipts
* Add simple tests using a basic Solidity contract

Example target:

```text
Deploy Counter.sol
Call increment()
Read counter value
Generate receipt
Update local state
```

Research required before building:

* EVM runtime API
* Gas model
* Contract storage model
* Ethereum transaction format
* Signing and account recovery

Expected result:

A local EVM execution engine that can deploy and execute simple Solidity contracts.

## Phase 3: Solizone State Layer

Goal:

Create a state system that tracks Solizone’s EVM state.

Tasks:

* Store accounts
* Store balances
* Store nonces
* Store contract bytecode
* Store contract storage
* Generate basic state root
* Persist state to disk
* Add state snapshot support

Research required before building:

* Whether to use Ethereum-compatible trie structure
* Whether simple key-value storage is enough for MVP
* How state roots should be calculated
* Whether state needs to be replayable from transaction batches

Expected result:

Solizone can maintain its own independent EVM state.

## Phase 4: Single Sequencer

Goal:

Build the first Solizone sequencer.

Tasks:

* Accept signed EVM transactions
* Order transactions
* Execute transaction batches
* Produce Solizone blocks
* Update state after each block
* Generate state commitments
* Sign block commitments
* Store sequencer history

Example block:

```text
SolizoneBlock {
  number: 1
  previous_state_root: 0x000...
  new_state_root: 0xabc...
  tx_batch_hash: 0x123...
  receipt_root: 0x456...
  sequencer_signature: 0x789...
}
```

Research required before building:

* Batch format
* State commitment format
* Sequencer signing format
* How much Ethereum block compatibility is needed
* How block timestamps and gas limits should work

Expected result:

A working local Solizone chain with a single sequencer.

## Phase 5: Ethereum-Compatible RPC Layer

Goal:

Allow developers to interact with Solizone using Ethereum-style tooling.

Tasks:

* Implement basic JSON-RPC server
* Support `eth_chainId`
* Support `eth_blockNumber`
* Support `eth_getBalance`
* Support `eth_getTransactionCount`
* Support `eth_sendRawTransaction`
* Support `eth_call`
* Support `eth_getTransactionReceipt`
* Support `eth_getCode`
* Support contract deployment from scripts

Research required before building:

* Minimum RPC methods required by Foundry
* Minimum RPC methods required by Hardhat
* Minimum RPC methods required by wallets
* Chain ID design
* Transaction encoding and signing compatibility

Expected result:

A developer can deploy and call a Solidity contract through standard Ethereum tooling.

## Phase 6: Mock Mantle Channel Integration

Goal:

Simulate how Solizone would publish state updates to Mantle channels.

Tasks:

* Create local mock channel interface
* Submit Solizone block commitments to mock channel
* Store ordered channel messages
* Simulate finalized and pending states
* Add replay from channel messages
* Add tests for ordering

Research required before building:

* Logos channel message structure
* Inscription format
* Finalized vs pending channel behavior
* How state updates should be replayed by indexers

Expected result:

Solizone can behave as if it is posting state updates to a Logos channel, without requiring live Logos integration yet.

## Phase 7: Real Zone SDK / Mantle Channel Integration

Goal:

Connect Solizone to the real Logos Zone SDK and Mantle channel flow.

Tasks:

* Integrate Logos Zone SDK
* Connect to a Logos node
* Initialize or attach to a channel
* Use sequencer key
* Publish Solizone state updates as inscriptions/channel messages
* Track pending and finalized channel state
* Persist sequencer checkpoints
* Handle channel events

Research required before building:

* Exact SDK setup flow
* Funding config requirements
* Channel ID handling
* Sequencer key registration
* Inscriptions API
* Error handling and retries
* Local testnet/devnet availability

Expected result:

Solizone can publish real state updates to a Logos channel.

## Phase 8: Indexer

Goal:

Make Solizone data readable and queryable.

Tasks:

* Read Solizone channel messages
* Decode Solizone blocks
* Track finalized state updates
* Store transactions and receipts
* Store contract deployments
* Store events/logs
* Expose query API
* Support simple block explorer data

Research required before building:

* How finalized channel messages are retrieved
* Whether the Zone SDK can be used in read-only mode
* Indexer database choice
* Event/log format compatibility with Ethereum tooling

Expected result:

Solizone has a readable history of blocks, transactions, contracts, and events.

## Phase 9: Bridge Prototype

Goal:

Create the first Bedrock-to-Solizone asset flow.

Tasks:

* Design internal wrapped asset model
* Detect deposit messages
* Credit user balance inside Solizone
* Support withdrawal request inside Solizone
* Debit or burn user balance
* Produce withdrawal message
* Track channel balance assumptions
* Add safety checks

Research required before building:

* Bedrock note model
* Deposit operation format
* Withdrawal operation format
* Channel balance accounting
* How metadata should map a deposit to an EVM address
* How Solizone proves a withdrawal is valid

Expected result:

A basic deposit/withdrawal flow between Bedrock and Solizone.

## Phase 10: Cross-Zone Messaging

Goal:

Allow Solizone to send and receive messages from other Zones.

Tasks:

* Define Solizone message format
* Read incoming channel messages
* Route messages to Solizone modules
* Expose cross-Zone message events to contracts
* Allow contracts to request outgoing messages
* Add simple cross-Zone transfer simulation

Research required before building:

* Logos suggested message standard
* Atomic token transfer structure
* Message targeting format
* Sequencer coordination requirements
* Whether Solidity contracts should directly access cross-Zone messages or use a precompile/system contract

Expected result:

Solizone can participate in basic cross-Zone communication.

## Phase 11: Correctness and Verification

Goal:

Move beyond trusted sequencer assumptions.

Tasks:

* Add deterministic re-execution support
* Publish transaction batches for verification
* Allow independent Solizone nodes to replay state
* Add fraud-proof research prototype
* Explore ZK proof feasibility
* Define validator responsibilities
* Document security assumptions clearly

Research required before building:

* Whether EVM fraud proofs are realistic for Solizone
* Whether ZK-EVM proof generation is practical
* Whether simple re-execution is enough for early version
* How disputes would be represented on Logos
* What data must be available for verification

Expected result:

Solizone has a clearer path toward trust minimization.

## Phase 12: Decentralized Sequencing

Goal:

Reduce dependency on a single Solizone sequencer.

Tasks:

* Add multiple sequencer support
* Research round-robin sequencing
* Research first-write-wins sequencing
* Add sequencer timeout handling
* Add sequencer set update logic
* Add threshold signing for sequencer list changes
* Add MEV-risk documentation

Research required before building:

* How Logos channels manage sequencer sets
* How automated timeout works
* How sequencer turns are enforced
* How malicious sequencers are removed
* How state conflicts are resolved

Expected result:

Solizone can move toward a more decentralized sequencer model.

## Phase 13: Developer Tooling

Goal:

Make Solizone easier for developers to use.

Tasks:

* Add starter Solidity project
* Add deployment examples
* Add Foundry template
* Add Hardhat template
* Add CLI for local Solizone node
* Add faucet or test account tooling
* Add contract interaction examples
* Add documentation for developers

Research required before building:

* Which Ethereum tools should be supported first
* Whether custom chain config is enough for wallets
* How users should get test assets
* Whether Solizone needs its own SDK
* What developer workflow should look like

Expected result:

Developers can deploy and test Solidity contracts on Solizone with minimal friction.

## Phase 14: Explorer / Dashboard

Goal:

Create a simple interface for viewing Solizone activity.

Tasks:

* Show latest Solizone blocks
* Show transactions
* Show contract deployments
* Show bridge activity
* Show channel message status
* Show pending vs finalized updates
* Show sequencer status

Research required before building:

* What data the indexer exposes
* How to display Bedrock finality status
* How to map Solizone blocks to channel messages
* Whether to support contract verification later

Expected result:

A simple Solizone explorer for developers and testers.

## Research Priority List

Before serious development begins, research should happen in this order:

```text
1. Logos Zone SDK
2. Mantle channels
3. Inscription/message format
4. EVM runtime choice
5. Solizone state commitment design
6. Ethereum RPC compatibility requirements
7. Deposit/withdrawal mechanics
8. Cross-Zone messaging standard
9. Correctness model
10. Decentralized sequencing model
```

## MVP Scope

The first MVP should be intentionally limited.

MVP should include:

* Local EVM execution
* Solidity contract deployment
* Single sequencer
* Local state database
* Basic Ethereum RPC
* State commitment generation
* Mock channel submission
* Basic documentation

MVP should not include:

* Real bridging
* Full decentralized sequencing
* ZK proofs
* Fraud proofs
* Full Ethereum equivalence
* Production security
* Complex cross-Zone messaging

MVP target:

```text
Deploy a Solidity contract to Solizone,
execute a transaction,
generate a state commitment,
and publish/mock-publish that update as a Zone channel message.
```

## First Demonstration Goal

The first useful demo should show:

```text
1. Start Solizone local node.
2. Deploy Counter.sol using Ethereum tooling.
3. Call increment().
4. Solizone sequencer executes the transaction.
5. New EVM state root is generated.
6. State update is published to mock Mantle channel.
7. Indexer shows the Solizone block and transaction.
```

This demo proves the core idea before adding complex Logos integrations.

## Open Questions

The project still needs answers to the following questions:

* What exact format should Solizone state updates use?
* Should Solizone publish full transaction batches or only commitments?
* Should Solizone be fully Ethereum-compatible or only Solidity-compatible?
* Should bridged Bedrock value appear as native gas token or wrapped ERC-20?
* How should an EVM address map to Bedrock note ownership?
* Should Solizone use Ethereum’s trie structure from the beginning?
* What is the simplest safe correctness model?
* Can the Logos Zone SDK support the required channel workflow today?
* How should cross-Zone messages be exposed to Solidity contracts?
* What should be the long-term trust model: sequencer trust, re-execution, fraud proofs, or ZK proofs?

## Suggested Build Order

```text
Research
  ↓
Local EVM runtime
  ↓
Single sequencer
  ↓
State database
  ↓
State commitments
  ↓
Ethereum RPC
  ↓
Mock channel
  ↓
Real Zone SDK integration
  ↓
Indexer
  ↓
Bridge prototype
  ↓
Cross-Zone messaging
  ↓
Verification model
  ↓
Decentralized sequencing
```

## Development Principle

Solizone should be built in layers.

Each layer should work independently before the next layer is added.

```text
Do not start with full Logos integration.
Do not start with decentralized sequencing.
Do not start with ZK proofs.
Do not start with bridge complexity.

Start with EVM execution + sequencer + state commitment.
```

## Final Direction

Solizone is a research prototype for answering one main question:

```text
Can an EVM-compatible execution environment be built as a Logos Sovereign Zone?
```

The development roadmap should prove this step by step.

The first version should stay simple, local, and testable. Once the core EVM Zone model works, Solizone can gradually integrate Mantle channels, Bedrock settlement, token bridging, cross-Zone messaging, and decentralized sequencing.
