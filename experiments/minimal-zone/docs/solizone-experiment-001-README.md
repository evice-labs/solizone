# Solizone R&D - Experiment 001: Minimal Logos Zone Publication

> **Purpose:** Prove that Solizone can create and recover a persistent Logos Zone channel, publish opaque data through the Logos Zone SDK, observe it on-chain, and track it toward finalization - **before introducing any EVM execution logic**.

---

## 60-second recall

If you come back to this experiment after a few days, remember this:

- We intentionally **did not use EVM, Solidity, JSON-RPC, REVM, Foundry, or MetaMask**.
- The goal was only to prove the **Bedrock/Logos-facing publication path** first.
- We built a tiny Rust app around the **Logos Zone SDK**.
- We created a persistent:
  - `ChannelId`
  - sequencer Ed25519 key
  - checkpoint file
- The sequencer:
  1. connected to the local Logos node,
  2. backfilled chain history,
  3. emitted `Ready`,
  4. published the bytes for `hello-solizone`,
  5. funded the Mantle transaction using the node wallet,
  6. submitted it,
  7. observed it in the node mempool,
  8. observed the channel tip advance to our message.
- The transaction is **confirmed on-chain and is the canonical channel tip**.
- Finalization is confirmed after the Logos LIB advances past the transaction’s inclusion slot.

This experiment validates the most important early assumption for Solizone:

> **Solizone can treat the Logos Zone SDK as its Bedrock publication boundary while keeping its own execution model separate.**

---

# 1. Why this experiment exists

Solizone is intended to become an **EVM-compatible Sovereign Zone on Logos Blockchain**.

The eventual architecture is roughly:

```text
Ethereum tooling
Foundry / wallets / ethers / viem
            │
            ▼
     Ethereum JSON-RPC
            │
            ▼
   Solizone Transaction Pool
            │
            ▼
   Solizone Block Producer
            │
            ▼
        EVM Runtime
            │
            ▼
      Solizone State
            │
            ▼
      Solizone Block
            │
            ▼
     Bedrock Publisher
            │
            ▼
      Logos Zone SDK
            │
            ▼
      Mantle Channel
            │
            ▼
   Logos Blockchain / Bedrock
```

Before implementing the EVM side, we needed to prove the lowest-level assumption:

> Can a standalone Solizone process publish arbitrary ordered bytes into a Logos Zone channel, recover state after restart, and observe that data progressing through Logos consensus/finality?

That is what Experiment 001 tests.

---

# 2. What we intentionally did NOT test

This experiment is deliberately small.

We did **not** test:

- Solidity
- EVM bytecode
- REVM
- smart contracts
- Ethereum JSON-RPC
- MetaMask
- Foundry
- EVM account state
- EVM gas semantics
- Solizone bridge design
- cross-zone communication
- fraud proofs
- ZK proofs
- decentralized sequencing
- multi-sequencer committees

Those belong to later experiments.

---

# 3. Experiment goal

Publish one minimal Solizone message:

```text
hello-solizone
```

Conceptually, this stands in for a future canonical Solizone block:

```text
SolizoneBlock {
    height: 0,
    parent: 0x00,
    payload: "hello-solizone"
}
```

For Experiment 001 we only published the raw payload bytes.

---

# 4. Success criteria

The experiment is considered successful when we can prove:

- [x] Solizone connects to a Logos node
- [x] Solizone has a stable `ChannelId`
- [x] Solizone has a stable sequencer identity
- [x] ZoneSequencer performs backfill
- [x] ZoneSequencer reaches `Ready`
- [x] checkpoint restore works
- [x] opaque bytes can be published
- [x] a Mantle transaction is created
- [x] the node accepts the transaction
- [x] the transaction is visible via Logos RPC
- [x] the message becomes the canonical channel tip
- [x] the transaction becomes finalized
- [x] restart is tested after finalization

At the time of writing, **everything mentioned above has been demonstrated**.

---

# 5. Environment used

## Logos node

Stable installed module:

```text
blockchain_module v0.2.2
```

The node was running locally and exposed HTTP on:

```text
http://localhost:8080
```

Typical status check:

```bash
/Users/bristinborah/bin/logoscore status
```

Chain status:

```bash
/Users/bristinborah/bin/logoscore call blockchain_module get_cryptarchia_info \
  | jq -r .result.value \
  | jq .
```

---

# 6. Project location

Local experiment path:

```text
~/logos/solizone/experiments/minimal-zone
```

Rust package:

```text
minimal-zone
```

---

# 7. Persistent experiment identities

We intentionally stored the channel identity and sequencer key outside source control.

Created with:

```bash
mkdir -p .secrets
openssl rand -hex 32 > .secrets/channel_id
openssl rand -hex 32 > .secrets/sequencer_key
printf "\n.secrets/\n" >> .gitignore
```

## Channel ID

```text
4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e
```

## Sequencer public key

```text
19c7730290dfa62a4f2eaeebc764b52dd81982b20f26fbcc056a6115f908d57a
```

> Do not commit or share `.secrets/sequencer_key`.

---

# 8. Funding wallet

We used this Logos wallet address as the transaction funding source:

```text
08a9dee9b06a2ec6fae91af10ef96b965ed38714fbdd2c53667d2e0555a36b14
```

Known wallet addresses can be inspected using:

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module wallet_get_known_addresses
```

The funding note was:

```text
Note ID:
b7fbc96c10a980ee6be1b821906859cfc50157e1ea0d66c092554a9500e26015

Value:
1000000000000
```

Wallet note query:

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module wallet_get_notes \
  08a9dee9b06a2ec6fae91af10ef96b965ed38714fbdd2c53667d2e0555a36b14 \
  ""
```

---

# 9. Official references used

We treated the official Logos repositories as the source of truth.

## Logos Blockchain

```text
https://github.com/logos-blockchain/logos-blockchain
```

This contains:

- Zone SDK
- Mantle types
- channel logic
- node HTTP client
- wallet logic
- sequencer implementation

## Logos Docs

```text
https://github.com/logos-co/logos-docs
```

Used for broader Logos architecture/context.

## Logos Execution Zone

```text
https://github.com/logos-blockchain/logos-execution-zone
```

Used as an **architecture reference**, especially for the pattern of separating execution-side logic from Bedrock/Zone publication logic.

Important:

> LEZ is not being copied as Solizone's execution engine. It is only a useful reference for how a Zone-facing publisher/indexer/sequencer may be structured.

## Logoscore CLI

```text
https://github.com/logos-co/logos-logoscore-cli
```

Used to inspect and control the installed Logos module.

## Logos Modules Release

```text
https://github.com/logos-co/logos-modules-release
```

Used to investigate the installed stable `blockchain_module v0.2.2` package.

---

# 10. Critical SDK compatibility discovery

This was the biggest blocker in Experiment 001.

Initially, the Rust project used the newer Logos source commit:

```text
6922565198d155f26617beb96100c42070feea4f
```

But our installed node was:

```text
blockchain_module v0.2.2
```

The newer Zone SDK expected a newer block header schema containing:

```rust
body_root
```

while the stable node returned the older schema.

This caused backfill to repeatedly fail with:

```text
Failed to parse response: missing field `body_root`
```

The sequencer was not hung.

It was:

1. connected,
2. restoring its checkpoint,
3. attempting backfill,
4. failing during JSON deserialization,
5. retrying the same batch.

---

# 11. How we diagnosed the version mismatch

We inspected the stable package:

```bash
mkdir -p ~/logos/pkg-inspect
lgpd --version 0.2.2 -o ~/logos/pkg-inspect download blockchain_module
```

The package was:

```text
blockchain_module-0.2.2.lgx
```

Its manifest confirmed:

```text
version: 0.2.2
```

but did not include a source Git commit.

We then inspected Logos repository history.

A major clue:

```text
commit 29b03994b9a2e5a1e575f9e9a50547752c98e177
Aug 15, 2026
feat(core): uncle-ref: carry uncle headers, committed by body_root (#3281)
```

The stable `0.2.2` package had been published before that change.

Therefore:

> `blockchain_module v0.2.2` uses the pre-`body_root` block header format.

---

# 12. Compatible Zone SDK revision

We selected the pre-release/pre-`body_root` revision:

```text
7e9000318f54dfb12944b397cea713a639d6414d
```

At that revision the HTTP client expected:

```rust
pub struct ApiHeader {
    pub id: HeaderId,
    pub parent_block: HeaderId,
    pub slot: Slot,
    pub block_root: ContentId,
    pub proof_of_leadership: Groth16LeaderProof,
}
```

Notice:

```text
block_root
```

instead of:

```text
body_root
```

This immediately fixed the historical backfill serialization failure.

### Important accuracy note

We did **not** prove that `7e900031...` is the exact source commit used to build the released `blockchain_module v0.2.2`.

What we proved is:

- it predates the incompatible `body_root` schema change,
- it contains the Zone SDK APIs we need,
- it successfully interoperates with our running `v0.2.2` node.

So this is currently our **working compatibility pin**.

---

# 13. Cargo dependency pin

The Logos dependencies were pinned to:

```toml
lb-zone-sdk = { package = "logos-blockchain-zone-sdk", git = "https://github.com/logos-blockchain/logos-blockchain.git", rev = "7e9000318f54dfb12944b397cea713a639d6414d" }
lb-core = { package = "logos-blockchain-core", git = "https://github.com/logos-blockchain/logos-blockchain.git", rev = "7e9000318f54dfb12944b397cea713a639d6414d" }
lb-key-management-system-service = { package = "logos-blockchain-key-management-system-service", git = "https://github.com/logos-blockchain/logos-blockchain.git", rev = "7e9000318f54dfb12944b397cea713a639d6414d" }
lb-groth16 = { package = "logos-blockchain-groth16", git = "https://github.com/logos-blockchain/logos-blockchain.git", rev = "7e9000318f54dfb12944b397cea713a639d6414d" }
```

After changing revisions:

```bash
rm -f Cargo.lock
cargo check
```

---

# 14. Older SDK API difference we hit

After pinning to the older compatible revision, compilation failed because `FundingConfig` had changed.

The newer code used:

```rust
priority_fee_percent
```

but the compatible revision uses:

```rust
priority_fee
```

and:

```rust
FundingConfig::DEFAULT_PRIORITY_FEE
```

Correct configuration:

```rust
let funding = FundingConfig {
    funding_pk,
    max_tx_fee: 1_000_000.into(),
    priority_fee: FundingConfig::DEFAULT_PRIORITY_FEE,
};
```

This was only an SDK API version difference.

---

# 15. Checkpointing

Checkpoint path:

```text
.state/sequencer-checkpoint.json
```

The checkpoint allows the sequencer to resume from the last known finalized position rather than replaying the entire chain every time.

Conceptually it stores information like:

```text
last_msg_id
pending_txs
lib
lib_slot
```

We save checkpoints atomically using a temporary file and rename.

---

# 16. Cold start behavior

On the first compatible run, the ZoneSequencer backfilled a large historical range.

Output looked like:

```text
Backfill progress: Slot(...)
Backfill progress: Slot(...)
...
Canonical backfill complete
Sequencer ready (backfill complete, first block processed)
```

Then:

```text
✅ ZoneSequencer emitted Ready
✅ ZoneSequencer is READY
```

This proved:

- the old serialization mismatch was fixed,
- historical blocks were readable,
- the sequencer could reconstruct the channel state.

---

# 17. Warm restart behavior

On later runs, the checkpoint was restored:

```text
✅ Loaded checkpoint from .state/sequencer-checkpoint.json
Restoring from checkpoint...
```

Instead of replaying the entire chain, it performed only an incremental catch-up:

```text
Starting incremental backfill from slot X to Y
```

This proved our recovery path works.

---

# 18. Publishing `hello-solizone`

We created an opaque inscription:

```rust
let inscription = Inscription::new_unchecked(b"hello-solizone".to_vec());
```

Then:

```rust
let (publish_result, publish_checkpoint) = sequencer
    .handle()
    .publish(inscription)
    .await?;
```

Immediately after publishing, we persisted the returned checkpoint.

This is important because `publish()` mutates the sequencer's local pending state before the transaction is necessarily accepted by the node.

---

# 19. First publish result

The SDK produced:

```text
payload="hello-solizone"

parent=
0000000000000000000000000000000000000000000000000000000000000000
```

The all-zero parent is significant.

It means this is the first message in this channel's message chain.

## Message ID

```text
d411af929c36b89041b7ce89ec0f7ed57a0888ff2742f5ec74b22b60a88717ff
```

## Transaction hash

```text
a2e2f1c205bfc2a320d15d59a7ad180cb5faf788a90287c848a0239f17814ce7
```

---

# 20. Why the first publish attempt caused a wallet issue

Our first publishing test intentionally stopped immediately after printing the `PublishReceipt`.

That exposed an important SDK behavior.

The Zone SDK does roughly:

```text
publish()
   │
   ├─ wallet_fund_tx
   ├─ build signed Mantle tx
   ├─ put tx in sequencer pending state
   └─ return PublishReceipt

next_event()
   │
   └─ actually drives the queued post / lifecycle
```

Because we exited immediately after `publish()`, the node wallet had already funded the transaction, but the sequencer had not continued driving the submission lifecycle.

On the next run we got:

```text
Wallet does not have enough funds, available=0
```

even though:

```text
wallet_get_balance = 1000000000000
```

and `wallet_get_notes` still showed the original note.

---

# 21. Why balance existed but funding said `available=0`

Inspection of Logos wallet source explained it.

Wallet funding intentionally excludes:

- consumed notes
- locked notes
- channel-owned notes
- **notes reserved/excluded for in-flight transactions**

So a note can still appear in the wallet state while not being available to fund another transaction.

This was not a missing-funds problem.

It was a stale/in-flight reservation caused by our early process exit.

---

# 22. How we cleared the stale funding state

We reloaded the blockchain module:

```bash
/Users/bristinborah/bin/logoscore reload-module blockchain_module
```

Important lesson:

> Reloading `blockchain_module` reloads the module host, but the node inside the module is not automatically started again.

After reload:

```text
The node is not running.
```

So the node had to be started again:

```bash
/Users/bristinborah/bin/logoscore call \
  blockchain_module start user_config.yaml ""
```

Then status returned:

```text
mode: Online
```

After this restart, the stale in-flight wallet reservation was cleared.

---

# 23. Important operational lesson about module reload

Do not assume:

```text
module loaded == blockchain node running
```

These are different states.

After:

```bash
logoscore reload-module blockchain_module
```

always verify:

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module get_cryptarchia_info
```

Healthy output should show:

```text
mode: Online
```

If it says:

```text
The node is not running.
```

run the node start command again.

---

# 24. Correct post-publish event loop

After fixing the funding state, we changed the program so it does **not exit after `publish()`**.

Instead it continuously calls:

```rust
sequencer.next_event().await
```

and handles:

```text
Event::MempoolPending
Event::BlocksProcessed
Event::Ready
Event::TurnNotification
```

Every `BlocksProcessed` checkpoint is persisted.

---

# 25. Mempool confirmation

The SDK emitted:

```text
📨 Mempool pending
✅ hello-solizone accepted by node / mempool
```

This proves the Logos node accepted the transaction through its post API.

---

# 26. Canonical channel inclusion

Later we saw:

```text
ChannelUpdate:
orphaned=0
adopted=0
new_tip=d411af929c36b89041b7ce89ec0f7ed57a0888ff2742f5ec74b22b60a88717ff
```

The `new_tip` is exactly our `hello-solizone` message ID.

Why was `adopted=0`?

Because the Zone SDK already knows about our own locally-published transaction.

On a normal chain extension it does not necessarily echo our own publish back through `channel_update.adopted`.

The important signal is:

```text
new_tip == our MsgId
```

This proved our message advanced the canonical channel history.

---

# 27. Verifying the transaction directly

We queried the transaction:

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module get_transaction \
  a2e2f1c205bfc2a320d15d59a7ad180cb5faf788a90287c848a0239f17814ce7
```

The node returned the Mantle transaction.

It contained:

```text
channel_id:
4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e
```

and:

```text
inscription:
68656c6c6f2d736f6c697a6f6e65
```

Hex decoding:

```text
68656c6c6f2d736f6c697a6f6e65
         ↓
hello-solizone
```

It also contained the sequencer signer:

```text
19c7730290dfa62a4f2eaeebc764b52dd81982b20f26fbcc056a6115f908d57a
```

and the wallet-funded transfer.

---

# 28. Verifying channel state directly

We queried:

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module get_channel_state \
  4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e
```

The node returned:

```text
accredited_keys:
[
  19c7730290dfa62a4f2eaeebc764b52dd81982b20f26fbcc056a6115f908d57a
]

configuration_threshold: 1
transfer_threshold: 1

tip_message:
d411af929c36b89041b7ce89ec0f7ed57a0888ff2742f5ec74b22b60a88717ff

tip_slot:
1597474

posting_timeframe: 0
posting_timeout: 0
```

This is one of the strongest results of Experiment 001.

It proves:

- the channel exists,
- our sequencer key controls it,
- our message is the canonical channel tip,
- our message is included on Logos,
- the channel is currently configured as a single-sequencer channel.

---

# 29. Why finalization has not happened yet

The message was included at:

```text
tip_slot = 1597474
```

At the latest check during this experiment:

```text
lib_slot = 1591245
current slot = 1598228
```

So:

```text
current chain > tx slot
```

which proves the transaction is already in chain history.

But:

```text
LIB < tx slot
```

which means the block containing the transaction is not finalized yet.

Difference at that point:

```text
1597474 - 1591245 = 6229 slots
```

Therefore the Rust process correctly continues to show the tx as pending from the finality-tracking perspective.

We are waiting for:

```text
lib_slot >= 1597474
```

At that point the SDK should emit the finalized transaction.

Expected output:

```text
🎉 hello-solizone FINALIZED
```

---

# 30. Current Experiment 001 status

## Proven

```text
Solizone process
      │
      ▼
persistent ChannelId
      │
      ▼
persistent sequencer key
      │
      ▼
ZoneSequencer
      │
      ▼
checkpoint restore
      │
      ▼
historical / incremental backfill
      │
      ▼
Ready
      │
      ▼
hello-solizone
      │
      ▼
wallet funding
      │
      ▼
signed Mantle tx
      │
      ▼
node post API
      │
      ▼
mempool
      │
      ▼
canonical Logos chain
      │
      ▼
channel tip = hello-solizone ✅
      │
      ▼
LIB finality ⏳
```

---

# 31. How Experiment 001 helps Solizone

This experiment validates a foundational architectural decision.

Solizone does **not** need Bedrock to understand:

- EVM transactions
- EVM state
- Solidity
- accounts
- bytecode
- storage
- receipts

Instead:

```text
Solizone owns execution.
Logos/Bedrock owns shared consensus/publication/order/interoperability.
```

Future Solizone blocks can be encoded as opaque inscriptions and published through this same channel.

Conceptually:

```text
future Solizone block
        │
        ▼
serialize block bytes
        │
        ▼
Inscription
        │
        ▼
ZoneSequencer.publish(...)
        │
        ▼
Mantle channel
        │
        ▼
Logos / Bedrock
```

That means we can now build the EVM side independently behind a clean interface.

---

# 32. Architectural interface validated by this experiment

A future internal interface can look roughly like:

```rust
trait BedrockPublisher {
    async fn publish_block(
        &mut self,
        block: &[u8],
    ) -> Result<PublicationReceipt>;
}
```

The implementation can wrap:

```rust
ZoneSequencer
```

This keeps the EVM runtime isolated from Logos internals.

---

# 33. What we learned about Zone SDK lifecycle

The SDK is stateful.

Correct lifecycle:

```text
init
 ↓
restore checkpoint
 ↓
backfill
 ↓
Ready
 ↓
publish
 ↓
SAVE publish checkpoint
 ↓
continue next_event()
 ↓
mempool
 ↓
on-chain
 ↓
SAVE block checkpoints
 ↓
finalized
```

Incorrect lifecycle:

```text
publish
 ↓
exit immediately ❌
```

because funding/pending state may already have changed.

---

# 34. Debugging lessons

## Lesson 1 - A retry loop is not necessarily a hang

The original SDK appeared to be stuck, but debug logs showed it was repeatedly retrying a failed backfill batch.

Always run with:

```bash
RUST_LOG=debug cargo run
```

during R&D.

## Lesson 2 - Match SDK source to installed node version

A Git dependency pinned to latest source is dangerous when the local module comes from a stable binary release.

For Logos experiments always record both:

```text
blockchain_module version
Zone SDK Git commit
```

## Lesson 3 - Persist publish checkpoints immediately

`publish()` changes local sequencer state before finalization.

The checkpoint returned by publish must be treated as durable state.

## Lesson 4 - Keep driving the event loop

A successful `publish()` does not mean the entire network lifecycle has completed.

Continue polling:

```rust
next_event()
```

## Lesson 5 - Wallet balance != spendable funding balance

A note may appear in:

```text
wallet_get_notes
```

while being temporarily unavailable to:

```text
wallet_fund_tx
```

because in-flight reservations are excluded.

## Lesson 6 - Module reload != node restart

After:

```bash
logoscore reload-module blockchain_module
```

verify the actual node state.

---

# 35. Useful commands cheat sheet

## Check Logoscore

```bash
/Users/bristinborah/bin/logoscore status
```

## Check chain

```bash
/Users/bristinborah/bin/logoscore call \
  blockchain_module get_cryptarchia_info \
  | jq -r .result.value \
  | jq .
```

## Known wallet addresses

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module wallet_get_known_addresses
```

## Wallet notes

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module wallet_get_notes \
  08a9dee9b06a2ec6fae91af10ef96b965ed38714fbdd2c53667d2e0555a36b14 \
  ""
```

## Channel state

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module get_channel_state \
  4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e
```

## Transaction

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module get_transaction \
  a2e2f1c205bfc2a320d15d59a7ad180cb5faf788a90287c848a0239f17814ce7
```

## Compile experiment

```bash
cd ~/logos/solizone/experiments/minimal-zone
cargo check
```

## Run with debug logs

```bash
RUST_LOG=debug cargo run
```

## Reload module

```bash
/Users/bristinborah/bin/logoscore reload-module blockchain_module
```

## Start node again after reload

```bash
/Users/bristinborah/bin/logoscore call \
  blockchain_module start user_config.yaml ""
```

---

# 36. When returning to this experiment later

Use this sequence.

### 1. Check node

```bash
/Users/bristinborah/bin/logoscore call \
  blockchain_module get_cryptarchia_info \
  | jq -r .result.value \
  | jq .
```

You want:

```text
mode: Online
```

### 2. Check channel

```bash
/Users/bristinborah/bin/logoscore -j call \
  blockchain_module get_channel_state \
  4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e
```

Remember:

```text
tip_message =
d411af929c36b89041b7ce89ec0f7ed57a0888ff2742f5ec74b22b60a88717ff
```

### 3. Check finality

Compare:

```text
lib_slot
```

against:

```text
1597474
```

If:

```text
lib_slot >= 1597474
```

the transaction should be finalizable/finalized.

### 4. Run the experiment

```bash
cd ~/logos/solizone/experiments/minimal-zone
RUST_LOG=debug cargo run
```

---

# 37. Remaining work for Experiment 001

Before declaring Experiment 001 fully complete:

- [ ] wait until `lib_slot >= 1597474`
- [ ] observe SDK `FinalizedTx`
- [ ] confirm Rust prints `hello-solizone FINALIZED`
- [ ] inspect checkpoint after finalization
- [ ] restart the Rust app
- [ ] verify checkpoint restores with zero pending txs
- [ ] verify channel tip remains `d411af...`
- [ ] optionally publish a second message whose parent is the first MsgId

The most valuable final recovery test will be:

```text
message #0 finalized
        ↓
stop Solizone
        ↓
restart
        ↓
restore checkpoint
        ↓
publish message #1
        ↓
parent(message #1) = message #0
```

That will prove ordered continuity across process restart.

---

# 38. What should come after Experiment 001

Do **not** jump directly to a full EVM node.

Recommended next research sequence:

## Experiment 002 - Inscription constraints

Determine:

- maximum payload size
- serialization overhead
- practical block payload format
- batching considerations
- publication fee behavior

## Experiment 003 - Canonical Solizone block format

Replace:

```text
hello-solizone
```

with a real serialized structure such as:

```text
SolizoneBlock {
    version
    height
    parent_hash
    timestamp
    tx_root
    state_root
    receipts_root
    payload
}
```

## Experiment 004 - EVM runtime

Only after publication mechanics are stable:

```text
Ethereum tx
   ↓
REVM
   ↓
Solizone state transition
   ↓
Solizone block
   ↓
Bedrock Publisher
   ↓
Zone SDK
```

---

# 39. Final conclusion

Experiment 001 has already validated the core Solizone/Logos integration hypothesis.

We proved that an independent Rust process can:

1. create a stable Zone identity,
2. recover from a persisted checkpoint,
3. synchronize against Logos,
4. publish arbitrary opaque bytes,
5. use the node wallet to fund the publication,
6. submit a valid Mantle transaction,
7. observe mempool acceptance,
8. create/advance a Logos channel,
9. make the Solizone message the canonical channel tip.

The only remaining observation is Logos finality.

This is enough to move forward with confidence that:

> **Solizone can own EVM execution locally while using the Logos Zone SDK and Mantle channel as its publication/ordering interface to Bedrock.**

---

# 40. Current immutable identifiers

Keep these together for future debugging.

```text
Channel ID
4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e

Sequencer public key
19c7730290dfa62a4f2eaeebc764b52dd81982b20f26fbcc056a6115f908d57a

Message ID
d411af929c36b89041b7ce89ec0f7ed57a0888ff2742f5ec74b22b60a88717ff

Transaction hash
a2e2f1c205bfc2a320d15d59a7ad180cb5faf788a90287c848a0239f17814ce7

Transaction inclusion / channel tip slot
1597474

Funding wallet
08a9dee9b06a2ec6fae91af10ef96b965ed38714fbdd2c53667d2e0555a36b14

Funding note
b7fbc96c10a980ee6be1b821906859cfc50157e1ea0d66c092554a9500e26015

Compatible Logos source pin
7e9000318f54dfb12944b397cea713a639d6414d

Installed Logos blockchain module
v0.2.2
```

---

## Document status

```text
Experiment: 001 - Minimal Logos Zone Publication
Status: ON-CHAIN / WAITING FOR FINALIZATION
Solizone EVM work: NOT STARTED BY DESIGN
```

### Voila , It worked now 

```text
After finalizing the first hello-solizone message, the process was restarted. The sequencer restored from checkpoint with zero pending transactions, performed incremental backfill, reached Ready, and published a second message whose parent was the first finalized MsgId. This confirms ordered channel continuity across process restart.
```
