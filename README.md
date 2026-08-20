# Solizone

**Solizone** is a research-oriented prototype for an EVM-compatible Sovereign Zone on the Logos Blockchain.

The goal of Solizone is to explore whether a Logos Zone can provide an Ethereum-like developer experience, where developers can deploy Solidity smart contracts while the Zone relies on Bedrock for ordering, data availability, settlement context, and interoperability.

Solizone is not intended to make Mantle execute Solidity directly. Instead, the core idea is that Solidity contracts are executed inside Solizone’s own EVM runtime, managed by the Zone’s sequencer. After execution, the sequencer submits Zone state updates to the Logos Blockchain through Mantle channels.

## About

Logos Zones are customizable application-specific blockchains that define their own state and execution environment while relying on the Logos Blockchain for consensus and data availability.

Solizone applies this model to an EVM-specific execution environment.

In this architecture:

```text
Solidity Developers
        ↓
Solizone RPC / Tooling
        ↓
EVM Runtime inside Solizone
        ↓
Solizone Sequencer
        ↓
Mantle Channel / Inscriptions
        ↓
Bedrock / Logos Blockchain
```

Developers would interact with Solizone using familiar Ethereum tooling and Solidity contracts, while Solizone handles transaction execution, state transitions, and contract storage internally.

Bedrock does not interpret Solizone’s EVM state directly. Instead, Solizone publishes ordered state updates or commitments through Mantle, allowing the Zone to benefit from Logos’ shared consensus, data availability, and cross-Zone communication primitives.

## Objective

The objective of Solizone is to build a practical research prototype that demonstrates how an EVM-compatible execution environment could exist as a Sovereign Zone on Logos.

The project aims to explore:

* How Solidity contracts can be deployed and executed inside a Logos Zone
* How an EVM runtime can be integrated into a Zone sequencer
* How Solizone can maintain its own account, contract, and storage state
* How state updates can be submitted to Bedrock through Mantle channels
* How token bridging between Bedrock and Solizone could be represented
* How cross-Zone messaging could be supported through Logos channel messages
* How Ethereum developer tooling can be adapted for a Logos-native Zone

## Core Design Idea

Solizone separates execution from settlement.

Execution happens inside the Zone:

```text
Solidity Contract → EVM Execution → Solizone State Update
```

Settlement and coordination happen through Logos:

```text
Solizone Sequencer → Mantle Channel → Bedrock
```

This keeps Solizone aligned with the Logos architecture: the Zone owns its execution logic, while Bedrock provides the shared foundation for ordering, data availability, token bridging, and interoperability.

## Project Status

Solizone is currently an experimental concept and research prototype.

It is not an official Logos project and should be treated as an independent exploration of how an EVM-compatible Sovereign Zone could be designed on top of the Logos Blockchain architecture.
