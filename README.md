# Solizone

**Solizone is an experimental EVM execution environment built as a Sovereign Zone on Logos.**

It keeps Solidity / EVM execution inside Solizone while using the Logos stack for publication, ordering, finality, and distributed persistence.

> Solizone is an independent research project and is **not an official Logos project**.

---

## Current status

The active implementation lives in [`solizone-evm/`](./solizone-evm).

Today, Solizone can:

- execute Solidity contracts and generic EVM transactions with REVM
- maintain persistent EVM account and contract state
- produce receipts and deterministic state / transaction / receipt commitments
- build canonical `SZB1` blocks and parent-linked block chains
- run as a long-running Solizone node
- persist execution checkpoints through Logos Storage
- recover EVM state and chain head after a full process restart
- continue block production from the recovered height
- keep recent canonical blocks locally through `HybridBlockStore`
- automatically archive older blocks into Logos Storage
- prune archived local blocks while keeping historical data recoverable
- validate and hash-check blocks recovered from Logos Storage
- publish canonical Solizone block bytes through the Logos Zone SDK
- observe published blocks reaching Logos finality

---

## Architecture

```text
Ethereum tooling
      ↓
Ethereum JSON-RPC                     (next phase)
      ↓
Solizone transaction pool             (next phase)
      ↓
BlockProducer
      ↓
REVM
      ↓
Solizone EVM state
      ↓
canonical Solizone block
      │
      ├───────────────┐
      │               │
      ↓               ↓
checkpoint      HybridBlockStore
      │          │           │
      │          ↓           ↓
      │      recent blocks   older blocks
      │        local         Logos Storage
      │
      ↓
Logos Storage

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

- EVM execution and state
- transaction execution and receipts
- state commitments
- block production
- canonical block history
- checkpointing and recovery
- local retention and historical block verification

**Logos Storage provides**

- execution-checkpoint persistence
- historical canonical block storage
- content-addressed retrieval

**Logos blockchain provides**

- Zone publication
- shared ordering
- consensus
- finality

Logos Storage does **not** determine canonical history. Recovered blocks are validated and checked against their recorded canonical hashes.

---

## Hybrid Storage Layer

Solizone now uses a hybrid storage model:

```text
HybridBlockStore
├── recent blocks → local FileBlockStore
└── older blocks  → Logos Storage
                    via height → hash → CID
```

A configurable retention window keeps local historical storage bounded while older canonical blocks remain retrievable and verifiable.

On restart, Solizone can:

```text
download latest checkpoint
        ↓
restore EVM state
        ↓
restore chain head
        ↓
verify stored head block
        ↓
continue producing blocks
```

---

## Repository structure

```text
solizone/
├── solizone-evm/        # active Rust implementation
├── experiments/         # Logos / Zone SDK research
├── research/            # design and protocol research
├── dev-roadmap/         # development notes
├── DEVELOPMENT_ROADMAP.md
└── README.md
```

See [`solizone-evm/README.md`](./solizone-evm/README.md) for the detailed implementation status and architecture.

---

## Proven flow

```text
Solidity / EVM transaction
      ↓
REVM execution
      ↓
state transition + receipt
      ↓
canonical Solizone block
      ↓
HybridBlockStore
      ├── recent history → local
      └── older history → Logos Storage
      ↓
execution checkpoint → Logos Storage
```

The publication path remains separate:

```text
canonical Solizone block
      ↓
Logos Zone SDK / Mantle
      ↓
Logos blockchain
      ↓
ordering / consensus / finality
```

---

## Next milestones

1. Design raw signed Ethereum transaction ingestion.
2. Add a Solizone transaction pool.
3. Implement signature recovery and transaction admission rules.
4. Add a minimal Ethereum-compatible JSON-RPC surface.
5. Connect Foundry, ethers, viem, and wallets.
6. Add independent block re-execution / validation.
7. Continue hardening the node, storage, and Logos publication paths.

---

## Run locally

```bash
cd solizone-evm

cargo check
cargo test
```

Run the long-running Solizone node:

```bash
cargo run --bin solizone_node
```

Reset the local node runtime state:

```bash
cargo run --bin solizone_node -- reset
```

Run the Logos publication harness:

```bash
cargo run --bin publish
```

---

## Project thesis

```text
Execution belongs to Solizone.

Recent execution data stays close to the node.

Historical execution data remains accessible
through Logos Storage.

Logos provides the shared foundation for publishing,
ordering, consensus, and finality.
```

The long-term goal is to give Ethereum developers a familiar Solidity / EVM development experience while remaining native to the Logos architecture.
