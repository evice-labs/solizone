# Solizone R&D - Experiment 002: Logos Publication & Inscription Constraints

> **Purpose:** Determine the practical limits, costs, behavior, and reliability characteristics of publishing Solizone data through the Logos Zone SDK before defining the canonical Solizone block format.

> **Status:** ✅ **COMPLETE FOR THE EXPERIMENT 003 DESIGN GATE**  
> Payload publication and mempool acceptance were tested from **128 B through 1 MiB**. No payload-size rejection was observed within that range. Finalization/inclusion timing remains deferred because the local Logos node stayed in `Bootstrapping` mode with `LIB slot = 0`.

---

## 60-second recall

Experiment 001 proved that Solizone can successfully:

- connect to a Logos node,
- recover from checkpoints,
- initialize a `ZoneSequencer`,
- reach `Ready`,
- publish opaque bytes,
- fund and submit a Mantle transaction,
- observe Logos mempool acceptance,
- advance the canonical channel tip,
- track the transaction until finalization,
- restart cleanly,
- and preserve ordered message continuity.

Experiment 002 asked:

> **How much data can Solizone practically publish through a Logos channel, what does it cost, how fast is it, and what publication model should future Solizone blocks use?**

We intentionally still did **not add an EVM runtime**.

### Final answer from Experiment 002

- Successful mempool publication was observed from **128 B to 1 MiB**.
- The largest tested payload, **1 MiB**, was accepted by the Logos node/mempool.
- No protocol payload-size rejection was observed at or below 1 MiB.
- Large-payload fee estimation scaled linearly for the transaction shape used by this experiment.
- Larger payloads became materially more expensive and slower at the top end.
- Multiple Zone publications remained pending simultaneously.
- Checkpoint recovery continued to work across repeated runs.
- Finalization latency could not be measured because the local Logos node remained `Bootstrapping` with `LIB slot = 0`.
- **1 MiB is the largest tested value, not a proven Logos protocol maximum.**
- For Experiment 003, an initial **soft operating target of ≤256 KiB per publication** is recommended.

---

# 1. Why Experiment 002 was necessary

Solizone eventually needs to publish real block data.

Future Solizone blocks may contain or commit to:

```text
block height
parent block hash
timestamp
transaction root
state root
receipts root
transactions or transaction commitments
metadata
```

Before deciding the canonical block structure, we needed to know what the Logos publication layer can comfortably support.

If we designed the block format first and only later discovered that:

- payloads are too large,
- fees become impractical,
- messages need chunking,
- publishing many messages at once causes issues,
- or there are important inscription constraints,

then we would have to redesign the block format.

Experiment 002 existed to avoid that.

---

# 2. Main objective

The objective was to characterize the publication boundary between Solizone and Logos.

```text
Solizone data
     │
     ▼
serialization
     │
     ▼
Inscription
     │
     ▼
Zone SDK
     │
     ▼
Mantle transaction
     │
     ▼
Logos mempool
     │
     ▼
Logos block
     │
     ▼
Channel history
     │
     ▼
Finalization
```

The part measured reliably in this run was:

```text
serialization
     ↓
Inscription
     ↓
Zone SDK publish()
     ↓
funding / fee estimation
     ↓
Logos mempool acceptance
```

---

# 3. Main findings

## Payload limits

The structured benchmark successfully published:

- 128 B
- 1 KiB
- 4 KiB
- 16 KiB
- 32 KiB
- 64 KiB
- 128 KiB
- 256 KiB
- 512 KiB
- 1 MiB

A prior 32-byte functional warm-up also reached the mempool, but it is excluded from the clean benchmark because it ran with inherited pending state and produced an unusually high 12.321 s mempool latency.

### Maximum tested payload

> **At least 1 MiB was accepted.**

The harness intentionally rejects payloads above 1 MiB, so this experiment does **not** establish the Logos protocol maximum.

### Hard payload rejection

None was observed.

Failures at 128 KiB, 512 KiB and an early 1 MiB attempt were caused by fee caps or funding-note availability, not by the payload size itself.

---

## Fee behavior

Measured fee-estimator outputs:

| Payload | Required fee observed |
|---:|---:|
| 128 KiB | 1,841,020 |
| 512 KiB | 7,346,044 |
| 1 MiB | 14,686,076 |

For those measured points, the estimator followed exactly:

```text
estimated_tx_fee = 14 × payload_bytes + 6,012
```

This should be treated as an **observed relationship for this experiment's transaction shape**, not as a universal Logos protocol formula.

### Fee-cap failures

128 KiB initially failed with:

```text
tx_fee(1841020) exceeds max_tx_fee(1000000)
```

After increasing the fee ceiling, the exact same payload was accepted.

512 KiB initially failed with:

```text
tx_fee(7346044) exceeds max_tx_fee(5000000)
```

After increasing the fee ceiling, the exact same payload was accepted.

The first 1 MiB attempt revealed:

```text
tx_fee(14686076) exceeds max_tx_fee(10000000)
```

After increasing the fee ceiling and using a fresh spendable funding note, 1 MiB was accepted.

---

## Funding-note behavior

A practical constraint was discovered:

> A wallet may show a high total balance while the funding path reports `available=0`.

Observed example:

```text
wallet_get_balance:
999,998,280,160

publish funding:
Wallet does not have enough funds, available=0
```

This happened while previous transactions remained pending.

Operational implication:

- total wallet balance is not the same as immediately spendable publication liquidity,
- long pending chains can tie up usable notes,
- production funding logic must not assume one high balance can fund unlimited concurrent publications.

---

## Mempool acceptance

Every structured benchmark payload from 128 B through 1 MiB eventually emitted:

```text
Event::MempoolPending
```

for the current transaction.

This confirms successful mempool acceptance for every tested payload size.

---

## Inclusion / finalization

The fresh Logos node remained:

```text
mode: Bootstrapping
lib_slot: 0
```

during the benchmark.

Therefore:

- mempool acceptance is measured,
- some `channel_update.adopted` activity was observed,
- finalization timing is unavailable,
- no Experiment 002 result should claim finalization latency.

Finalization benchmarking is deferred until the node reaches a usable non-zero LIB / online state.

---

## Multiple in-flight publications

The final persisted checkpoint contained:

```text
pending_count: 14
lib_slot: 0
```

Across repeated runs the sequencer:

```text
loaded checkpoint
     ↓
recovered pending state
     ↓
reached Ready
     ↓
published another message
```

This confirms that multiple pending publications can coexist in SDK checkpoint state.

However, the funding-note failures show why Solizone should not initially design around an unbounded pending queue.

---

# 4. What this experiment did NOT test

Experiment 002 did **not** test:

- REVM
- Solidity
- EVM state
- Ethereum transactions
- smart contracts
- JSON-RPC
- MetaMask
- Foundry
- Ethereum gas calculation
- Solizone transaction pool
- bridges
- cross-zone messaging
- multi-sequencer consensus
- fraud proofs
- validity proofs

Those come later.

---

# 5. Why we delayed the EVM

```text
Experiment 001
Minimal Zone SDK lifecycle
             ✅
             │
             ▼
Experiment 002
Publication constraints
             ✅
             │
             ▼
Experiment 003
Canonical Solizone block
             ⏭ NEXT
             │
             ▼
Experiment 004
Minimal EVM execution
             ⏳
```

The EVM runtime eventually needs to produce something that can be published.

Therefore:

> We first measured the publication layer, then design Solizone blocks to fit those constraints.

---

# 6. Experiment environment

## Channel ID

```text
4cbae339257687a8a37f76d46b0f8aa80204a0951c50fbcf701e0592afcdd32e
```

## Sequencer public key

```text
19c7730290dfa62a4f2eaeebc764b52dd81982b20f26fbcc056a6115f908d57a
```

## Working Logos Zone SDK pin

```text
7e9000318f54dfb12944b397cea713a639d6414d
```

## Installed blockchain module during final benchmark

```text
blockchain_module v0.2.3
```

## Local node HTTP endpoint

```text
http://localhost:8080
```

## Experiment directory

```text
~/logos/solizone/experiments/experiment-002-publication-limits
```

## Node state during benchmark

```text
mode: Bootstrapping
lib_slot: 0
```

---

# 7. Implementation

Payload generation:

```rust
let payload = vec![0x42; payload_size];
let inscription = Inscription::new_unchecked(payload);
```

Payload size:

```bash
cargo run -- --payload-size <bytes>
```

Because finalization was unavailable, a mempool-only measurement mode was added:

```bash
cargo run -- --payload-size <bytes> --mempool-only
```

Behavior:

```text
publish()
   ↓
MempoolPending(current_tx)
   ↓
record JSONL result
   ↓
exit cleanly
```

The normal finalization-tracking path remains in the code for future use.

---

# 8. Executed payload matrix

| Test | Payload size | Result |
|---|---:|---|
| Warm-up | 32 B | ✅ Mempool accepted; excluded from clean benchmark |
| A | 128 B | ✅ |
| B | 1 KiB | ✅ |
| C | 4 KiB | ✅ |
| D | 16 KiB | ✅ |
| E | 32 KiB | ✅ |
| F | 64 KiB | ✅ |
| G | 128 KiB | ✅ after increasing fee ceiling |
| H | 256 KiB | ✅ |
| I | 512 KiB | ✅ after increasing fee ceiling |
| J | 1 MiB | ✅ after increasing fee ceiling / fresh spendable funding note |

No payload-size rejection was observed within the configured 1 MiB experiment ceiling.

---

# 9. Recorded fields

The current machine-readable result file records:

```text
payload size
transaction hash
status
time to MempoolPending
finalization time (null when unavailable)
L1 slot (null when unavailable)
fee-cap information when encountered
```

Still missing or deferred:

```text
serialized Mantle transaction byte size
exact per-test wallet before/after balances
explicit resulting MsgId in JSONL
explicit parent MsgId in JSONL
channel inclusion latency
finalization latency
```

These are not blockers for Experiment 003's first block-format design.

---

# 10. Final benchmark results

| Payload | Publish / mempool | Publish → mempool | Notes |
|---:|---|---:|---|
| 32 B | ✅ | 12.321 s | warm-up only; inherited pending state |
| 128 B | ✅ | 0.598 s | structured benchmark |
| 1 KiB | ✅ | 0.244 s | structured benchmark |
| 4 KiB | ✅ | 0.113 s | structured benchmark |
| 16 KiB | ✅ | 0.133 s | structured benchmark |
| 32 KiB | ✅ | 0.529 s | structured benchmark |
| 64 KiB | ✅ | 0.967 s | structured benchmark |
| 128 KiB | ✅ | 0.514 s | fee ceiling raised before successful run |
| 256 KiB | ✅ | 0.570 s | accepted |
| 512 KiB | ✅ | 1.040 s | fee ceiling raised before successful run |
| 1 MiB | ✅ | 4.413 s | largest configured test payload |

### Interpretation

These are single observations, not statistical averages.

Do not interpret the non-monotonic small-payload timings as a general performance law.

The strongest conclusions are:

- acceptance across the tested size range,
- increasing publication cost with payload size,
- materially higher latency at the 1 MiB end of this single-run benchmark.

---

# 11. Publication model implications

## Model A - Full block publication

```text
Solizone block
    ↓
serialize full block
    ↓
one inscription
```

**Result:** technically viable for moderately sized blocks.

Payload capacity alone does not force Solizone into a commitment-only design.

---

## Model B - Commitment-only publication

```text
Solizone block
     ↓
state root
tx root
receipts root
block hash
     ↓
small inscription
```

**Result:** still attractive for reducing publication cost, but not required because of a tiny payload limit.

---

## Model C - Chunked block publication

```text
Block
 ↓
chunk 0
chunk 1
chunk 2
 ↓
multiple inscriptions
```

**Result:** not required for the initial Solizone design at or below the tested 1 MiB range.

Chunking remains a fallback for future oversized blocks.

---

## Model D - Hybrid

Publish a canonical header / commitments through Logos while larger execution data is handled separately.

**Result:** remains a strong future option if EVM block bodies become large or publication cost dominates.

---

# 12. Parent and checkpoint behavior

Experiment 001 proved canonical parent continuity and finalization.

Experiment 002 additionally demonstrated:

- checkpoint state persisted across repeated runs,
- prior pending transactions were restored,
- new publications could be created while previous publications remained pending,
- `last_msg_id` continued to advance in checkpoint state.

Final checkpoint:

```text
last_msg_id:
3ee2b17a76a0b5c3b5a93234419e0943deffbea57b80f3bd76444d73632d21cf

pending_count:
14
```

A dedicated high-throughput parent-linkage audit can still be performed later.

---

# 13. Crash/restart behavior

Repeated Experiment 002 runs successfully performed:

```text
load checkpoint
      ↓
ZoneSequencer::init(...)
      ↓
recovery / block processing
      ↓
Ready
```

Pending publications survived process restarts/runs.

The experiment did not repeat every originally proposed crash timing scenario because the node's lack of finalization made those variants less informative.

---

# 14. Instrumentation

Experiment 002 prints:

```text
payload size
publish queued
TxHash
MempoolPending
publish → mempool latency
BlocksProcessed
LIB slot
pending transaction count
adopted count
orphaned count
finalization information when available
```

This was sufficient to characterize the payload/mempool envelope.

---

# 15. Machine-readable evidence

Raw result log:

```text
.results/experiment-002.jsonl
```

Normalized summary:

```text
.results/summary.csv
```

The raw JSONL should remain unchanged as experiment evidence.

---

# 16. Success criteria - final status

- [x] Establish a tested maximum payload for this experiment  
  **Result:** 1 MiB accepted; protocol maximum not discovered.

- [x] Identify a recommended initial operating region  
  **Result:** soft target of **≤256 KiB** recommended for Experiment 003.

- [x] Characterize fee scaling  
  **Result:** observed linear estimator relationship at measured large payloads.

- [ ] Measure exact fixed serialized transaction overhead  
  **Deferred:** full Mantle transaction byte size was not separately captured.

- [x] Measure mempool acceptance behavior  
  **Result:** all structured benchmark sizes through 1 MiB accepted.

- [ ] Measure normal channel inclusion latency  
  **Deferred:** node remained bootstrapping with `LIB = 0`.

- [ ] Measure finalization latency  
  **Deferred:** node remained bootstrapping with `LIB = 0`.

- [x] Demonstrate multiple pending transactions  
  **Result:** checkpoint reached 14 pending transactions.

- [x] Demonstrate checkpoint recovery while transactions are in flight  
  **Result:** repeated runs restored pending state and reached `Ready`.

- [x] Determine whether one Solizone block per inscription is realistic  
  **Result:** yes from a payload-capacity perspective.

- [x] Determine whether chunking is immediately required  
  **Result:** no, not for payloads ≤1 MiB.

- [x] Produce concrete constraints for Experiment 003  
  **Result:** see below.

---

# 17. Design inputs for Experiment 003

## 1. Start with one block → one Logos message

```text
Solizone block
      ↓
canonical serialization
      ↓
single Logos inscription
```

Do not introduce chunking in the first canonical block format.

## 2. Soft payload target

```text
recommended initial publication target:
≤ 256 KiB
```

This is an engineering recommendation, **not a Logos protocol limit**.

## 3. Defensive tested ceiling

```text
1 MiB
```

This is the largest payload actually tested.

Do not describe it as Logos's maximum.

## 4. Use compact deterministic serialization

Avoid verbose JSON as the canonical publication representation.

The block should include only fields required for:

```text
block identity
parent linkage
execution commitment
transaction commitment
receipt commitment
state commitment
versioning
future extension
```

## 5. Add publication backpressure

Do not allow the first Solizone block producer to emit an unlimited number of blocks while prior Logos publications remain pending.

## 6. Abstract funding

Funding should not remain a hardcoded public key in production architecture.

## 7. Keep publication states separate

Solizone must distinguish:

```text
produced
published
mempool accepted
adopted / included
finalized
```

Experiment 002 proved that mempool acceptance must not be treated as finality.

---

# 18. Relationship to Experiment 003

Experiment 003 will define the first real Solizone block structure.

Starting shape:

```text
SolizoneBlock
├── version
├── height
├── parent_hash
├── timestamp
├── tx_root
├── state_root
├── receipts_root
├── execution metadata
└── optional block body / transaction representation
```

The exact fields and serialization format will be determined in Experiment 003.

Experiment 002 provides the publication envelope:

```text
one block per inscription:
viable

tested payload:
≤ 1 MiB

recommended initial operating target:
≤ 256 KiB

chunking:
not initially required

finalization:
must remain explicitly tracked
```

---

# 19. Relationship to future EVM development

Eventually:

```text
Ethereum transactions
        ↓
Solizone mempool
        ↓
REVM
        ↓
Solizone state transition
        ↓
Solizone block
        ↓
canonical serialization
        ↓
publication rules discovered in Experiment 002
        ↓
Zone SDK
        ↓
Logos
```

Experiment 002 therefore defines the practical boundary between EVM execution and Logos publication.

---

# 20. Current Solizone R&D status

```text
Experiment 001
Minimal Zone SDK lifecycle
        ✅ COMPLETE
             │
             ▼
Experiment 002
Publication & inscription constraints
        ✅ COMPLETE FOR DESIGN GATE
             │
             ▼
Experiment 003
Canonical Solizone block
        ⏭ NEXT
             │
             ▼
Experiment 004
Minimal EVM runtime
        ⏳
```

---

# 21. Final technical conclusion

> We tested Logos Zone publication with structured benchmark payloads ranging from 128 bytes to 1 MiB. Every tested size was successfully accepted by the Logos mempool once fee and spendable funding constraints were satisfied. No payload-size rejection was observed within the 1 MiB experiment ceiling. The largest test, 1 MiB, reached `MempoolPending` in 4.413 seconds. Measured large-payload fee estimates followed `14 × payload_bytes + 6,012` for the transaction shape used by this experiment. Multiple transactions could remain pending and survive checkpoint recovery, although accumulated pending transactions reduced immediately spendable wallet liquidity. Because the fresh Logos node remained in `Bootstrapping` mode with `LIB slot = 0`, inclusion and finalization latency could not be measured and are explicitly deferred. Based on these results, Experiment 003 should begin with a one-Solizone-block-per-inscription model, compact deterministic serialization, a soft operating target of ≤256 KiB, and a tested defensive ceiling of 1 MiB.

---

# 22. Experiment status

```text
Experiment:
002 - Logos Publication & Inscription Constraints

Status:
✅ COMPLETE FOR EXPERIMENT 003 DESIGN

Payload benchmark:
128 B → 1 MiB accepted

Largest tested payload:
1 MiB

Recommended initial operating target:
≤ 256 KiB

Payload-size rejection observed:
No, not within tested range

Finalization benchmark:
Deferred - local node remained Bootstrapping with LIB = 0

Final pending checkpoint:
14 transactions

Depends on:
Experiment 001 ✅

Unblocks:
Experiment 003 - Canonical Solizone Block

EVM integration:
NOT STARTED BY DESIGN
```

---

# Final objective - outcome

By the end of Experiment 002, we needed enough information to answer:

> **How should a Solizone block be represented and published through Logos?**

The current answer is:

> Start with **one compact canonical Solizone block per Logos inscription**, keep the normal payload comfortably below the tested upper range, target **≤256 KiB initially**, preserve explicit publication/finality states, and keep chunking as a future fallback rather than an initial requirement.

This is sufficient to begin Experiment 003.
