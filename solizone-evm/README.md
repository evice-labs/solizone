# Solizone EVM - Implementation Guide

`solizone-evm` contains the active Rust implementation of the Solizone EVM execution layer.

This README is intentionally implementation-focused. For the wider architecture, experiments, and long-term roadmap, see the project-level documentation in the repository root:

```text
../SOLIZONE_EVM_README.md
../DEVELOPMENT_ROADMAP.md
```

> Solizone is an independent research prototype and is not an official Logos project.

---

## Current Status

Implemented:

```text
REVM dependency                     ✅
In-memory EVM state                 ✅
Funded test account                 ✅
EVM value-transfer transaction      ✅
Execution-result inspection         ✅
Resulting account-state inspection  ✅
```

Not implemented yet:

```text
Persistent state                    ⏳
Real signed Ethereum transactions   ⏳
Solidity contract deployment        ⏳
Contract calls / storage            ⏳
Transaction receipts abstraction    ⏳
Solizone block production           ⏳
Logos publisher integration         ⏳
Ethereum JSON-RPC                   ⏳
```

The current implementation proves one deliberately small thing:

> Solizone can execute a real EVM state transition locally through REVM.

---

## Current Execution Flow

The current `src/main.rs` performs a minimal value transfer:

```text
Alice starts with 1,000,000 units
        ↓
Alice sends 100 units to Bob
        ↓
REVM executes the transaction
        ↓
REVM returns execution result + changed state
        ↓
Alice's resulting balance/nonce are inspected
        ↓
Bob's resulting balance is inspected
```

The current prototype uses:

- REVM's `InMemoryDB`,
- REVM fixed benchmark addresses for Alice and Bob,
- a `21,000` gas limit,
- zero gas price,
- direct construction of `TxEnv`.

These are development choices, not final Solizone protocol rules.

---

## Repository Location

```text
solizone/
├── README.md
├── SOLIZONE_EVM_README.md
├── DEVELOPMENT_ROADMAP.md
│
├── experiments/
│   ├── minimal-zone/
│   ├── experiment-002-publication-limits/
│   └── experiment-003-canonical-block/
│
└── solizone-evm/
    ├── Cargo.toml
    ├── Cargo.lock
    ├── README.md
    └── src/
        └── main.rs
```

`experiments/` preserves the R&D work that validated the Logos-facing side.

`solizone-evm/` is the active execution-layer implementation.

---

## Requirements

You need a Rust toolchain with support for Rust Edition 2024.

Check your environment:

```bash
rustc --version
cargo --version
```

---

## Dependency

The current EVM engine is:

```toml
revm = "43.0.2"
```

The crate currently has no direct dependency on the Logos Zone SDK.

That separation is intentional.

At this stage:

```text
solizone-evm
    = local EVM execution work
```

while the already-proven Logos publishing code remains in the earlier experiments until it is extracted into a reusable publisher component.

---

## Build

From the repository root:

```bash
cd solizone-evm
cargo build
```

Or, if you are already inside `solizone-evm/`:

```bash
cargo build
```

---

## Check Without Running

```bash
cargo check
```

This is the fastest command for verifying that the crate still compiles while developing.

---

## Run

```bash
cargo run
```

The program will:

1. create an in-memory EVM database,
2. fund Alice,
3. build an EVM value-transfer transaction,
4. execute it through REVM,
5. print the execution result,
6. print Alice's resulting balance and nonce,
7. print Bob's resulting balance.

A successful run ends with:

```text
✅ First Solizone EVM transaction executed
```

---

## Current Code

The current implementation lives in:

```text
src/main.rs
```

At the moment, the crate is intentionally small.

The current program performs roughly this sequence:

```text
create InMemoryDB
        ↓
insert funded Alice account
        ↓
build TxEnv
        ↓
build REVM mainnet context
        ↓
transact()
        ↓
inspect ExecutionResult
        ↓
inspect changed account state
```

---

## Test

Run:

```bash
cargo test
```

At the current stage there may be little or no dedicated test coverage yet.

The first automated test should reproduce the working value-transfer invariant:

```text
Given:
Alice = funded
Bob   = zero

When:
Alice sends value to Bob

Then:
transaction succeeds
Alice nonce changes correctly
Alice balance changes correctly
Bob receives the transferred value
```

Future tests should cover:

```text
EOA transfer
contract deployment
contract call
contract storage
revert
out-of-gas behavior
nonce validation
state persistence
block commitments
deterministic replay
```

---

## Format

```bash
cargo fmt
```

Check formatting without modifying files:

```bash
cargo fmt -- --check
```

---

## Lint

```bash
cargo clippy --all-targets
```

---

## Current Limitations

The current program is an execution proof, not yet a blockchain node.

### State is temporary

The database is:

```rust
InMemoryDB
```

State disappears when the program exits.

### Accounts are test fixtures

The current Alice and Bob addresses use REVM benchmark constants.

### Transactions are constructed internally

The program creates `TxEnv` directly.

It does not yet accept a real signed Ethereum transaction from a wallet.

### No local chain exists yet

The current EVM transition is not yet packaged into a real Solizone block.

### No Logos publication happens here yet

The working Logos publication/finality path exists in the completed experiments, especially Experiment 003.

That integration will be brought into `solizone-evm` after execution, state, and block boundaries are clean.

### No RPC server exists yet

There is currently no Ethereum JSON-RPC interface such as:

```text
eth_sendRawTransaction
eth_call
eth_getBalance
eth_getTransactionReceipt
```

---

## Development Principle

Keep execution and publication separated.

Do not make REVM responsible for Logos behavior.

The intended boundary is:

```text
Ethereum transaction
      ↓
Solizone execution
      ↓
Solizone state
      ↓
canonical Solizone block
      ↓
publisher interface
      ↓
Logos Zone SDK
```

This lets us test EVM execution without a Logos node and test publication without embedding Logos assumptions into the EVM engine.

---

## Planned Module Structure

A likely future structure is:

```text
src/
├── main.rs
├── config.rs
│
├── execution/
│   ├── mod.rs
│   └── revm_engine.rs
│
├── state/
│   ├── mod.rs
│   ├── memory.rs
│   ├── persistent.rs
│   └── commitment.rs
│
├── transaction/
│   ├── mod.rs
│   ├── decode.rs
│   └── validate.rs
│
├── mempool/
│   └── mod.rs
│
├── block/
│   ├── mod.rs
│   ├── header.rs
│   ├── codec.rs
│   ├── commitment.rs
│   └── producer.rs
│
├── chain/
│   ├── mod.rs
│   └── storage.rs
│
├── publisher/
│   ├── mod.rs
│   ├── logos.rs
│   └── mock.rs
│
├── rpc/
│   ├── mod.rs
│   └── eth.rs
│
└── indexer/
    └── mod.rs
```

This is a target structure, not a request to create every directory immediately.

Modules should appear when the implementation needs them.

---

## Immediate Next Steps

From the working value transfer:

```text
1. Extract REVM execution from main.rs.

2. Create an ExecutionEngine boundary.

3. Replace REVM benchmark addresses with explicit test accounts.

4. Convert the working transfer into an automated test.

5. Deploy a minimal Solidity contract.

6. Execute a contract call.

7. Verify contract storage mutation.

8. Capture real receipt data.

9. Add a StateBackend abstraction.

10. Introduce persistent state.

11. Feed real EVM execution results into the canonical
    Solizone block format proven in Experiment 003.
```

The next major execution target is a minimal Solidity contract:

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
compile contract
      ↓
deployment bytecode
      ↓
REVM deployment
      ↓
contract created
      ↓
call increment()
      ↓
storage changes
      ↓
receipt produced
```

---

## Relationship to Experiment 003

Experiment 003 created a canonical Solizone block with synthetic transactions and placeholder execution commitments.

Its block model included:

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

`solizone-evm` will progressively replace the placeholders:

```text
synthetic transactions
        ↓
real signed Ethereum transactions

placeholder state_root
        ↓
execution-derived state commitment

placeholder receipts_root
        ↓
real receipt commitment

fixed gas_used
        ↓
actual EVM gas usage
```

That is the bridge between the completed research phase and the active implementation.

---

## Relationship to Logos

The target full pipeline is:

```text
Ethereum tooling
      ↓
Solizone JSON-RPC
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
Logos publisher
      ↓
Logos Zone SDK
      ↓
Mantle channel
      ↓
Logos ordering / finality
```

Solizone owns what the block means.

Logos does not need to execute the EVM transaction itself.

---

## Useful Commands

```bash
# Enter the crate
cd solizone-evm

# Verify compilation
cargo check

# Build
cargo build

# Run current EVM prototype
cargo run

# Run tests
cargo test

# Format
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint
cargo clippy --all-targets
```

---

## Related Documentation

```text
../README.md
```

Repository overview.

```text
../SOLIZONE_EVM_README.md
```

Solizone EVM concept, architecture, and project context.

```text
../DEVELOPMENT_ROADMAP.md
```

Detailed implementation roadmap.

```text
../experiments/minimal-zone/
```

Experiment 001 - minimal Logos Zone publication.

```text
../experiments/experiment-002-publication-limits/
```

Experiment 002 - publication and payload constraints.

```text
../experiments/experiment-003-canonical-block/
```

Experiment 003 - canonical Solizone block publication and finalization.

---

## Current Goal

The short-term goal is:

```text
real Ethereum transaction
      ↓
REVM execution
      ↓
persistent Solizone state
      ↓
real receipt
      ↓
canonical Solizone block
```

Then reconnect that block to the Logos publication path already proven by the experiments.

---

## Disclaimer

Solizone is an independent research prototype and is **not an official Logos project**.
