# Solizone

**Solizone is an experimental EVM execution environment built as a Sovereign Zone on Logos.**

It lets Solidity/EVM execution remain inside Solizone while using the Logos stack for publication, ordering, data availability, and finality.

> Solizone is an independent research prototype and is **not an official Logos project**.

---

## Current status

The active implementation lives in [`solizone-evm/`](./solizone-evm).

Today, Solizone can:

- execute Solidity contracts and generic EVM transactions with REVM
- maintain account, nonce, balance, bytecode, and contract-storage state
- produce execution receipts and deterministic state / transaction / receipt commitments
- build canonical `SZB1` Solizone blocks
- produce parent-linked multi-block chains
- persist EVM state checkpoints across restarts
- persist canonical block history to disk
- recover state, chain head, and historical blocks after a full process restart
- continue execution from the recovered state
- publish canonical Solizone block bytes through the Logos Zone SDK
- observe published blocks reaching Logos finality

The current test suite passes **29 tests with 0 failures** on the `feat/second-phase` branch.

---

## Architecture

```text
Ethereum tooling
      ↓
Ethereum JSON-RPC                     (next)
      ↓
Solizone transaction pool             (next)
      ↓
BlockProducer
      ↓
REVM
      ↓
Solizone EVM state
      ↓
canonical Solizone block
      │
      ├──────── persistence ────────┐
      │                              │
      ↓                              ↓
state checkpoint                block history
      │                              │
local file backend             FileBlockStore
      │                              │
      └──── future Logos Storage ────┘

canonical Solizone block
      ↓
BlockPublisher
      ↓
Logos Zone SDK / Mantle
      ↓
Logos blockchain
      ↓
ordering / consensus / finality
```

### Responsibility split

**Solizone owns**

- EVM execution
- accounts and contract state
- transaction execution
- receipts and gas accounting
- state commitments
- block production
- canonical block history
- local execution recovery

**Logos provides**

- Zone publication
- shared ordering
- data availability
- consensus
- finality

**Logos Storage** is the next persistence experiment. The goal is to evaluate it as an additional backend for state checkpoints and canonical block history without coupling REVM directly to storage infrastructure.

---

## Repository structure

```text
solizone/
├── solizone-evm/        # active Rust implementation
├── experiments/         # Logos / Zone SDK research experiments
├── research/            # design and protocol research
├── dev-roadmap/         # development notes
├── DEVELOPMENT_ROADMAP.md
└── README.md
```

### `solizone-evm/`

The active implementation includes:

```text
REVM execution
      ↓
persistent EVM state
      ↓
BlockProducer
      ↓
parent-linked Solizone blocks
      ↓
BlockStore / FileBlockStore
      ↓
restart + recovery
      ↓
Logos publication
```

See [`solizone-evm/README.md`](./solizone-evm/README.md) for the detailed implementation status and architecture.

---

## Proven flow

```text
Solidity
   ↓
REVM execution
   ↓
state transition + receipt
   ↓
canonical Solizone block
   ↓
persistent state + block history
   ↓
Logos inscription
   ↓
Logos block inclusion
   ↓
finality
```

A complete local restart has also been tested:

```text
produce block #0
      ↓
produce block #1
      ↓
persist state + block history
      ↓
process stops
      ↓
restore state + chain head + blocks
      ↓
produce block #2
```

---

## Next milestones

1. Prototype **Logos Storage** as a persistence backend.
2. Add a long-running Solizone node lifecycle.
3. Add a transaction pool.
4. Accept raw signed Ethereum transactions.
5. Implement signature recovery and admission rules.
6. Add a minimal Ethereum-compatible JSON-RPC surface.
7. Connect familiar tooling such as Foundry, ethers, viem, and wallets.
8. Turn Logos publication into a durable node service.

---

## Run locally

```bash
cd solizone-evm

cargo check
cargo test
```

Run the execution example:

```bash
cargo run
```

Run the Logos publication harness:

```bash
cargo run --bin publish
```

See the detailed [`solizone-evm/README.md`](./solizone-evm/README.md) for publication modes, persistence tests, and architecture notes.

---

## Project thesis

```text
Execution belongs to Solizone.

Canonical execution history belongs to Solizone.

Logos provides the shared foundation for publishing,
ordering, and finalizing that history.
```

The long-term goal is to give Ethereum developers a familiar Solidity/EVM development experience while remaining native to the Logos Zone architecture.
