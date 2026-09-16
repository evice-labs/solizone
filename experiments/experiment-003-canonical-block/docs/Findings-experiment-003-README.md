# Experiment 003 - Canonical Solizone Block Publication

## Status

**PASSED / FINALIZED**

Experiment 003 proves that Solizone can define its own deterministic block format, serialize a complete block into canonical bytes, publish those bytes through the Logos Zone stack, and observe the resulting publication become part of finalized Logos chain history.

This experiment is the final foundational publication experiment before moving into the actual `solizone-evm` implementation.

---

## 1. Objective

The goal of Experiment 003 was to answer one core question:

> Can Solizone create its own canonical blockchain block, treat that block as an opaque payload from Logos' point of view, publish it through a Logos Zone channel, and verify that the publication becomes finalized chain history?

Earlier experiments had already established two important facts:

- **Experiment 001** proved that a Zone channel could be used successfully and that ordered messages could be published through the Logos stack.
- **Experiment 002** explored publication behavior across payload sizes and showed that payloads from small messages up to 1 MiB could be accepted in the tested environment.

Experiment 003 moved beyond arbitrary payloads.

Instead of publishing test strings or synthetic benchmark bytes, this experiment created a real Solizone-specific block format containing:

- block metadata,
- parent linkage,
- state commitment,
- transaction commitment,
- receipt commitment,
- gas accounting,
- and an ordered transaction body.

The key architectural idea is that **Solizone owns the meaning of the block**, while **Logos only needs to transport, order, and finalize the bytes**.

---

## 2. Why This Experiment Matters

Before Experiment 003, we had proven:

```text
Solizone can publish bytes to Logos.
```

After Experiment 003, we can say:

```text
Solizone can define a deterministic block,
serialize it into canonical bytes,
hash it,
publish that exact block through Logos,
and have the publication finalized.
```

That is a materially different milestone.

It establishes the boundary we want between Solizone and Logos:

```text
Solizone
  owns execution
  owns state
  owns block format
  owns block hashes
  owns transaction semantics

        ↓ canonical block bytes

Logos / Bedrock
  transports Zone data
  orders Zone publications
  anchors Zone history
  provides shared consensus/finality
```

Logos does not need to understand EVM semantics, Solizone accounts, state roots, receipts, or transaction contents.

It only needs to reliably carry the canonical Solizone block bytes.

---

## 3. Experiment Directory

The experiment lives under:

```text
experiments/experiment-003-canonical-block/
```

Relevant files:

```text
experiment-003-canonical-block/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs
│   └── bin/
│       └── publish.rs
├── .output/
│   └── block-0.szb
├── .state/
│   └── sequencer-checkpoint.json
└── .secrets/
    ├── channel_id
    └── sequencer_key
```

> `.secrets/` must never be committed.

---

## 4. Experiment Design

The experiment was split into two logical stages.

### Stage A - Build and verify a canonical Solizone block

`src/main.rs`:

1. constructs a deterministic Solizone block,
2. serializes it into a canonical byte representation,
3. computes the Solizone block hash,
4. decodes the bytes,
5. verifies the transaction commitment,
6. verifies that decoding preserves the original block,
7. performs a tamper test.

### Stage B - Publish the exact canonical block bytes through Logos

`src/bin/publish.rs`:

1. reads the exact block artifact from `.output/block-0.szb`,
2. verifies the `SZB1` magic,
3. recomputes the block hash from the canonical header,
4. restores the Zone sequencer checkpoint,
5. waits for the Zone sequencer to reach `Ready`,
6. publishes the exact 245-byte block payload,
7. records the Logos transaction hash,
8. observes mempool acceptance,
9. verifies subsequent channel state,
10. verifies the funding wallet state transition,
11. verifies Logos LIB passed the publication slot.

---

## 5. Canonical Solizone Block Model

The experiment defines two primary structures.

```rust
type Hash32 = [u8; 32];

struct SolizoneBlockHeader {
    version: u16,
    chain_id: u64,
    height: u64,
    parent_hash: Hash32,
    timestamp: u64,
    state_root: Hash32,
    transactions_root: Hash32,
    receipts_root: Hash32,
    gas_limit: u64,
    gas_used: u64,
}

struct SolizoneBlock {
    header: SolizoneBlockHeader,
    transactions: Vec<Vec<u8>>,
}
```

The header is intentionally fixed-width and deterministic.

---

## 6. Canonical Header Encoding

The header is exactly **170 bytes**.

All integer fields are encoded in **big-endian** order.

| Field | Size |
|---|---:|
| `version` | 2 bytes |
| `chain_id` | 8 bytes |
| `height` | 8 bytes |
| `parent_hash` | 32 bytes |
| `timestamp` | 8 bytes |
| `state_root` | 32 bytes |
| `transactions_root` | 32 bytes |
| `receipts_root` | 32 bytes |
| `gas_limit` | 8 bytes |
| `gas_used` | 8 bytes |
| **Total** | **170 bytes** |

Byte ranges:

```text
version            [0..2]
chain_id           [2..10]
height             [10..18]
parent_hash        [18..50]
timestamp          [50..58]
state_root         [58..90]
transactions_root  [90..122]
receipts_root      [122..154]
gas_limit          [154..162]
gas_used           [162..170]
```

Because the field order and integer encoding are fixed, the same logical header always produces the same byte representation.

---

## 7. Solizone Block Hash

The Solizone block hash is:

```text
Keccak256(canonical_header_bytes)
```

For Block #0:

```text
0x3d45108eb70b5ddc35d3a19ad88758347b9d9de70a19c321793e63948eac156f
```

This hash identifies the Solizone block independently of the Logos transaction that carries it.

This distinction is important:

```text
Solizone block hash
    identifies the Solizone block

Logos transaction hash
    identifies the Logos transaction used to publish it
```

They belong to two different layers.

---

## 8. Transaction Commitment

The transaction root in this experiment is not yet Ethereum's transaction trie.

Experiment 003 deliberately uses a simpler deterministic commitment:

```rust
fn compute_transactions_root(transactions: &[Vec<u8>]) -> Hash32 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"SOLIZONE_TX_ROOT_V1");
    bytes.extend_from_slice(&(transactions.len() as u32).to_be_bytes());

    for tx in transactions {
        bytes.extend_from_slice(&(tx.len() as u32).to_be_bytes());
        bytes.extend_from_slice(tx);
    }

    keccak256(&bytes)
}
```

The commitment includes:

```text
domain separator
+
transaction count
+
transaction 0 length
+
transaction 0 bytes
+
transaction 1 length
+
transaction 1 bytes
+
...
```

This gives Experiment 003 three useful properties:

- transaction order matters,
- transaction contents matter,
- transaction boundaries are unambiguous.

Changing any transaction byte changes the resulting transaction root.

---

## 9. Full Block Encoding

A complete Solizone block is encoded as:

```text
4 bytes   magic
170 bytes canonical header
4 bytes   transaction count

for each transaction:
    4 bytes transaction length
    N bytes transaction payload
```

The magic value is:

```text
SZB1
```

or in bytes:

```text
53 5a 42 31
```

This allows a decoder to immediately distinguish a Solizone canonical block from unrelated payloads.

---

## 10. Block #0 Test Data

Experiment 003 uses three synthetic transactions:

```text
alice -> bob : 10
bob -> charlie : 4
charlie -> alice : 1
```

Block #0 uses:

```text
version       = 1
chain_id      = 9001
height        = 0
parent_hash   = 0x00...00
timestamp     = 1,800,000,000
state_root    = 0x01...01
receipts_root = 0x03...03
gas_limit     = 30,000,000
gas_used      = 21,000
```

The transaction root is computed from the three ordered transaction payloads.

The state and receipt roots are placeholders in this experiment.

They are intentionally not real EVM-derived values yet.

---

## 11. Generated Artifact

Running:

```bash
cargo run --bin solizone-experiment-003
```

created:

```text
.output/block-0.szb
```

The canonical encoded block size was:

```text
245 bytes
```

The beginning of the artifact can be inspected as:

```text
535a 4231 ...
```

which corresponds to:

```text
SZB1
```

---

## 12. Local Verification Result

The canonical block generator produced:

```text
=== Solizone Experiment 003 ===
Canonical full-block encoding

Saved canonical block: .output/block-0.szb
Block height: 0
Transactions: 3
Header size: 170 bytes
Full encoded block: 245 bytes

Block hash:
0x3d45108eb70b5ddc35d3a19ad88758347b9d9de70a19c321793e63948eac156f

Decoded transactions:
  tx 0: alice -> bob : 10
  tx 1: bob -> charlie : 4
  tx 2: charlie -> alice : 1

✅ Full block encoded
✅ Full block decoded
✅ Transaction commitment verified
✅ Decoded block equals original block
✅ Block hash preserved
```

This proved that the block encoding was deterministic and round-trip safe.

---

## 13. Tamper Test

Experiment 003 also modifies a transaction byte after encoding.

The decoder then recomputes the transaction root and compares it to the root committed in the header.

The tampered block was rejected:

```text
=== Tamper test ===
✅ Tampered block rejected
Reason: transactions_root does not match block body
```

This proves that the canonical header commits to the ordered transaction body.

A modified block body cannot silently pass verification while retaining the original header commitment.

---

## 14. Logos Testnet Environment

The Logos node used in this experiment was connected to the Logos **testnet**.

The configured bootstrap peers included:

```text
/ip4/65.109.51.37/udp/3000/quic-v1/...
/ip4/65.109.51.37/udp/3001/quic-v1/...
/ip4/65.109.51.37/udp/3002/quic-v1/...
/ip4/65.109.51.37/udp/50001/quic-v1/...
```

These matched the published Logos testnet peers used during the experiment.

At the time of publication the node was online and actively following the chain.

---

## 15. Zone Channel

Experiment 003 reused the existing Zone channel:

```text
Channel ID:
4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e
```

The channel's accredited sequencer key was:

```text
19c7730290dfa62a4f2eaeebc764b52dd81982b20f26fbcc056a6115f908d57a
```

The channel state after publication confirmed this key as the accredited sequencer.

---

## 16. Sequencer Checkpoint Recovery

A major practical issue encountered before publication was an old Zone sequencer checkpoint containing pending transactions from Experiments 001 and 002.

The checkpoint contained:

```text
pending_count = 14
```

Those pending entries mapped directly to older experiment traffic, including payloads such as:

```text
14 B
32 B
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

The old checkpoint was backed up before modification.

The stale channel-specific pending state was cleared while preserving the already-known LIB position.

Conceptually:

```text
preserve:
    lib
    lib_slot

reset:
    last_msg_id -> root
    pending_txs -> []
```

This allowed the sequencer to resume from the existing chain progress instead of replaying the full chain from genesis.

After recovery:

```text
pending 0
adopted 0
orphaned 0
```

and the Zone sequencer successfully reached:

```text
READY
```

---

## 17. Funding Wallet

The original publisher funding address was no longer valid for the current node wallet.

A new wallet key controlled by the active Basecamp node was funded through the Logos testnet faucet:

```text
5d0307d3c63850c53bd13d6b2597bcc751ee927f346956d135a472ca1b7e3e2d
```

Initial balance:

```text
1,000,000,000,000
```

Initial spendable note:

```text
fa0da9b0d66ce561e4ecf0b4ad412450a684f725e875f446276249f3b75c2f0c
```

---

## 18. Publishing Block #0

The final publication command was:

```bash
cargo run --bin publish --   --send   --funding-address 5d0307d3c63850c53bd13d6b2597bcc751ee927f346956d135a472ca1b7e3e2d
```

The publisher:

1. loaded `.output/block-0.szb`,
2. verified the `SZB1` magic,
3. recomputed the Solizone block hash,
4. restored the sequencer checkpoint,
5. connected to the Logos node,
6. caught up to the current LIB,
7. reached `ZoneSequencer READY`,
8. published the exact canonical bytes.

---

## 19. Publication Result

The publish operation returned:

```text
Publishing canonical Solizone Block #0...
✅ Publication queued
```

Logos transaction hash:

```text
d197f87042b2317fc4511034d71e6ae91d392249bb4a5b099c96343755f591bd
```

Canonical payload:

```text
245 bytes
```

The Zone sequencer then reported:

```text
📨 MempoolPending
```

and the publisher confirmed:

```text
✅ Solizone Block #0 accepted by Logos mempool
```

Measured publish-to-mempool latency:

```text
0.130 seconds
```

---

## 20. Mempool Observation

Immediately after publication, the Zone sequencer observed the transaction as `MempoolPending`.

Later, querying:

```text
/mempool/view
```

showed that the transaction hash was no longer present.

That alone does not prove finalization, but it established that the transaction had moved beyond its initial mempool state.

---

## 21. Wallet State Transition

Before publication:

```text
balance:
1,000,000,000,000

note:
fa0da9b0d66ce561e4ecf0b4ad412450a684f725e875f446276249f3b75c2f0c
```

After publication:

```text
balance:
999,999,994,242

note:
f5e36a4346d76c05c5ac24d98b2ea2c71bd59a5d2b71bd9d8f7d242a96e3af1c
```

The original note had been consumed and replaced by a new change note.

Observed cost:

```text
1,000,000,000,000
- 999,999,994,242
-----------------
5,758
```

So the publication consumed:

```text
5,758
```

units from the funding wallet.

This is evidence that the publication transaction had been applied to the current chain state rather than remaining only as a local mempool reservation.

---

## 22. Channel State After Publication

The correct API route for the current node was:

```text
/channel/:id
```

Query:

```bash
curl   "http://127.0.0.1:8080/channel/4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e"
```

returned:

```json
{
  "accredited_keys": [
    "19c7730290dfa62a4f2eaeebc764b52dd81982b20f26fbcc056a6115f908d57a"
  ],
  "configuration_threshold": 1,
  "tip_message": "5e7dd93cd8ae71b2630ce41a9608edba6dda35ebeadc98f9c6b6dc66e6987447",
  "tip_slot": 1062443,
  "tip_sequencer": 0,
  "tip_sequencer_starting_slot": 1062443,
  "posting_timeframe": 0,
  "posting_timeout": 0,
  "transfer_threshold": 1
}
```

The important values were:

```text
tip_message:
5e7dd93cd8ae71b2630ce41a9608edba6dda35ebeadc98f9c6b6dc66e6987447

tip_slot:
1,062,443
```

This confirmed that the Zone channel had advanced.

---

## 23. Finalization Check

At first, the current Logos chain state was:

```text
LIB slot = 1,059,068
current slot = 1,063,239
```

while the publication was at:

```text
channel tip slot = 1,062,443
```

So the transaction was included in the current canonical chain view, but had not yet crossed the Logos last irreversible block boundary.

The experiment remained in:

```text
included / awaiting finalization
```

state.

Later, the node reported:

```json
{
  "lib_slot": 1062891,
  "slot": 1066960,
  "height": 34767,
  "state": "Online"
}
```

At that point:

```text
publication slot = 1,062,443
LIB slot         = 1,062,891
```

Therefore:

```text
LIB - publication slot = 448 slots
```

The Logos last irreversible block had moved beyond the publication.

This was the final condition required to mark the experiment as finalized.

---

## 24. Final Result

Experiment 003 passed all intended checks.

```text
✅ Canonical Solizone Block #0 created
✅ Deterministic canonical encoding
✅ Fixed 170-byte header
✅ Canonical full block = 245 bytes
✅ Deterministic Keccak block hash
✅ Decode round-trip successful
✅ Transaction commitment verified
✅ Tampered body rejected
✅ Zone sequencer restored successfully
✅ Clean sequencer checkpoint established
✅ Testnet funding wallet prepared
✅ Exact canonical block bytes published
✅ Logos mempool accepted publication
✅ Logos transaction left mempool
✅ Funding note was consumed
✅ New change note appeared
✅ Zone channel tip advanced
✅ Publication slot identified
✅ Logos LIB crossed publication slot
✅ Publication finalized
```

**Experiment 003 status: PASSED / FINALIZED**

---

## 25. What Was Proven

The strongest conclusion from Experiment 003 is:

> A Solizone-native block can be defined independently of Logos, serialized deterministically, hashed by Solizone, published as an opaque payload through a Logos Zone channel, and finalized as part of Logos chain history.

This proves an important architectural separation.

Logos does not need to execute or interpret Solizone blocks.

Solizone can own:

```text
execution
state
transactions
receipts
block format
block hash
local chain semantics
```

while Logos provides the lower-level publication and consensus infrastructure for the resulting canonical block data.

---

## 26. What Was Not Proven

Experiment 003 intentionally does **not** prove EVM execution.

The transaction body:

```text
alice -> bob : 10
bob -> charlie : 4
charlie -> alice : 1
```

is synthetic.

Likewise:

```text
state_root
receipts_root
gas_used
```

are not yet derived from actual EVM execution.

The experiment also does not yet provide:

```text
Ethereum transaction decoding
EVM bytecode execution
account state
contract storage
real transaction receipts
Ethereum-compatible state roots
Ethereum JSON-RPC
MetaMask compatibility
Foundry compatibility
Hardhat compatibility
persistent Solizone block storage
multi-block Solizone chain execution
reorg handling at the Solizone layer
```

Those belong to the next stage of the project.

---

## 27. Architectural Consequence

The experiment validates the following architecture direction:

```text
Ethereum tooling
      ↓
Ethereum JSON-RPC
      ↓
Solizone transaction pool
      ↓
Solizone block producer
      ↓
EVM runtime
      ↓
Solizone state
      ↓
Canonical Solizone block
      ↓
Bedrock / Logos publisher
      ↓
Zone SDK
      ↓
Mantle channel
      ↓
Logos Blockchain
```

Experiment 003 validates the lower section:

```text
Canonical Solizone block
      ↓
Bedrock / Logos publisher
      ↓
Zone SDK
      ↓
Mantle channel
      ↓
Logos Blockchain
```

The next development phase focuses on building the upper execution layer.

---

## 28. Next Step - `solizone-evm`

Experiment 003 is the final foundational publication experiment.

The project should now move from isolated experiments into the actual Solizone implementation.

Recommended root-level directory:

```text
solizone/
├── experiments/
│   ├── minimal-zone/
│   ├── experiment-002-publication-limits/
│   └── experiment-003-canonical-block/
│
├── solizone-evm/
├── research/
├── dev-roadmap/
└── README.md
```

`solizone-evm/` becomes the real implementation rather than another experiment.

Its first milestone should be:

```text
Ethereum-style transaction
        ↓
EVM execution
        ↓
state transition
        ↓
receipt generation
        ↓
real state/receipt commitments
        ↓
canonical Solizone block
        ↓
publish through the Experiment 003 path
```

The next important question is therefore:

> Can an Ethereum-compatible transaction execute deterministically inside Solizone, produce real state changes and receipts, be committed into a canonical Solizone block, and then be published through Logos using the path proven by Experiment 003?

---

## 29. Summary

Experiment 003 moved Solizone from:

```text
"We can publish arbitrary Zone bytes."
```

to:

```text
"We can define and publish a deterministic Solizone blockchain block."
```

The experiment established:

```text
Solizone block semantics
        +
deterministic encoding
        +
block hashing
        +
body commitment
        +
tamper detection
        +
Zone publication
        +
Logos inclusion
        +
channel progression
        +
Logos finalization
```

This is the point where the project can stop treating block publication as an open research question and begin building the actual EVM-compatible Solizone execution stack.
