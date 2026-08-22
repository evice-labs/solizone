# Solizone

**Solizone** is a research-oriented prototype for an **EVM-compatible Sovereign Zone** on the Logos Blockchain.

The project explores how a Logos Zone could provide a familiar Ethereum-like developer experience, allowing developers to deploy and interact with Solidity smart contracts while relying on the Logos architecture for ordering, data availability, settlement context, and interoperability.

Solizone is not an official Logos project. It is an independent research prototype inspired by the Logos Zone model, Bedrock, Mantle channels, the Logos Zone SDK, LEZ, and SPEL.

## Overview

Logos Zones are customizable, high-performance blockchains that define their own state and execution environment while relying on the Logos Blockchain for consensus and data availability.

Solizone applies this model to an EVM-specific execution environment.

The core idea is simple:

```text
Solizone executes Solidity.
Mantle does not execute Solidity.
Bedrock does not interpret Solidity.
Solizone posts ordered state updates to Logos through Mantle channels.
```

In Solizone, Solidity contracts would run inside the Zone’s own EVM runtime. The Solizone sequencer would process transactions, update the EVM state, and publish state updates or commitments to the Logos Blockchain through Mantle channels.

## High-Level Architecture

```text
Solidity Developers
        ↓
Ethereum-Compatible RPC / Tooling
        ↓
Solizone Transaction Pool
        ↓
Solizone Sequencer
        ↓
EVM Runtime
        ↓
Solizone State Database
        ↓
State Root / State Commitment
        ↓
Mantle Channel / Inscription
        ↓
Bedrock / Logos Blockchain
```

Solizone is designed as a Zone that owns its own execution logic, while Logos provides the shared foundation underneath.

## What Solizone Is

Solizone is intended to be:

* An EVM-compatible execution environment built as a Logos Zone
* A place where developers can deploy Solidity smart contracts
* A Zone with its own account, contract, and storage state
* A sequenced environment that submits ordered state updates to Bedrock through Mantle
* A research prototype for studying how EVM execution could fit into the Logos Zone model

## What Solizone Is Not

Solizone is not:

* A smart contract deployed directly on Bedrock
* A replacement for Mantle
* A replacement for LEZ
* An official Logos implementation
* A system where Mantle executes Solidity contracts
* A system where Bedrock understands EVM state internally

Mantle acts as the interaction layer between Zones and Bedrock. It provides operations such as inscriptions, channels, note transfers, bridging-related accounting, and message passing. The EVM execution itself remains inside Solizone.

## Why Solizone

The Logos documentation describes Zones as customizable execution environments that can range from applications to virtual machines hosting many applications.

This creates an interesting possibility:

```text
Can an EVM runtime itself become a Logos Zone?
```

Solizone explores that question.

The motivation is to combine two ideas:

```text
Ethereum developer experience
        +
Logos Sovereign Zone architecture
```

Developers should ideally be able to use familiar EVM tooling while Solizone handles the Logos-native side: sequencing, state commitments, channel updates, bridging, and cross-Zone messaging.

## Relationship to Logos Architecture

### Bedrock

Bedrock is the foundational layer of the Logos Blockchain. It provides the shared consensus, ordering, data availability, and settlement context that Zones build on.

For Solizone, Bedrock would not execute EVM transactions. It would provide the underlying blockchain layer where Solizone’s ordered state updates are eventually recorded.

```text
Solizone computes state.
Bedrock records and orders Solizone updates.
```

### Mantle

Mantle is the system layer that allows Zones to interact with Bedrock.

For Solizone, Mantle would be used to:

* Publish state updates through inscriptions
* Use channels for ordered Zone messages
* Support deposit and withdrawal flows
* Support token balance accounting for bridged assets
* Enable cross-Zone messaging

Mantle is not the Solidity execution engine.

### Logos Channels

A Solizone channel would act as the ordered message lane between Solizone and Bedrock.

The Solizone sequencer would publish messages such as:

```text
Solizone block #102
Previous state root: 0xabc...
New state root:      0xdef...
Batch hash:          0x123...
```

The channel helps preserve ordering of Solizone updates, even when the underlying Logos chain may temporarily fork or reorganize.

### Zones

A Zone defines its own state and execution environment.

Solizone would define:

* EVM account state
* Contract bytecode storage
* Contract storage
* Transaction execution rules
* Gas accounting model
* State commitment format
* Sequencer rules
* Bridge behavior
* Cross-Zone message interpretation

## Relationship to LEZ

The **Logos Execution Zone (LEZ)** is the flagship execution Zone in the Logos ecosystem. It provides program execution, private accounts, sequencers, validators, indexers, and zero-knowledge verification.

Solizone is conceptually similar to LEZ in that both are execution Zones.

However, their execution models are different:

```text
LEZ:
Program execution using the Logos-native execution model

Solizone:
EVM execution for Solidity smart contracts
```

LEZ is a useful architectural reference for Solizone because it shows how an execution-focused Zone can exist within the Logos ecosystem.

## Relationship to SPEL

**SPEL** is a developer framework for building programs, inspired by Anchor for Solana.

It provides developer tooling such as:

* Program macros
* IDL generation
* CLI tooling
* Project scaffolding
* Transaction submission
* Account handling
* Client generation

SPEL is not a Zone. It is a developer framework for writing programs.

For Solizone, SPEL is useful as a reference for developer experience. Solizone would focus on Solidity and EVM compatibility, but it may eventually need a similar tooling layer for developers.

A possible future direction could be:

```text
SPEL is to LEZ-style programs
what Solizone tooling could be to Solidity contracts.
```

## Possible Structure

Solizone could be structured around the following high-level components.

### 1. EVM Runtime

The EVM runtime is responsible for executing Solidity smart contracts.

It would handle:

* Contract deployment
* Contract calls
* EVM bytecode execution
* Gas calculation
* Logs and events
* Account balances
* Contract storage

This is the execution core of Solizone.

### 2. Ethereum-Compatible RPC Layer

To make Solizone familiar to developers, it should expose an Ethereum-compatible RPC interface.

This could allow common tools to work with Solizone, such as:

* MetaMask-style wallets
* Foundry
* Hardhat
* ethers.js
* viem
* block explorers
* contract deployment scripts

The goal is to make Solizone feel like an EVM chain from the developer’s point of view.

### 3. Solizone Sequencer

The sequencer receives user transactions, orders them, executes them through the EVM runtime, and produces new Solizone state.

In an MVP, Solizone can start with a single sequencer.

Later, the design can explore decentralized sequencing using Logos channels.

```text
Phase 1: Single sequencer
Phase 2: Sequencer committee
Phase 3: Round-robin or first-write-wins sequencing
```

### 4. State Database

Solizone needs its own internal state database.

This state would include:

* EVM accounts
* Contract bytecode
* Contract storage
* Nonces
* Balances
* Logs
* Receipts
* Block metadata

Bedrock does not store or understand this full internal state. Solizone maintains it independently.

### 5. State Commitment Layer

After executing transactions, Solizone should produce a state commitment.

This could be represented as:

```text
Previous state root
New state root
Batch hash
Transaction list hash
Receipt root
```

The exact commitment structure can evolve, but the purpose is to provide a compact representation of the latest Solizone state.

### 6. Mantle Channel Adapter

The Mantle channel adapter is the bridge between Solizone and Logos.

It would be responsible for submitting Solizone updates to the Logos Blockchain through Mantle operations.

Conceptually:

```text
Solizone Sequencer
        ↓
Creates state update message
        ↓
Submits inscription/channel message
        ↓
Mantle records it
        ↓
Bedrock orders/finalizes it
```

### 7. Bridge Module

Solizone should eventually support deposits and withdrawals between Bedrock and Solizone.

A deposit flow could look like:

```text
User deposits Mantle note
        ↓
Solizone channel balance increases
        ↓
Solizone mints or credits equivalent EVM asset
        ↓
User can use that asset inside Solidity contracts
```

A withdrawal flow could look like:

```text
User withdraws from Solizone
        ↓
Solizone burns or debits equivalent EVM asset
        ↓
Solizone channel balance decreases
        ↓
Mantle creates a new note on Bedrock
```

Important distinction:

```text
Bedrock tracks total value bridged into Solizone.
Solizone tracks user balances internally.
```

### 8. Cross-Zone Messaging Module

Logos channels can carry arbitrary messages.

Solizone could use this to support communication with other Zones.

Examples:

```text
Solizone → another Zone:
"Transfer asset to Zone B"

Another Zone → Solizone:
"Unlock wrapped asset for user"

Solizone → another Zone:
"Trigger coordinated state update"
```

The receiving Zone would need to interpret the message and decide how to act on it.

### 9. Indexer

Solizone would likely need an indexer to make its state readable.

The indexer could track:

* Solizone blocks
* Transactions
* Receipts
* Contract deployments
* Contract events
* User balances
* Bridging activity
* Channel messages

This would make it easier to build explorers, dashboards, wallets, and developer tools.

### 10. Correctness / Verification Layer

Solizone needs a way to prove or validate that its state transitions are correct.

This can evolve in stages.

Possible models include:

* Honest sequencer for the first prototype
* Re-execution by Solizone nodes
* Fraud-proof challenge windows
* Validity proofs
* Zero-knowledge proofs for EVM execution

For the first version, Solizone can keep this simple and focus on execution, state updates, and channel integration.

## Transaction Flow

A typical Solizone transaction could work like this:

```text
1. User signs an Ethereum-style transaction.

2. Transaction is submitted to Solizone RPC.

3. Solizone sequencer receives and orders the transaction.

4. EVM runtime executes the transaction.

5. Solizone updates its internal state.

6. Solizone produces a new state commitment.

7. Sequencer posts the state update to a Mantle channel.

8. Bedrock records and orders the update.

9. Indexer reads the update and exposes it to apps/tools.
```

## Contract Deployment Flow

```text
1. Developer writes a Solidity contract.

2. Developer compiles the contract using existing Ethereum tooling.

3. Developer deploys the contract through Solizone RPC.

4. Solizone sequencer executes the deployment transaction.

5. Contract bytecode is stored in Solizone state.

6. New Solizone state commitment is generated.

7. Sequencer publishes the update through Mantle channel.

8. Contract becomes available for interaction inside Solizone.
```

## Bridging Flow

```text
Bedrock Note
    ↓
Deposit operation
    ↓
Solizone channel balance increases
    ↓
Equivalent EVM asset credited inside Solizone
    ↓
User interacts with Solidity contracts
```

Withdrawal works in reverse:

```text
EVM asset debited inside Solizone
    ↓
Solizone channel balance decreases
    ↓
New Bedrock note created
```

## Cross-Zone Flow

```text
Solizone
    ↓
Publishes message through channel
    ↓
Bedrock / Mantle
    ↓
Another Zone reads message
    ↓
Other Zone performs corresponding state update
```

This could enable coordinated actions between Solizone and other Logos Zones.

## Current Research Findings

Based on the current Logos ecosystem exploration:

* Logos already has the concept of customizable Zones.
* Logos has a Zone SDK for building Zone sequencers and indexers.
* LEZ exists as a flagship execution Zone.
* SPEL exists as a developer framework for LEZ-style programs.
* Logos has EVM-related wallet modules, but these are wallet/RPC tooling, not an EVM Sovereign Zone.
* No official EVM-compatible Logos Zone equivalent to Solizone was found during the initial research.

This makes Solizone a useful independent research direction: combining the Logos Zone architecture with an EVM/Solidity execution environment.

## Design Philosophy

Solizone follows a separation-of-responsibilities model.

```text
Solizone:
Executes Solidity and maintains EVM state.

Mantle:
Provides operations, inscriptions, channels, notes, and bridge/message interfaces.

Bedrock:
Provides consensus, ordering, data availability, and final settlement context.
```

This keeps the design aligned with Logos:

```text
Execution belongs to the Zone.
Coordination happens through Mantle.
Consensus and data availability come from Bedrock.
```

## Project Goals

The main goals of Solizone are:

* Explore how an EVM-compatible runtime can be implemented as a Logos Zone
* Allow Solidity contracts to be deployed inside a Zone
* Design a sequencer that can execute EVM transactions and publish state updates
* Study how Mantle channels can be used for ordered Solizone updates
* Explore Bedrock-to-Solizone token bridging
* Explore cross-Zone messaging from an EVM environment
* Provide a familiar developer experience for Ethereum developers
* Use the Logos Zone model as the foundation for a new execution environment

## Non-Goals

Solizone does not aim to:

* Modify Bedrock consensus
* Make Bedrock execute EVM transactions
* Make Mantle a smart contract platform
* Replace LEZ
* Compete with SPEL
* Provide production-ready security in the first prototype
* Guarantee full Ethereum equivalence from day one

## Status

Solizone is currently an experimental research concept.

The initial focus is to define the architecture and validate whether an EVM-compatible execution environment can be modeled as a Logos Sovereign Zone.

A detailed development plan, milestone breakdown, and implementation roadmap will be prepared separately.
