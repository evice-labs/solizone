# Experiment 004 - Logos Storage-Backed Solizone State Snapshots

> **Status:** Research proposal / not implemented  
> **Project:** Solizone EVM  
> **Experiment:** 004  
> **Purpose:** Investigate whether Logos Storage can act as a decentralised, content-addressed snapshot distribution layer for Solizone EVM state.
>
> ⚠️ **Disclaimer:** Solizone is an independent research prototype and is not an official Logos project. This document describes a research hypothesis, not a finalized architecture.

---

## 1. Research Question

Solizone currently executes Solidity contracts through REVM, derives execution-backed state and receipts, builds a canonical Solizone block, publishes that block through the Logos Zone SDK, and observes Logos finality.

The next major implementation phase requires **persistent Solizone EVM state**.

This experiment asks a related but separate question:

> **Can Logos Storage be used to distribute verifiable snapshots of Solizone's EVM state, allowing new or recovering Solizone nodes to bootstrap from a recent state snapshot instead of reconstructing the entire state from genesis?**

The proposed idea is **not** to make REVM query Logos Storage for every account or storage read.

The proposed architecture is:

```text
                         SOLIZONE LIVE EXECUTION

Ethereum transaction
        ↓
Solizone admission
        ↓
REVM
        ↓
Local persistent StateBackend
        ↓
Updated Solizone EVM state
        ↓
state_root
        ↓
Canonical Solizone block
        ↓
Logos publication / finality


                    PERIODIC STATE SNAPSHOT PATH

Finalized / checkpointed Solizone state
        ↓
Deterministic snapshot serialization
        ↓
Optional compression
        ↓
Logos Storage upload
        ↓
CID
        ↓
Snapshot metadata
        │
        ├── Solizone block height
        ├── Solizone block hash
        ├── state_root
        ├── snapshot CID
        ├── snapshot format version
        └── optional size / compression metadata
```

A new Solizone node could potentially perform:

```text
Find trusted/finalized Solizone checkpoint
        ↓
Obtain snapshot CID
        ↓
Download snapshot from Logos Storage
        ↓
Decode snapshot
        ↓
Recompute Solizone state_root
        ↓
Compare with committed block state_root
        ↓
ACCEPT / REJECT snapshot
        ↓
Replay only blocks after snapshot height
        ↓
Reach current Solizone head
```

---

## 2. Why Explore This?

A blockchain node needs more than block history.

To execute the next transaction it needs the **current state**, including:

```text
accounts
├── balances
├── nonces
├── contract bytecode / code hashes
└── contract storage
```

The simplest Solizone implementation can keep this state in a local embedded database.

That is still the correct plan for **live execution**.

However, a new node joining much later may otherwise need to reconstruct state by executing a potentially very long sequence of historical blocks.

A verifiable state snapshot could provide a faster bootstrap path.

Instead of:

```text
Genesis
  ↓
Block 1
  ↓
Block 2
  ↓
Block 3
  ↓
...
  ↓
Block 1,000,000
  ↓
Current state
```

a node could potentially do:

```text
Verified checkpoint at Block 990,000
        ↓
Download state snapshot
        ↓
Verify state_root
        ↓
Replay Blocks 990,001 → 1,000,000
        ↓
Current state
```

This does not eliminate verification.

It changes **where verification begins**.

---

## 3. Important Architectural Principle

### Logos Storage should initially be treated as a snapshot distribution layer, not Solizone's live database.

Recommended:

```text
REVM
  ↓
Local persistent StateBackend
  ↓
Fast account / storage reads and writes
```

and separately:

```text
Solizone state
  ↓
Periodic snapshot
  ↓
Logos Storage
```

Not recommended for the initial design:

```text
REVM storage read
      ↓
network request
      ↓
Logos Storage
      ↓
contract execution continues
```

Live EVM execution requires predictable low-latency state access.

Introducing network retrieval into normal EVM account/storage access would add additional latency, availability dependencies, and execution failure modes.

Experiment 004 therefore focuses on:

- state snapshots,
- state distribution,
- node bootstrap,
- recovery,
- archival assistance,
- independent state verification.

---

## 4. Relationship to Current Solizone Architecture

Current Solizone responsibility:

```text
Solizone
├── EVM execution
├── accounts
├── balances
├── nonces
├── contract bytecode
├── contract storage
├── transactions
├── receipts
├── gas accounting
├── state commitments
└── block semantics
```

Current Logos responsibility:

```text
Logos
├── Zone publication
├── shared ordering
├── channel sequencing
├── consensus
├── data availability for published Zone history
└── finality
```

Experiment 004 investigates an additional optional component:

```text
Logos Storage
└── decentralised distribution of Solizone state snapshots
```

The proposed complete picture becomes:

```text
                        ETHEREUM TOOLING
                              ↓
                     Ethereum JSON-RPC
                              ↓
                     Solizone Mempool
                              ↓
                     Block Producer
                              ↓
                            REVM
                              ↓
                  Local Persistent State
                       ↙             ↘
                state_root        snapshot
                    ↓                ↓
             Solizone Block    Logos Storage
                    ↓                ↓
              Logos Zone           CID
              Publication            │
                    ↓                │
             Logos Finality          │
                    │                │
                    └──── checkpoint metadata
```

---

## 5. Logos Storage Properties Relevant to This Experiment

At the time of writing, Logos Storage is documented as a decentralised file-sharing protocol provided as a Logos module.

Relevant properties to investigate:

### Content identifiers

Uploaded files receive a **CID**.

Conceptually:

```text
snapshot bytes
      ↓
content identifier
      ↓
CID
```

This is useful for immutable snapshot artifacts because changing the underlying content should result in a different content identity.

### Organic replication

Logos Storage currently describes replication as **organic**.

A file initially exists on the uploading node.

When another node downloads the file, that node also stores and serves it, increasing the number of replicas.

Conceptually:

```text
Snapshot uploaded
    replica count ≈ 1

Node B downloads
    ↓
more availability

Node C downloads
    ↓
more availability
```

This is important for Solizone because snapshot persistence must **not be assumed automatically**.

### Availability limitation

The current Logos Storage documentation explicitly notes a fundamental persistence limitation of organic replication:

If no other nodes replicate a file and the original hosting node disappears, the content may become unavailable.

For Solizone, this means:

> **A CID proves the identity of content, but a CID alone must not be treated as a guarantee that the content will remain retrievable forever.**

This is one of the central questions of Experiment 004.

### Network discovery and reachability

Logos Storage uses peer discovery and content lookup across its network.

Node reachability, NAT, relays, DHT discovery, firewalls, peer availability and bandwidth may therefore affect snapshot retrieval performance.

These conditions must be tested rather than assumed.

---

## 6. Proposed Snapshot Model

A Solizone snapshot should represent enough state to continue EVM execution from a specific block.

Possible logical contents:

```text
Snapshot
├── format_version
├── chain_id
├── block_height
├── block_hash
├── state_root
│
├── accounts[]
│   ├── address
│   ├── balance
│   ├── nonce
│   ├── code_hash
│   ├── bytecode
│   └── storage[]
│       ├── slot
│       └── value
│
└── metadata
```

Potential canonical rule:

```text
same Solizone state
        +
same snapshot format version
        =
same canonical snapshot bytes
```

This is desirable for:

- reproducibility,
- deterministic hashing,
- independent verification,
- debugging,
- duplicate detection.

The exact snapshot format is intentionally **not selected yet**.

Candidates to research:

- custom canonical binary format,
- CBOR,
- SSZ-like deterministic encoding,
- canonical JSON only for early experiments,
- database export format,
- chunked manifest + independently addressable state segments.

For a real implementation, the format should be versioned.

---

## 7. Snapshot Verification Model

A snapshot should never be trusted only because it came from Logos Storage.

The trust model should be:

```text
Logos Storage
     =
transport / discovery / replication

Solizone state_root
     =
state integrity commitment

Logos finality
     =
finality of the Solizone block carrying that commitment
```

A node should perform:

```text
download snapshot
        ↓
decode
        ↓
reconstruct Solizone state
        ↓
compute_state_root(state)
        ↓
expected state_root from checkpoint block
        ↓
compare
```

If:

```text
computed_state_root == block.state_root
```

the snapshot contents correspond to the state committed by that block under Solizone's commitment algorithm.

If not:

```text
REJECT SNAPSHOT
```

---

## 8. How Should the CID Be Connected to a Solizone Block?

This is an open design question.

### Option A - CID inside the canonical block header

Example:

```text
SolizoneBlockHeader
├── ...
├── state_root
└── state_snapshot_cid
```

#### Advantage

The block cryptographically commits directly to the snapshot reference.

#### Disadvantage

The existing `SZB1` header is currently fixed-width.

Adding a CID would alter the canonical block format and block hash semantics.

This should not be done casually.

---

### Option B - Snapshot commitment outside the fixed header

Possible approaches:

```text
system transaction
```

or:

```text
block auxiliary metadata
```

or:

```text
snapshot registry keyed by block hash
```

This could preserve the current block header while the research is still evolving.

---

### Option C - Commit only a fixed-size snapshot hash

Instead of storing the variable-length CID directly:

```text
snapshot_hash = Keccak256(snapshot metadata / CID)
```

The full CID could live in an external mapping.

This preserves a fixed-size commitment but introduces another lookup step.

---

### Research rule

**Do not change `SZB1` yet solely for Experiment 004.**

First prove:

1. snapshots are useful,
2. snapshots are reproducible,
3. Logos Storage can distribute them reliably enough,
4. verification works,
5. snapshot creation and retrieval costs are acceptable.

Only then decide whether snapshot references belong in the canonical block format.

---

## 9. When Should Solizone Create a Snapshot?

Possible strategies:

### Every block

```text
Block N
   ↓
Snapshot N
```

Probably too expensive at scale.

### Every N blocks

```text
Block 0
Snapshot

Block 1
Block 2
...
Block 99

Block 100
Snapshot
```

Simple and deterministic.

### Time-based

```text
one snapshot every X minutes / hours
```

### State-growth based

Generate a snapshot after state changes by a configured threshold.

### Finality-based

Only advertise a snapshot as a trusted bootstrap candidate after the corresponding Solizone block has reached Logos finality.

Likely initial research direction:

```text
snapshot every N blocks
        +
only expose snapshots tied to finalized Solizone blocks
```

---

## 10. Potential New-Node Bootstrap Flow

Future Solizone bootstrap could become:

```text
Start Solizone node
      ↓
connect to Logos
      ↓
discover finalized Solizone history
      ↓
select recent accepted checkpoint
      ↓
obtain snapshot CID
      ↓
download from Logos Storage
      ↓
verify snapshot
      ↓
load into local StateBackend
      ↓
replay post-snapshot Solizone blocks
      ↓
reach current head
      ↓
begin normal operation
```

Fallback:

```text
snapshot unavailable
        ↓
try another provider / replica
        ↓
try older snapshot
        ↓
if necessary:
replay chain from an earlier trusted state
```

Snapshot availability should therefore be an **optimization**, not a single point of failure.

---

## 11. Potential Advantages for Solizone

### 11.1 Faster node bootstrap

New nodes may avoid executing the full Solizone history from genesis.

Instead:

```text
download verified state
+
replay recent blocks
```

---

### 11.2 Easier crash recovery and migration

A node could restore a known state snapshot and replay only the missing tail of the chain.

Potential uses:

- machine failure,
- corrupted local database,
- moving a node to a new server,
- testing another Solizone client,
- disaster recovery.

---

### 11.3 Reduced requirement for every node to behave as an archive node

Live execution nodes could focus on:

```text
current state
+
recent history
```

while older state snapshots may be independently replicated by interested operators.

This requires a deliberate pruning and archival policy.

---

### 11.4 Decentralised snapshot distribution

Without a decentralised mechanism, Solizone might eventually rely on:

```text
snapshots.solizone.example
```

or another project-operated server.

Logos Storage creates the possibility of:

```text
Node A
Node B
Node C
Node D
   ↘
same snapshot CID
```

instead of requiring one project-controlled download source.

---

### 11.5 Content-addressed state artifacts

Each snapshot can be identified by content rather than a mutable filename.

Instead of:

```text
latest-state.db
```

Solizone can reason about:

```text
Snapshot for Block N
Block hash = H
State root = R
CID = C
```

---

### 11.6 Independently verifiable state distribution

A snapshot provider does not need to be trusted to tell the truth if the receiver can reconstruct the state commitment and compare it against a finalized Solizone block.

This could separate:

```text
Who serves state?
```

from:

```text
What state is accepted?
```

---

### 11.7 Cleaner Logos-native architecture

Solizone could make use of different Logos components for different jobs:

```text
REVM
    execution

Solizone
    state semantics + blocks

Logos blockchain / Zone stack
    ordering + publication + finality

Logos Storage
    large snapshot distribution
```

The usefulness of this composition must be demonstrated experimentally.

---

### 11.8 Potential support for independent replay / verification nodes

A verifier could obtain:

```text
checkpoint block
+
snapshot
+
later blocks
```

and independently reconstruct current Solizone state.

---

### 11.9 Potential basis for historical state services

If old snapshots remain sufficiently replicated, additional services could potentially retrieve historical Solizone state without every live execution node preserving all historical state locally.

This is a future possibility, not an Experiment 004 requirement.

---

## 12. Disadvantages and Risks

### 12.1 Organic replication does not guarantee persistence

This is the most important current concern.

A snapshot that has only one replica can disappear if that node disappears.

Solizone would need to research:

- minimum replication policy,
- dedicated replica operators,
- periodic availability checks,
- re-replication,
- snapshot retention policy.

---

### 12.2 CID availability and CID correctness are different problems

Content addressing can help identify data.

It does not automatically guarantee:

```text
someone is currently online and serving that CID
```

Solizone therefore needs separate concepts:

```text
integrity
```

and:

```text
availability
```

---

### 12.3 Snapshot size may become very large

EVM state can grow continuously.

Questions:

- How large is a snapshot after 10k accounts?
- 100k?
- 1M?
- 10M?
- How does contract storage dominate size?
- How long does upload/download take?
- Does compression help enough?

---

### 12.4 Snapshot creation may interrupt block production

If snapshot creation requires copying a large mutable state database, it could interfere with execution.

Potential solutions to research:

- copy-on-write snapshots,
- database checkpoints,
- MVCC,
- background serialization,
- immutable state generations.

---

### 12.5 Network retrieval may be slower than expected

Peer discovery, NAT, relay paths, bandwidth, Mix routing and replica location can influence retrieval time.

Bootstrapping must tolerate slow or unavailable peers.

---

### 12.6 Additional implementation complexity

Solizone would need:

- snapshot encoding,
- snapshot indexing,
- CID tracking,
- availability monitoring,
- restore logic,
- verification,
- fallback logic,
- snapshot version migration,
- pruning policy.

---

### 12.7 State format migration

Future Solizone versions may change:

- account encoding,
- state commitment,
- database layout,
- EVM rules,
- snapshot format.

Every snapshot should therefore include explicit version information.

---

### 12.8 Malicious or malformed snapshot input

Even if invalid data ultimately fails `state_root` verification, decoders must still defend against:

- extremely large allocations,
- malformed lengths,
- duplicate accounts,
- duplicate storage slots,
- decompression bombs,
- parser crashes,
- unexpected versions.

---

### 12.9 Snapshot freshness

A snapshot may be valid but old.

A node may still have a large amount of replay work after loading it.

Solizone needs a checkpoint selection policy.

---

### 12.10 Privacy considerations

Public blockchain state is generally public by design, but Logos Storage has privacy-oriented networking characteristics.

Research should distinguish:

```text
content confidentiality
```

from:

```text
provider/downloader unlinkability
```

A Solizone snapshot should not accidentally contain:

- private node keys,
- API secrets,
- local metadata,
- database credentials,
- temporary application data.

Only consensus-relevant Solizone state should be serialized.

---

## 13. Comparison to Ethereum Concepts

Experiment 004 should study Ethereum's state synchronization architecture before choosing a Solizone-specific solution.

### Ethereum full replay

Conceptually:

```text
Genesis
  ↓
download blocks
  ↓
execute all transactions
  ↓
reconstruct current state
```

Strong verification properties, but expensive for a mature chain.

---

### Geth Snap Sync

Geth's Snap Sync is a major research reference.

At a high level, Geth can obtain a recent blockchain/state view without independently replaying every transaction from genesis.

Important concepts to study:

- recent checkpoint / synchronization target,
- state trie,
- state ranges,
- range proofs,
- rebuilding authenticated state locally,
- state healing while the chain continues moving,
- transition from initial sync to normal block-by-block operation.

Solizone does **not** currently use Ethereum's Merkle Patricia Trie, so Snap Sync cannot simply be copied.

The useful research question is:

> **Which principles from Snap Sync should Solizone adopt for its own state commitment model?**

---

### Ethereum archive nodes

Research:

- current-state nodes vs historical-state/archive nodes,
- storage growth,
- pruning,
- historical state regeneration,
- why not every node retains every historical state.

---

## 14. Related Topics to Research

### Ethereum state and synchronization

- Geth Snap Sync
- Geth full sync
- Ethereum archive nodes
- Ethereum state trie
- Merkle Patricia Trie
- range proofs
- state healing
- checkpoint-based synchronization
- Ethereum pruning
- historical state reconstruction
- EIP-4444 / historical data expiry concepts
- state expiry research
- stateless Ethereum / witness concepts
- Verkle-tree-related state research

### Other Ethereum clients

Compare how different clients approach state and history:

- Reth
- Erigon
- Nethermind
- Besu

Research:

- snapshots,
- flat state,
- database layouts,
- static/history files,
- pruning,
- checkpoint restore,
- fast initial sync.

### L2 / rollup state synchronization

Research how rollups handle:

- node bootstrapping,
- state derivation,
- execution replay,
- snapshots,
- trusted/untrusted snapshot providers,
- finality anchors,
- sequencer history.

Potential ecosystems to compare:

- OP Stack
- Arbitrum
- Polygon
- zkSync
- Starknet

The goal is not to copy them, but to understand established tradeoffs.

### Content-addressed storage

Research:

- CID semantics,
- content addressing,
- immutable blobs,
- manifests,
- chunked files,
- deduplication,
- Merkle DAGs,
- IPFS concepts,
- BitTorrent-style replication,
- provider discovery.

### Decentralised storage availability

Compare:

- organic replication,
- incentivized storage,
- replication guarantees,
- storage proofs,
- repair / re-replication,
- pinning,
- retention policies,
- erasure coding.

Useful comparison areas may include:

- IPFS
- Filecoin
- Arweave
- Swarm
- Sia

These systems have different trust and availability models; no direct equivalence to Logos Storage should be assumed.

### Data availability vs storage

Research the distinction between:

```text
Data Availability
```

and:

```text
Long-term retrievable storage
```

A block being available when consensus validates it is not necessarily the same thing as a snapshot remaining retrievable years later.

### Database snapshotting

Research:

- RocksDB checkpoints
- LMDB snapshots
- redb
- SQLite WAL snapshots
- MVCC
- copy-on-write databases
- atomic snapshots
- incremental backups

### State serialization

Research:

- deterministic encoding,
- canonical encoding,
- schema versioning,
- compression,
- chunking,
- streaming decode,
- incremental snapshots,
- full snapshots,
- delta snapshots.

### Cryptographic verification

Research:

- Merkle commitments,
- Merkle proofs,
- range proofs,
- chunk-level commitments,
- manifest roots,
- authenticated data structures,
- snapshot root vs EVM state root,
- proof-carrying state chunks.

### Network behaviour

Research:

- DHT lookup performance,
- NAT traversal,
- relays,
- peer discovery,
- replication factor,
- cold content,
- hot content,
- bandwidth asymmetry,
- network partitions,
- peer churn.

---

## 15. Core Research Questions

Experiment 004 should answer at least the following.

### Architecture

1. Should Solizone snapshot the raw database or a canonical logical state representation?
2. Can two independent nodes produce byte-identical snapshots for the same state?
3. Should the snapshot CID be consensus-critical?
4. Where should the CID be committed?
5. Should snapshot creation happen before or after Logos finality?
6. How should Solizone discover the newest valid snapshot?
7. How should old snapshots be pruned?
8. Should snapshots be full or incremental?
9. How should snapshot format upgrades work?

### Verification

10. Can a downloaded snapshot reproduce the exact Solizone `state_root`?
11. Can verification be streamed instead of loading the full snapshot into memory?
12. Can individual chunks be verified independently?
13. Does Solizone need a snapshot-specific Merkle tree?
14. Can the existing flat state commitment scale sufficiently for snapshot verification?

### Availability

15. How many replicas are required before Solizone considers a snapshot "available"?
16. Can Solizone measure replica count reliably?
17. What happens when the original publisher disappears?
18. How quickly does content become unavailable under peer churn?
19. Can replication groups be built using existing Logos components?
20. Who is responsible for keeping important snapshots alive?

### Performance

21. What is the maximum practical snapshot size?
22. What upload throughput can Logos Storage sustain?
23. What download throughput can it sustain?
24. What is time-to-first-byte?
25. How does retrieval perform with one replica vs many replicas?
26. What is the effect of NAT / relay use?
27. What is the effect of Mix routing?
28. How much CPU does verification consume?
29. How much disk I/O does snapshot creation consume?
30. Can snapshots be generated without delaying block execution?

### Recovery

31. Can a node restore state after deleting its complete local state DB?
32. Can it restore after an unclean shutdown?
33. What if the newest snapshot is unavailable?
34. Can it automatically fall back to an older snapshot?
35. Can it recover if only some snapshot chunks are available?
36. What happens if a snapshot download is interrupted?

---

# 16. Logos Storage Stress-Test Plan

The goal is to measure Logos Storage rather than assume its suitability.

All tests should record:

```text
snapshot_size
chunk_size
upload_time
download_time
time_to_first_byte
average_throughput
peak_throughput
CPU
memory
disk_usage
peer_count
replica_count if observable
network topology
success/failure
retry count
verification time
```

---

## Test 001 - Basic Round Trip

```text
Solizone state
   ↓
snapshot
   ↓
upload
   ↓
CID
   ↓
download from second node
   ↓
verify bytes
   ↓
recompute state_root
```

Pass condition:

```text
downloaded snapshot == uploaded snapshot
computed state_root == expected state_root
```

---

## Test 002 - Snapshot Size Ladder

Test:

```text
1 MiB
10 MiB
50 MiB
100 MiB
250 MiB
500 MiB
1 GiB
2 GiB
5 GiB
10 GiB
```

Continue upward only while sensible for the environment.

Measure:

- upload time,
- download time,
- memory,
- CPU,
- disk usage,
- timeout behavior,
- transfer stability.

Goal:

Determine the practical size envelope for Solizone snapshots.

---

## Test 003 - Chunk-Size Matrix

The Logos Storage examples currently use a default chunk size of 65,536 bytes.

Test multiple values if the API permits:

```text
16 KiB
32 KiB
64 KiB
128 KiB
256 KiB
512 KiB
1 MiB
```

Measure:

- transfer throughput,
- memory,
- failure/retry behavior,
- latency,
- manifest overhead.

---

## Test 004 - Single Replica Failure

```text
Node A uploads snapshot
        ↓
CID generated
        ↓
stop Node A
        ↓
Node B attempts download
```

Purpose:

Directly demonstrate the availability consequence of having only one replica.

---

## Test 005 - Replication Survival

```text
Node A uploads
        ↓
Nodes B, C, D download
        ↓
stop Node A
        ↓
Node E downloads
```

Then progressively stop replicas.

Goal:

Measure actual availability as replica count falls.

---

## Test 006 - Cold Snapshot Retrieval

Upload a snapshot.

Do not access it for:

```text
1 hour
6 hours
24 hours
3 days
7 days
```

Then attempt retrieval from another node.

Longer-duration testing should be added if infrastructure permits.

Goal:

Understand persistence of low-interest content.

---

## Test 007 - Publisher Disappearance

```text
Publisher uploads
        ↓
other replicas download
        ↓
publisher disk erased / node removed
        ↓
fresh node attempts retrieval
```

This is different from a temporary shutdown.

---

## Test 008 - Peer Churn

Continuously start and stop storage peers during a large snapshot transfer.

Example:

```text
10 peers
randomly remove / restore peers
while 1–5 GiB snapshot downloads
```

Measure:

- transfer continuation,
- restart requirements,
- corruption,
- throughput collapse.

---

## Test 009 - Interrupted Download / Resume Behaviour

Kill the downloading process at:

```text
10%
25%
50%
75%
95%
```

Restart and attempt recovery.

Determine:

- whether partial data can be reused,
- whether transfer restarts from zero,
- whether final CID/data verification still succeeds.

---

## Test 010 - Interrupted Upload

Terminate the uploader at different stages.

Verify:

- incomplete manifests are not treated as valid snapshots,
- partially available data cannot be mistaken for a completed checkpoint.

---

## Test 011 - Concurrent Bootstrap

Simulate:

```text
1
5
10
25
50
100
```

new nodes requesting the same snapshot.

Measure:

- serving-node CPU,
- bandwidth,
- aggregate throughput,
- per-client latency,
- failure rate.

This is important for recovery after a widespread outage or version upgrade.

---

## Test 012 - Multiple Snapshots Concurrently

Upload and retrieve many different snapshots simultaneously.

Example:

```text
Snapshot 1000
Snapshot 2000
Snapshot 3000
...
```

Goal:

Understand whether many historical snapshots create discovery or resource pressure.

---

## Test 013 - NAT / Relay Performance

Compare:

```text
publicly reachable node
vs
NATed node using relay
```

Measure upload/download performance and reliability.

The current Logos Storage documentation notes that relayed connectivity may have lower performance than direct reachability.

---

## Test 014 - Network Partition

Create:

```text
Group A |X| Group B
```

with replicas split between groups.

Test:

- snapshot discovery,
- retrieval failure,
- recovery after partition heals,
- duplicate/redundant transfer behavior.

---

## Test 015 - No Bootstrap / Discovery Failure

Test fresh nodes when:

- bootstrap node unavailable,
- DHT discovery degraded,
- discovery port blocked,
- TCP transfer port blocked.

Goal:

Document operational failure modes clearly.

---

## Test 016 - Corrupted Local Replica

Manually corrupt stored snapshot bytes on one test node.

A fresh node retrieves the snapshot.

Expected:

```text
content / CID verification or
Solizone state_root verification
must reject corrupted data
```

Never allow corrupted state to become active execution state.

---

## Test 017 - Malformed Snapshot

Create intentionally malformed snapshot files:

- truncated account entry,
- invalid lengths,
- duplicate addresses,
- duplicate storage slots,
- unsupported version,
- invalid bytecode length,
- invalid metadata.

Expected:

```text
clean error
no panic
no excessive allocation
no partial state activation
```

---

## Test 018 - Decompression Bomb

If compression is introduced, test highly compressible malicious input where:

```text
small compressed file
        ↓
huge decompressed output
```

Snapshot decoding must enforce explicit resource limits.

---

## Test 019 - State-Root Mismatch

Upload a syntactically valid snapshot that represents the wrong state.

Expected:

```text
CID retrieval succeeds
        ↓
snapshot decode succeeds
        ↓
computed state_root != block.state_root
        ↓
REJECT
```

This test validates the separation between storage availability and blockchain truth.

---

## Test 020 - Wrong-Block Snapshot

Provide:

```text
Snapshot from Block 100
```

while metadata claims:

```text
Block 101
```

Expected:

verification failure through block hash / state-root checks.

---

## Test 021 - Snapshot Version Upgrade

Create:

```text
Snapshot format V1
Snapshot format V2
```

Verify:

- V1 remains readable if supported,
- unsupported versions fail explicitly,
- no ambiguous decoding occurs.

---

## Test 022 - Restore into Empty Solizone Node

Delete:

```text
local state database
```

Keep only:

```text
Solizone finalized history / checkpoint reference
```

Then:

```text
download snapshot
        ↓
restore StateBackend
        ↓
execute next transaction
```

This is one of the most important Experiment 004 tests.

---

## Test 023 - Restore + Replay

```text
Snapshot at Block 1000
        ↓
restore
        ↓
replay Blocks 1001–1100
        ↓
compare with continuously-running node
```

Pass condition:

```text
state_root(restored node)
==
state_root(original node)
```

---

## Test 024 - Deterministic Snapshot Generation

Two independent Solizone nodes at the same block independently create a snapshot.

Desired property:

```text
same state
        ↓
same canonical bytes
        ↓
same CID
```

If this is not achievable or desirable, document exactly why.

---

## Test 025 - Snapshot Generation Under Load

While Solizone continuously executes blocks:

```text
transactions
transactions
transactions
```

create snapshots periodically.

Measure:

- block execution latency,
- state DB lock time,
- CPU,
- memory,
- disk I/O,
- missed block-production deadlines.

---

## Test 026 - Storage Exhaustion

Run storage nodes close to disk capacity.

Test:

- upload failure,
- download failure,
- cleanup behavior,
- incomplete snapshots,
- recovery after disk space becomes available.

---

## Test 027 - Bandwidth-Limited Bootstrap

Throttle network to:

```text
1 Mbps
5 Mbps
10 Mbps
50 Mbps
100 Mbps
```

Measure time to bootstrap different snapshot sizes.

This helps determine practical requirements for operators.

---

## Test 028 - High-Latency Network

Introduce:

```text
50 ms
100 ms
250 ms
500 ms
1000 ms
```

latency.

Measure DHT discovery and transfer performance.

---

## Test 029 - Packet Loss

Simulate:

```text
1%
2%
5%
10%
```

packet loss.

Measure transfer completion and retries.

---

## Test 030 - Long-Term Replica Retention

Keep a set of checkpoints alive for an extended test period.

Periodically test:

```text
CID exists?
download succeeds?
hash verifies?
state_root verifies?
number of reachable providers?
```

Goal:

Measure whether Solizone would need an explicit replication service.

---

# 17. Solizone-Specific State Stress Tests

Logos Storage performance alone is not enough.

Solizone snapshot generation itself must be tested.

---

## Account Count

Generate states containing:

```text
1,000 accounts
10,000 accounts
100,000 accounts
1,000,000 accounts
```

Measure snapshot size and generation time.

---

## Contract Count

Generate:

```text
1,000 contracts
10,000 contracts
100,000 contracts
```

with different bytecode sizes.

---

## Storage-Heavy Contracts

Test contracts containing:

```text
100 slots
1,000 slots
10,000 slots
100,000 slots
```

per contract.

EVM storage is likely to dominate certain state workloads.

---

## Mixed Workloads

Model realistic mixtures:

```text
EOAs
ERC-20-like contracts
NFT-like mappings
DEX-like state
large mapping contracts
small contracts
```

---

## State-Churn Workload

Generate many writes where old values are continuously replaced.

Research whether full snapshots unnecessarily resend unchanged state and whether incremental snapshots become worthwhile.

---

# 18. Snapshot Strategies to Compare

## Full snapshot

Every checkpoint contains the complete state.

### Pros

- simplest restore,
- simplest verification model,
- no dependency chain between snapshots.

### Cons

- largest transfer,
- repeated unchanged data.

---

## Incremental snapshot

Store only changes since an earlier snapshot.

```text
Full Snapshot A
      ↓
Delta B
      ↓
Delta C
```

### Pros

- smaller updates.

### Cons

- restore depends on multiple artifacts,
- retention becomes harder,
- losing one delta may break recovery,
- verification is more complex.

---

## Chunked full snapshot

```text
Snapshot manifest
├── accounts chunk 0
├── accounts chunk 1
├── accounts chunk 2
├── storage chunk 0
└── storage chunk 1
```

Potential advantages:

- parallel download,
- partial retry,
- chunk-level deduplication,
- streaming verification.

This may be the most interesting long-term direction if whole-state snapshots become large.

---

# 19. Snapshot Availability Policy - Open Research

Solizone may eventually require rules such as:

```text
Do not advertise snapshot
until replication target >= N
```

but this only works if replica availability can be observed or enforced reliably.

Alternative:

```text
Solizone-operated archival nodes
+
community replicas
+
periodic availability probes
```

Another possibility:

```text
replication groups coordinated through Logos Messaging
```

The Logos Storage documentation mentions replication groups as a possible approach but does not currently present them as an out-of-the-box persistence guarantee.

Experiment 004 should determine whether Solizone needs its own small **Snapshot Replicator** service.

Example:

```text
SnapshotReplicator
├── watches finalized Solizone checkpoints
├── downloads required snapshot CIDs
├── verifies them
├── keeps configured retention window
└── periodically checks availability
```

---

# 20. Security Model

A malicious storage peer should not be able to change accepted Solizone state.

Expected security boundary:

```text
malicious snapshot
        ↓
download
        ↓
verify against finalized state_root
        ↓
reject
```

However, malicious peers can still potentially attack:

- availability,
- bandwidth,
- CPU,
- memory,
- parser behavior,
- decompression,
- peer discovery.

Therefore snapshot restoration should use:

- strict size limits,
- streaming parsers,
- bounded allocations,
- canonical validation,
- version checks,
- state-root verification,
- atomic activation.

---

## Atomic Activation

Never do:

```text
download snapshot
        ↓
write directly into live DB
        ↓
discover corruption halfway through
```

Prefer:

```text
download into staging area
        ↓
validate
        ↓
compute state_root
        ↓
verify
        ↓
atomically promote as active state
```

---

# 21. Snapshot Lifecycle

Possible state machine:

```text
CREATING
   ↓
SERIALIZED
   ↓
UPLOADING
   ↓
CID_AVAILABLE
   ↓
VERIFYING
   ↓
AVAILABLE
   ↓
CHECKPOINT_FINALIZED
   ↓
BOOTSTRAP_ELIGIBLE
   ↓
RETIRED
```

Failure states:

```text
SNAPSHOT_CREATION_FAILED
UPLOAD_FAILED
UNAVAILABLE
CORRUPTED
ROOT_MISMATCH
UNSUPPORTED_VERSION
REPLICATION_BELOW_TARGET
```

---

# 22. Metrics Experiment 004 Should Produce

At the end of the experiment we should know:

```text
snapshot generation time
snapshot raw size
snapshot compressed size
compression ratio
upload throughput
download throughput
verification throughput
restore time
replay-after-restore time
CPU usage
memory usage
disk usage
replica survival
cold-CID availability
concurrent-bootstrap performance
NAT / relay penalty
Mix-network penalty if enabled
```

And most importantly:

```text
Full replay time
        vs
Snapshot bootstrap + replay-tail time
```

That comparison determines whether this architecture actually improves Solizone node operation.

---

# 23. Proposed Experiment Phases

## Phase A - Understand Logos Storage

No Solizone integration yet.

Prove:

```text
file
 ↓
upload
 ↓
CID
 ↓
second node
 ↓
download
 ↓
identical file
```

Then run basic failure tests.

---

## Phase B - Synthetic Solizone Snapshot

Create a deterministic synthetic state file.

Test:

```text
snapshot
 ↓
Logos Storage
 ↓
download
 ↓
hash verification
```

---

## Phase C - Real REVM State Snapshot

Use the Solizone `StateBackend` work from the persistent-state milestone.

```text
REVM state
 ↓
canonical snapshot
 ↓
Logos Storage
 ↓
restore
```

---

## Phase D - State-Root Verification

```text
Block N.state_root
        ↑
recompute from downloaded snapshot
```

This is the first security-critical milestone.

---

## Phase E - Fresh Node Bootstrap

Start a clean Solizone instance.

```text
no local EVM state
        ↓
checkpoint
        ↓
snapshot CID
        ↓
download
        ↓
verify
        ↓
restore
        ↓
execute next block
```

---

## Phase F - Snapshot + Replay Tail

```text
Snapshot Block N
      +
Blocks N+1 ... head
      ↓
Current state
```

Compare against a continuously running reference node.

---

## Phase G - Availability / Stress Tests

Run the Logos Storage stress-test matrix.

---

## Phase H - Architecture Decision

Choose one:

### Result 1 - Strong fit

Integrate Logos Storage snapshots into the Solizone node architecture.

### Result 2 - Useful optional feature

Support snapshots as an optional bootstrap / archival optimization.

### Result 3 - Not currently reliable enough

Keep local persistence and block replay as the core mechanism and revisit Logos Storage later.

A negative result is still a successful research outcome.

---

# 24. Exit Criteria

Experiment 004 is successful as research when we can answer:

- Can Logos Storage reliably move large Solizone state snapshots?
- What practical snapshot size is supported?
- What is the bootstrap performance improvement?
- Can snapshots always be verified against `state_root`?
- How many replicas are required for acceptable availability?
- What happens when providers disappear?
- What happens under NAT, relay, churn and network partitions?
- Can a fresh node restore and execute the next block correctly?
- Can two restored nodes converge on the same state?
- Can snapshot generation happen without materially harming block production?
- Does this architecture provide enough value to justify its complexity?

---

# 25. Success Demonstration

The ideal final demo for Experiment 004:

```text
NODE A
──────
run Solizone
deploy Counter
increment many times
produce Blocks 0 → 100
persist state
create snapshot at Block 100
upload snapshot to Logos Storage
obtain CID
publish / record checkpoint metadata


NODE B
──────
start with empty state directory
discover Block 100 checkpoint
obtain CID
download snapshot from Logos Storage
verify snapshot state_root
load state
read Counter value correctly
receive Block 101
execute Block 101
produce same state_root as Node A
```

Then:

```text
Stop original snapshot host
        ↓
Node B / another replica still serves snapshot
        ↓
NODE C bootstraps successfully
```

That would demonstrate both:

1. **verifiable state bootstrap**, and  
2. **replicated snapshot distribution**.

---

# 26. What This Experiment Is NOT Trying to Prove

Experiment 004 is not claiming that:

- Logos Storage is already guaranteed permanent storage,
- Logos Storage should replace Solizone's local StateBackend,
- Solizone has solved state sync,
- a CID automatically means a snapshot is available forever,
- Solizone snapshots are superior to Ethereum Snap Sync,
- adding snapshots automatically makes Solizone more decentralised,
- every Solizone block needs a state snapshot,
- the current Solizone block format should immediately change.

All of those require evidence.

---

# 27. Working Hypothesis

The working hypothesis is:

> **Solizone can keep a fast local persistent EVM state for live execution while using Logos Storage as a decentralised, content-addressed distribution layer for periodic verifiable state snapshots.**

If viable, this could provide:

```text
fast local execution
        +
faster new-node bootstrap
        +
decentralised state distribution
        +
independent state verification
        +
recovery / migration support
        +
less dependence on project-operated snapshot servers
```

while preserving the existing architectural boundary:

```text
Solizone
    owns execution and state semantics

Logos blockchain
    orders and finalizes Solizone history

Logos Storage
    optionally distributes large state artifacts
```

---

# 28. References / Starting Reading

## Solizone

- `../../solizone-evm/README.md`
- `../../DEVELOPMENT_ROADMAP.md`
- `../experiment-003-canonical-block/`

## Logos Storage

- Introduction to Logos Storage:  
  https://docs.logos.co/storage

- Run a Logos Storage node:  
  https://docs.logos.co/storage/get-started/run-logos-storage-node

- Logos Storage connectivity / NAT / discovery:  
  https://docs.logos.co/storage/concepts/connectivity

- Logos Storage troubleshooting:  
  https://docs.logos.co/storage/get-started/faq

- Build an app using the Storage API:  
  https://docs.logos.co/storage/build-app/build-cli-app-that-uses-storage-api

## Ethereum / Geth

- Geth sync modes / Snap Sync:  
  https://geth.ethereum.org/docs/fundamentals/sync-modes

- Ethereum archive nodes:  
  https://ethereum.org/developers/docs/nodes-and-clients/archive-nodes

Research should also expand beyond these sources into other Ethereum clients, rollup node synchronization, authenticated state structures, content-addressed storage systems and database snapshotting techniques.

---

# 29. Current Decision

For the main Solizone implementation, continue with:

```text
StateBackend abstraction
        ↓
local persistent EVM state
        ↓
parent-linked Solizone blocks
```

Experiment 004 should run **alongside that work**, not block it.

The persistent local `StateBackend` is required regardless of whether Logos Storage snapshots ultimately succeed.

If Experiment 004 succeeds, the future architecture becomes:

```text
                    ┌─────────────────────────┐
                    │   Local StateBackend    │
                    │ live authoritative state│
                    └───────────┬─────────────┘
                                │
                         periodic snapshot
                                │
                                ▼
                    ┌─────────────────────────┐
                    │     Logos Storage       │
                    │ snapshot distribution   │
                    └───────────┬─────────────┘
                                │ CID
                                ▼
                    ┌─────────────────────────┐
                    │ Solizone checkpoint     │
                    │ block hash + state_root │
                    └─────────────────────────┘
```

This experiment exists to determine whether that final layer is technically useful, operationally reliable, and worth making part of Solizone.

---

*Solizone is an independent research prototype and is not an official Logos project.*
