use revm::primitives::B256;

use crate::{
    block::SolizoneBlock,
    block_builder::build_block,
    execution::{RevmExecutionEngine, transaction::SolizoneTransaction},
    state::MemoryState,
};

pub struct BlockProducer {
    version: u16,
    chain_id: u64,
    gas_limit: u64,
    next_height: u64,
    parent_hash: B256,
}

impl BlockProducer {
    pub fn new(version: u16, chain_id: u64, gas_limit: u64) -> Self {
        Self {
            version,
            chain_id,
            gas_limit,
            next_height: 0,
            parent_hash: B256::ZERO,
        }
    }

    pub fn resume(
        version: u16,
        chain_id: u64,
        gas_limit: u64,
        next_height: u64,
        parent_hash: B256,
    ) -> Self {
        Self {
            version,
            chain_id,
            gas_limit,
            next_height,
            parent_hash,
        }
    }

    pub fn produce_block(
        &mut self,
        engine: &RevmExecutionEngine,
        state: &mut MemoryState,
        transactions: &[SolizoneTransaction],
        timestamp: u64,
    ) -> SolizoneBlock {
        let outcome = engine.execute_block(state, transactions);

        let block = build_block(
            self.version,
            self.chain_id,
            self.next_height,
            self.parent_hash,
            timestamp,
            outcome.state_root,
            transactions,
            &outcome.receipts,
            self.gas_limit,
        );

        self.parent_hash = block.hash();

        self.next_height += 1;

        block
    }

    pub fn next_height(&self) -> u64 {
        self.next_height
    }

    pub fn parent_hash(&self) -> B256 {
        self.parent_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use revm::{
        primitives::{Bytes, U256, address},
        state::AccountInfo,
    };

    use crate::execution::transaction::{SolizoneTransaction, TransactionKind};

    #[test]
    fn produces_linked_blocks_automatically() {
        let engine = RevmExecutionEngine::new();

        let mut state = MemoryState::new();

        let mut producer = BlockProducer::new(1, 9001, 30_000_000);

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

        /*
         * BLOCK #0
         */

        let transactions_0 = vec![SolizoneTransaction {
            sender,
            nonce: 0,
            kind: TransactionKind::Call(recipient),
            value: U256::from(100),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_0 = producer.produce_block(&engine, &mut state, &transactions_0, 1_800_000_000);

        assert_eq!(block_0.header.height, 0,);

        assert_eq!(block_0.header.parent_hash, B256::ZERO,);

        assert_eq!(producer.next_height(), 1,);

        assert_eq!(producer.parent_hash(), block_0.hash(),);

        /*
         * BLOCK #1
         */

        let transactions_1 = vec![SolizoneTransaction {
            sender,
            nonce: 1,
            kind: TransactionKind::Call(recipient),
            value: U256::from(200),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_1 = producer.produce_block(&engine, &mut state, &transactions_1, 1_800_000_001);

        assert_eq!(block_1.header.height, 1,);

        assert_eq!(block_1.header.parent_hash, block_0.hash(),);

        assert_eq!(producer.next_height(), 2,);

        assert_eq!(producer.parent_hash(), block_1.hash(),);

        assert_ne!(block_0.header.state_root, block_1.header.state_root,);

        assert_eq!(block_1.header.state_root, state.state_root(),);

        block_0.validate().expect("block #0 should be valid");

        block_1.validate().expect("block #1 should be valid");

        println!(
            "Block #0: height={} hash={}",
            block_0.header.height,
            block_0.hash()
        );

        println!(
            "Block #1: height={} parent={}",
            block_1.header.height, block_1.header.parent_hash
        );

        println!("Block #1 hash: {}", block_1.hash());

        println!("Next height:   {}", producer.next_height());
    }

    #[test]
    fn resumes_block_production_from_chain_head() {
        let engine = RevmExecutionEngine::new();

        let mut state = MemoryState::new();

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

        let mut producer = BlockProducer::new(1, 9001, 30_000_000);

        /*
         * BLOCK #0
         */

        let transactions_0 = vec![SolizoneTransaction {
            sender,
            nonce: 0,
            kind: TransactionKind::Call(recipient),
            value: U256::from(100),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_0 = producer.produce_block(&engine, &mut state, &transactions_0, 1_800_000_000);

        /*
         * BLOCK #1
         */

        let transactions_1 = vec![SolizoneTransaction {
            sender,
            nonce: 1,
            kind: TransactionKind::Call(recipient),
            value: U256::from(200),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_1 = producer.produce_block(&engine, &mut state, &transactions_1, 1_800_000_001);

        assert_eq!(block_1.header.parent_hash, block_0.hash(),);

        /*
         * Capture the chain head that would be persisted.
         */

        let next_height = producer.next_height();

        let parent_hash = producer.parent_hash();

        assert_eq!(next_height, 2);
        assert_eq!(parent_hash, block_1.hash());

        /*
         * Simulate BlockProducer process restart.
         */

        drop(producer);

        let mut restored_producer =
            BlockProducer::resume(1, 9001, 30_000_000, next_height, parent_hash);

        /*
         * BLOCK #2 — produced after restart.
         */

        let transactions_2 = vec![SolizoneTransaction {
            sender,
            nonce: 2,
            kind: TransactionKind::Call(recipient),
            value: U256::from(300),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_2 =
            restored_producer.produce_block(&engine, &mut state, &transactions_2, 1_800_000_002);

        /*
         * CHAIN CONTINUITY
         */

        assert_eq!(block_2.header.height, 2,);

        assert_eq!(block_2.header.parent_hash, block_1.hash(),);

        assert_eq!(restored_producer.next_height(), 3,);

        assert_eq!(restored_producer.parent_hash(), block_2.hash(),);

        assert_eq!(block_2.header.state_root, state.state_root(),);

        /*
         * State also continued across all three blocks.
         */

        let sender_account = state
            .db()
            .cache
            .accounts
            .get(&sender)
            .and_then(|account| account.info())
            .expect("sender missing");

        let recipient_account = state
            .db()
            .cache
            .accounts
            .get(&recipient)
            .and_then(|account| account.info())
            .expect("recipient missing");

        assert_eq!(sender_account.nonce, 3,);

        assert_eq!(recipient_account.balance, U256::from(600),);

        println!(
            "Block #0: height={} hash={}",
            block_0.header.height,
            block_0.hash()
        );

        println!(
            "Block #1: height={} hash={}",
            block_1.header.height,
            block_1.hash()
        );

        println!("--- producer restart ---");

        println!("Recovered next height: {}", next_height);

        println!("Recovered parent hash: {}", parent_hash);

        println!(
            "Block #2: height={} parent={}",
            block_2.header.height, block_2.header.parent_hash
        );

        println!("Block #2 hash: {}", block_2.hash());

        println!("Final sender nonce: {}", sender_account.nonce);

        println!("Recipient balance:  {}", recipient_account.balance);
    }

    #[test]
    fn resumes_state_and_block_production_after_restart() {
        let engine = RevmExecutionEngine::new();

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        let mut state = MemoryState::new();

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

        let mut producer = BlockProducer::new(1, 9001, 30_000_000);

        /*
         * BLOCK #0
         */

        let transactions_0 = vec![SolizoneTransaction {
            sender,
            nonce: 0,
            kind: TransactionKind::Call(recipient),
            value: U256::from(100),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_0 = producer.produce_block(&engine, &mut state, &transactions_0, 1_800_000_000);

        /*
         * BLOCK #1
         */

        let transactions_1 = vec![SolizoneTransaction {
            sender,
            nonce: 1,
            kind: TransactionKind::Call(recipient),
            value: U256::from(200),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_1 = producer.produce_block(&engine, &mut state, &transactions_1, 1_800_000_001);

        /*
         * Capture everything required for restart.
         */

        let state_root_before_restart = state.state_root();

        let snapshot = state.snapshot();

        let next_height = producer.next_height();

        let parent_hash = producer.parent_hash();

        assert_eq!(next_height, 2);
        assert_eq!(parent_hash, block_1.hash());

        /*
         * Simulate complete Solizone process shutdown.
         */

        drop(state);
        drop(producer);

        /*
         * Restore EVM state.
         */

        let mut restored_state = MemoryState::from_snapshot(snapshot);

        assert_eq!(
            restored_state.state_root(),
            state_root_before_restart,
            "state root changed after restart"
        );

        /*
         * Restore chain head.
         */

        let mut restored_producer =
            BlockProducer::resume(1, 9001, 30_000_000, next_height, parent_hash);

        /*
         * BLOCK #2 — first block after restart.
         */

        let transactions_2 = vec![SolizoneTransaction {
            sender,
            nonce: 2,
            kind: TransactionKind::Call(recipient),
            value: U256::from(300),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_2 = restored_producer.produce_block(
            &engine,
            &mut restored_state,
            &transactions_2,
            1_800_000_002,
        );

        /*
         * Chain continuity.
         */

        assert_eq!(block_2.header.height, 2,);

        assert_eq!(block_2.header.parent_hash, block_1.hash(),);

        assert_eq!(restored_producer.next_height(), 3,);

        /*
         * State continuity.
         */

        assert_eq!(block_2.header.state_root, restored_state.state_root(),);

        let sender_account = restored_state
            .db()
            .cache
            .accounts
            .get(&sender)
            .and_then(|account| account.info())
            .expect("sender missing");

        let recipient_account = restored_state
            .db()
            .cache
            .accounts
            .get(&recipient)
            .and_then(|account| account.info())
            .expect("recipient missing");

        assert_eq!(sender_account.nonce, 3,);

        assert_eq!(recipient_account.balance, U256::from(600),);

        println!("Block #0 hash: {}", block_0.hash());

        println!("Block #1 hash: {}", block_1.hash());

        println!("--- full Solizone restart ---");

        println!("Recovered state root:  {}", state_root_before_restart);

        println!("Recovered next height: {}", next_height);

        println!("Recovered parent hash: {}", parent_hash);

        println!("Block #2 height: {}", block_2.header.height);

        println!("Block #2 parent: {}", block_2.header.parent_hash);

        println!("Block #2 state root: {}", block_2.header.state_root);

        println!("Final sender nonce: {}", sender_account.nonce);

        println!("Recipient balance:  {}", recipient_account.balance);
    }

    #[test]
    fn resumes_full_node_from_disk_checkpoint() {
        use crate::state::{FileCheckpointBackend, SolizoneCheckpoint};

        let checkpoint_path = std::env::temp_dir().join("solizone-full-restart-test.json");

        if checkpoint_path.exists() {
            std::fs::remove_file(&checkpoint_path).expect("failed to remove old checkpoint");
        }

        let backend = FileCheckpointBackend::new(checkpoint_path);

        let engine = RevmExecutionEngine::new();

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        /*
         * Start fresh Solizone node.
         */

        let mut state = MemoryState::new();

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

        let mut producer = BlockProducer::new(1, 9001, 30_000_000);

        /*
         * BLOCK #0
         */

        let transactions_0 = vec![SolizoneTransaction {
            sender,
            nonce: 0,
            kind: TransactionKind::Call(recipient),
            value: U256::from(100),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_0 = producer.produce_block(&engine, &mut state, &transactions_0, 1_800_000_000);

        /*
         * BLOCK #1
         */

        let transactions_1 = vec![SolizoneTransaction {
            sender,
            nonce: 1,
            kind: TransactionKind::Call(recipient),
            value: U256::from(200),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_1 = producer.produce_block(&engine, &mut state, &transactions_1, 1_800_000_001);

        /*
         * Build ONE consistent checkpoint.
         */

        let checkpoint = SolizoneCheckpoint::new(
            state.snapshot(),
            producer.next_height(),
            producer.parent_hash(),
        );

        let state_root_before_restart = state.state_root();

        backend
            .save(&checkpoint)
            .expect("failed to persist checkpoint");

        assert!(backend.path().exists(), "checkpoint file should exist");

        /*
         * Simulate complete process shutdown.
         *
         * No in-memory state or producer survives.
         */

        drop(checkpoint);
        drop(state);
        drop(producer);

        /*
         * Simulate fresh process loading from disk.
         */

        let recovered_checkpoint = backend
            .load()
            .expect("failed to load checkpoint")
            .expect("checkpoint missing");

        let mut restored_state = MemoryState::from_snapshot(recovered_checkpoint.state);

        assert_eq!(
            restored_state.state_root(),
            state_root_before_restart,
            "state root changed after disk recovery"
        );

        let mut restored_producer = BlockProducer::resume(
            1,
            9001,
            30_000_000,
            recovered_checkpoint.next_height,
            recovered_checkpoint.parent_hash,
        );

        /*
         * BLOCK #2 — first block after disk recovery.
         */

        let transactions_2 = vec![SolizoneTransaction {
            sender,
            nonce: 2,
            kind: TransactionKind::Call(recipient),
            value: U256::from(300),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block_2 = restored_producer.produce_block(
            &engine,
            &mut restored_state,
            &transactions_2,
            1_800_000_002,
        );

        /*
         * Verify chain continuity.
         */

        assert_eq!(block_2.header.height, 2,);

        assert_eq!(block_2.header.parent_hash, block_1.hash(),);

        assert_eq!(restored_producer.next_height(), 3,);

        assert_eq!(restored_producer.parent_hash(), block_2.hash(),);

        /*
         * Verify state continuity.
         */

        assert_eq!(block_2.header.state_root, restored_state.state_root(),);

        let sender_account = restored_state
            .db()
            .cache
            .accounts
            .get(&sender)
            .and_then(|account| account.info())
            .expect("sender missing after recovery");

        let recipient_account = restored_state
            .db()
            .cache
            .accounts
            .get(&recipient)
            .and_then(|account| account.info())
            .expect("recipient missing after recovery");

        assert_eq!(sender_account.nonce, 3,);

        assert_eq!(recipient_account.balance, U256::from(600),);

        println!("Block #0 hash: {}", block_0.hash());

        println!("Block #1 hash: {}", block_1.hash());

        println!("Checkpoint state root: {}", state_root_before_restart);

        println!("--- complete process restart from disk ---");

        println!("Block #2 height: {}", block_2.header.height);

        println!("Block #2 parent: {}", block_2.header.parent_hash);

        println!("Block #2 hash: {}", block_2.hash());

        println!("Block #2 state root: {}", block_2.header.state_root);

        println!("Final sender nonce: {}", sender_account.nonce);

        println!("Recipient balance: {}", recipient_account.balance);

        std::fs::remove_file(backend.path()).expect("failed to remove checkpoint");
    }
}
