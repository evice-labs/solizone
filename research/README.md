Solizone R&D Baseline v0.01

This document is the base research reference for Solizone before implementation begins.

Its purpose is to keep the project grounded in verified Logos architecture, clearly separate assumptions from confirmed facts, and define the order in which critical technical questions should be answered.

This is not a production architecture specification and it is not a final development plan.

It is the working R&D baseline that should be updated as experiments produce stronger evidence.

1. Project Research Goal

Solizone is an experimental EVM-compatible Sovereign Zone built on the Logos Blockchain.

The first research objective is not to build a complete EVM chain.

The first objective is to validate that an EVM execution environment can correctly fit into the Logos Zone model and identify the minimum architecture needed for a working MVP.

The core research question is:

Can an independent EVM execution environment operate as a Logos Zone,
produce its own state and Zone blocks,
and publish that ordered history through the Logos Zone SDK / Mantle channel model?

2. Current Architecture Baseline

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

Solizone owns

EVM execution

Ethereum-style transactions

EVM accounts

contract bytecode and storage

balances and nonces

receipts and logs

Solizone block production

Ethereum-facing RPC

internal Zone state

Logos Zone SDK / Mantle owns or assists with

channel interaction

inscription publication

ordered Zone message history

pending/adopted/finalized channel views

reorg awareness

sequencer accreditation

checkpointing

Bedrock communication

deposits and withdrawals

Bedrock provides the shared foundation

consensus

ordered blockchain history

data availability for published Zone data

channel state

finality context

bridging primitives

inter-Zone coordination primitives

A safer description than calling Bedrock an Ethereum-style settlement layer is:

Bedrock provides the shared consensus, data-availability, ordering and interoperability foundation for Solizone's published Zone history.

3. Confirmed Findings from Logos Source Code

3.1 Zone SDK is the primary Bedrock-facing integration layer

The current Logos Zone SDK exposes a ZoneSequencer that can publish inscriptions, track pending/finalized channel state, surface channel events, backfill finalized history, persist checkpoints, resume from checkpoints, and communicate with a Logos node.

Solizone implication: do not recreate Mantle/channel handling unless research proves a missing capability.

3.2 Zone inscription payloads are opaque

The SDK represents a Zone block as an identifier plus opaque inscription data.

This supports a Solizone model where Bedrock does not need to understand EVM execution or EVM state.

3.3 Channels are both message-log and bridge boundaries

Current Logos bridging material treats a channel as both the Zone's published message log and the boundary for blockchain-to-Zone value movement.

Deposits increase the channel balance. Withdrawals decrease it. The Zone decides how aggregate channel value maps to its own internal user accounts.

3.4 Single-sequencer Zones are supported

A newly created channel can begin with one accredited key and one-signature thresholds.

Therefore a single-sequencer Solizone MVP is aligned with the current model.

3.5 LEZ demonstrates the key integration boundary

LEZ already uses the Logos Zone SDK behind its own block-publisher layer and handles concepts such as:

Zone blocks
Zone SDK publication
adopted blocks
orphaned blocks
finalized blocks
Bedrock deposits
Bedrock withdrawals
sequencer checkpoints
channel reconstruction

For Solizone, LEZ is therefore a direct reference for the Bedrock-facing boundary.

4. Important Architecture Refinement

Solizone should not treat "the sequencer" as one indivisible component.

4.1 Solizone Block Producer

Responsible for:

accept EVM transactions
order EVM transactions
execute transactions
generate receipts
update EVM state
produce Solizone blocks

4.2 Logos Channel Publisher

Responsible for:

serialize Zone block payload
publish through Zone SDK
follow channel ordering
track Bedrock adoption
track finalization
handle orphaned inscriptions
persist Zone SDK checkpoint
handle Bedrock deposits/withdrawals

For the MVP both may live in one process, but they should remain separate architectural responsibilities.

5. Research Method

Every research topic should use:

Research Question:
Why It Matters:
What We Know:
What Needs Verification:
Relevant Logos Repo / File:
Conclusion:
Impact on Solizone:

Every finding should be classified as:

Confirmed Fact — directly supported by current Logos code/docs.

Assumption — a working mental model that is not yet proven.

Solizone Design Decision — a choice owned by Solizone rather than Logos.

Open Research Question — unresolved and must be investigated before locking the affected design.

6. R&D Topic 001 — Zone SDK Publishing Lifecycle

Research Question:
What exact lifecycle is required for Solizone to publish Zone blocks through the Logos Zone SDK?

Why It Matters:
This defines the real integration boundary between Solizone and Logos.

What We Know:
The Zone SDK exposes ZoneSequencer, publishing, channel-state tracking, finalized history and sequencer checkpoints. LEZ already wraps this SDK behind a block-publisher abstraction.

What Needs Verification:

initialization flow

funding configuration

signing-key requirements

ChannelId creation

genesis / first inscription

publish lifecycle

MsgId creation

pending vs adopted vs finalized state

checkpoint persistence

restart recovery

retry and resubmission behavior

Bedrock reorg behavior

local/testnet setup

Relevant Logos Repo / File:

logos-blockchain/logos-blockchain
  zone-sdk/src/lib.rs
  zone-sdk/src/sequencer/*
  zone-sdk/src/adapter.rs

logos-blockchain/logos-execution-zone
  lez/sequencer/core/src/block_publisher.rs

Conclusion:
The Zone SDK should be treated as Solizone's default Bedrock adapter.

Impact on Solizone:
Do not build a custom Mantle/channel client before proving the SDK is insufficient.

7. R&D Topic 002 — Inscription Capacity, Cost and Throughput

Research Question:
How much data can Solizone practically publish in one inscription, how often, and at what cost?

Why It Matters:
This determines whether Solizone can publish complete transaction batches or only compact commitments.

What We Know:
Inscription data is opaque, so Solizone can define its own payload.

What Needs Verification:

maximum payload size

transaction size limits

storage pricing

execution fees

practical block frequency

channel throughput

Bedrock block cadence

number of Zone inscriptions per Bedrock block

compression feasibility

chunking requirements

Relevant Logos Repo / File:

logos-blockchain/logos-blockchain
  zone-sdk/src/sequencer/*
  Mantle inscription types
  Mantle transaction validation
  gas / storage pricing

Conclusion:
Unknown and P0.

Impact on Solizone:
Do not finalize the canonical Solizone block format until this is measured.

8. R&D Topic 003 — Canonical Solizone Zone Block

Research Question:
What exact data should Solizone publish to Bedrock for each Zone block?

Why It Matters:
Publishing only a state root may make the Zone impossible to reconstruct independently if transaction data disappears.

What We Know:
Bedrock does not dictate the internal format of a Zone inscription.

What Needs Verification:

Model A — Commitment only

state_root

Model B — Commitment plus batch hash

state_root
transaction_batch_hash

Model C — Full transaction batch plus commitments

height
parent
transactions[]
state_root
receipts_root
timestamp

Model D — Chunked or compressed DA

block header
state commitment
compressed/chunked transaction data

Relevant Logos Repo / File:

logos-blockchain/logos-blockchain
  zone-sdk/src/lib.rs

logos-blockchain/logos-execution-zone
  LEZ block serialization
  lez/sequencer/core/src/block_publisher.rs

Conclusion:
Topic 002 must be answered first.

Impact on Solizone:
The existing README's "state commitment" remains conceptual, not protocol-final.

9. R&D Topic 004 — EVM Runtime Architecture

Research Question:
Should Solizone use REVM directly, reuse Reth execution components, or another mature Rust EVM stack?

Why It Matters:
EVM bytecode execution alone does not provide an Ethereum-compatible chain.

Solizone also needs:

transaction decoding
signature recovery
chain specification
block environment
gas accounting
state backend
contract creation
precompiles
logs
receipts
hardfork rules

What We Know:
Rust fits the Logos ecosystem and REVM is a strong candidate, but it is not yet a confirmed architectural choice.

What Needs Verification:

REVM directly
vs
Reth execution/primitives
vs
other reusable Rust EVM stacks

Evaluate coupling, state integration, transaction infrastructure, block-production flexibility and long-term maintainability.

Relevant Logos Repo / File:
Primarily external EVM research.

Conclusion:
REVM is a candidate only.

Impact on Solizone:
Do not lock solizone-evm around REVM yet.

10. R&D Topic 005 — EVM Compatibility Target

Research Question:
What does "EVM-compatible" mean for the first Solizone MVP?

Why It Matters:
There is a major difference between executing bytecode and supporting normal Ethereum developer tooling.

What We Know:
Solizone's product goal is a familiar Solidity developer experience.

What Needs Verification:
Define an explicit MVP matrix, for example:

EVM bytecode               YES
Solidity deployment        YES
EOA signed transactions    YES
CREATE                      YES
CREATE2                     YES
logs/events                 YES
eth_call                    YES
eth_sendRawTransaction      YES
eth_getBalance              YES
eth_getCode                 YES
eth_getTransactionReceipt   YES
Foundry deployment          YES

archive RPC                 NO
debug_trace*                NO
eth_getProof                NO
blob transactions           NO
Ethereum consensus          NO

Also choose the target EVM hardfork/specification.

Relevant Logos Repo / File:
Solizone-specific.

Conclusion:
A compatibility matrix must exist before RPC implementation.

Impact on Solizone:
Prevents accidental commitment to full Ethereum equivalence.

11. R&D Topic 006 — State Model and State Commitment

Research Question:
Should Solizone use Ethereum's state trie/state root or define a Solizone-specific commitment?

Why It Matters:
Once published, the commitment becomes protocol history.

What We Know:
Bedrock does not require Ethereum's state representation.

What Needs Verification:

Ethereum MPT requirements

alternative authenticated structures

REVM/Reth state integration

RocksDB compatibility

snapshots

historical queries

replay

effect on eth_getProof

future proof systems

Important distinction:

simple RocksDB storage = acceptable MVP implementation choice
temporary arbitrary canonical commitment = risky protocol choice

Relevant Logos Repo / File:
Solizone-specific, with LEZ storage as a general persistence reference.

Conclusion:
Storage and state commitment are separate design decisions.

Impact on Solizone:
Research commitment semantics before publishing canonical blocks.

12. R&D Topic 007 — Bedrock Reorgs and Solizone Reorgs

Research Question:
How should Solizone react if an adopted Zone inscription is later orphaned?

Why It Matters:
Ethereum clients need coherent blockchain semantics.

What We Know:
The Zone SDK exposes adopted, orphaned and finalized updates. LEZ already handles block rollback around this model.

What Needs Verification:

Solizone likely needs:

local head
Bedrock-adopted head
Bedrock-finalized head

And clear meanings for:

latest
safe
finalized

Also determine:

when eth_blockNumber advances

when a block becomes safe/finalized

how orphaned transactions return to the mempool

how state rollback works

required retained state history

Relevant Logos Repo / File:

logos-blockchain/logos-blockchain
  zone-sdk/src/sequencer/state.rs
  zone-sdk/src/sequencer/block_fetch.rs

logos-blockchain/logos-execution-zone
  lez/sequencer/core/src/block_publisher.rs

Conclusion:
Reorg semantics are an MVP architecture concern.

Impact on Solizone:
Resolve before final RPC semantics.

13. R&D Topic 008 — Bridge Mapping into the EVM

Research Question:
How should a finalized Bedrock channel deposit appear inside Solizone?

Why It Matters:
Logos provides deposit events, but Solizone owns internal EVM accounting.

What We Know:

Bedrock notes
    ↓
ChannelDeposit
    ↓
channel balance increases
    ↓
finalized deposit event
    ↓
Solizone interprets metadata
    ↓
internal EVM balance credited

What Needs Verification:

Option A: EVM native currency
Option B: ERC-20 representation
Option C: system contract / precompile

Also:

metadata → EVM address

withdrawal format

replay prevention

accounting invariants

insolvency prevention

channel reconciliation

Relevant Logos Repo / File:

logos-blockchain/logos-blockchain
  zone-sdk/BRIDGING.md
  zone-sdk/src/sequencer/channel_wallet.rs

Conclusion:
Conceptually validated but not needed for Experiment 001.

Impact on Solizone:
Keep bridge implementation post-core MVP.

14. R&D Topic 009 — Cross-Zone Messaging

Research Question:
What is the current Logos-native mechanism for cross-Zone communication, and how should Solizone expose it to Solidity?

Why It Matters:
This could become a major Logos-native differentiator.

What We Know:
LEZ includes dedicated cross-Zone modules and inbox/outbox patterns.

What Needs Verification:

current message model

destination identification

authentication

ordering

atomicity

failures/retries

Solidity exposure via precompile, system contract, inbox contract or transaction type

Relevant Logos Repo / File:

logos-blockchain/logos-execution-zone
  lez/cross_zone/*
  lez/programs/cross_zone_inbox/*
  lez/programs/cross_zone_outbox/*
  lez/indexer/core/src/cross_zone_verifier.rs

Conclusion:
Important, but not required for the first MVP validation.

Impact on Solizone:
Defer implementation.

15. R&D Topic 010 — Decentralized Sequencing and Verification

Research Question:
How can Solizone eventually reduce trust in a single block producer?

Why It Matters:
A production Sovereign Zone should eventually have stronger correctness and sequencing guarantees.

What We Know:
Logos channels already contain multi-sequencer and threshold concepts.

What Needs Verification:

multiple Solizone block producers
round-robin publishing
first-write-wins behavior
sequencer timeouts
independent EVM re-execution
fraud proofs
validity proofs
ZK-EVM integration

Relevant Logos Repo / File:

logos-blockchain/logos-blockchain
  zone-sdk/DECENTRALIZED_SEQUENCING.md
  zone-sdk/src/sequencer/*

Conclusion:
Not required for MVP.

Impact on Solizone:
Do not let decentralization/proof research block architecture validation.

16. Research Priority Order

Priority

Research Area

Status

P0

Zone SDK publishing lifecycle

Start first

P0

Inscription payload limits / cost / throughput

Required before block format

P0

Canonical Solizone Zone block format

Required before protocol design

P0

EVM runtime architecture

Required before EVM implementation

P0

EVM compatibility target

Required before RPC implementation

P1

State commitment format

Required before canonical blocks

P1

Bedrock reorg mapping

Required before RPC finality semantics

P1

Zone SDK checkpoint / recovery model

Required for robust sequencer

P1

LEZ integration pattern

Avoid duplicate infrastructure

P2

Bridge asset model

Post-core MVP

P2

Withdrawal model

Post-core MVP

P3

Cross-Zone messaging

Later

P3

Multi-sequencer design

Later

P4

Fraud proofs / ZK verification

Post-MVP

17. Tentative R&D Sequence

R&D-001
Minimal Zone SDK publication experiment
        ↓
R&D-002
Measure inscription constraints and economics
        ↓
R&D-003
Define canonical Solizone Zone block
        ↓
R&D-004
Evaluate REVM vs Reth execution components
        ↓
R&D-005
Define EVM compatibility matrix
        ↓
R&D-006
Choose Solizone state commitment model
        ↓
R&D-007
Define Bedrock reorg → EVM reorg semantics
        ↓
──────────────────────────────────────
CORE MVP IMPLEMENTATION CAN BEGIN
──────────────────────────────────────
        ↓
Local EVM execution
        ↓
Solizone state
        ↓
Solizone block producer
        ↓
Ethereum RPC
        ↓
Zone SDK / Bedrock publisher

The point is not to fully implement Logos integration first.

The point is to prove the integration boundary before building the EVM side around it.

18. Experiment 001 — Minimal Solizone Zone

Objective

Prove that an independent Rust application can use the current Logos Zone SDK to publish and recover ordered opaque Solizone blocks through a Logos channel.

No EVM code is required.

Minimal Payload

SolizoneBlock {
    height: 0,
    parent: 0x00,
    payload: "hello-solizone"
}

Experiment Flow

Minimal Rust application
        ↓
connect to Logos node
        ↓
create / choose ChannelId
        ↓
initialize ZoneSequencer
        ↓
publish genesis SolizoneBlock
        ↓
receive MsgId
        ↓
observe adopted update
        ↓
observe finalized update
        ↓
persist SequencerCheckpoint
        ↓
restart process
        ↓
restore checkpoint
        ↓
recover finalized history
        ↓
publish SolizoneBlock #1
        ↓
verify ordered parent relationship

Success Criteria

Logos node reachable

ChannelId selected/created

ZoneSequencer initialized

opaque Solizone bytes published

MsgId returned

inscription visible in channel state

adopted state observed

finalized state observed

SequencerCheckpoint persisted

process restarted from checkpoint

finalized history reconstructed

second Solizone block references first

publication ordering demonstrated

practical payload constraints begin to be measured

Non-Goals

REVM
Solidity
Ethereum JSON-RPC
MetaMask
Foundry
contract deployment
EVM state
bridge implementation
cross-Zone messaging
multiple sequencers
fraud proofs
ZK proofs

Expected Result

If Experiment 001 succeeds, this boundary becomes practically validated:

Solizone execution layer
        ↓
serialize Solizone Zone block
        ↓
Logos Zone SDK
        ↓
Mantle Channel
        ↓
Bedrock

19. What We Should Not Research Yet

Keep these deliberately out of scope until the core boundary works:

ZK-EVM
fraud proofs
full decentralized sequencing
MEV mitigation
advanced bridge UX
cross-Zone Solidity standards
production explorer
production wallet integration
archive RPC
high-performance parallel EVM execution
production validator economics

20. Current R&D Verdict

Solizone as an EVM-compatible Logos Zone: PROMISING

No fundamental contradiction has been identified between the Solizone concept and the current Logos Zone architecture.

Strongest current evidence:

Logos provides a dedicated Zone SDK.

Zone SDK supports opaque Zone inscriptions.

Channels expose adopted/orphaned/finalized history.

Channels support Zone bridging primitives.

Single-sequencer operation is supported.

LEZ already uses the Zone SDK through a dedicated block-publisher boundary.

The biggest unresolved architectural question is:

What exact data should a Solizone Zone block publish so that Bedrock provides sufficient ordered data availability for Solizone state to be reconstructed and eventually verified independently?

Experiment 001 should validate the publication boundary first.

21. Baseline Rule Going Forward

Before implementing any Solizone component, ask:

1. Does Logos already provide this component?

2. Has the required Logos behavior been confirmed from source code?

3. Is this a confirmed requirement or only an assumption?

4. Does this decision affect the canonical Zone protocol?

5. Can a small experiment prove the assumption before production code is written?

If uncertain, research first.

This document should evolve from v0.01 as experiments convert assumptions into confirmed architecture decisions.