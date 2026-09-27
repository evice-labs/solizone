use std::{env, fs, path::Path};

use revm::{
    primitives::{Bytes, U256, address},
    state::AccountInfo,
};

use solizone_evm::{
    block_producer::BlockProducer,
    block_store::BlockStore,
    execution::{
        RevmExecutionEngine,
        transaction::{SolizoneTransaction, TransactionKind},
    },
    hybrid_block_store::HybridBlockStore,
    state::{CheckpointBackend, LogosStorageCheckpointBackend, MemoryState, SolizoneCheckpoint},
};

const VERSION: u16 = 1;
const CHAIN_ID: u64 = 9001;
const GAS_LIMIT: u64 = 30_000_000;
const RETENTION_WINDOW: usize = 3;

fn main() -> Result<(), String> {
    let mode = env::args()
        .nth(1)
        .ok_or_else(|| "usage: hybrid_node_restart <write|recover>".to_string())?;

    match mode.as_str() {
        "write" => process_a(),
        "recover" => process_b(),
        _ => Err("mode must be 'write' or 'recover'".to_string()),
    }
}

fn process_a() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let block_directory = root.join("hybrid-node-blocks");

    let archive_index_path = root.join("hybrid-node-archive-index.json");

    let recovery_directory = root.join("hybrid-node-recovery");

    let archive_work_directory = root.join("hybrid-node-archive-work");

    let checkpoint_work_directory = root.join("hybrid-node-checkpoint-work");

    let checkpoint_cid_path = root.join("hybrid-node-latest-checkpoint.cid");

    /*
     * Fresh-node cleanup.
     */

    clean_directory(&block_directory)?;

    clean_directory(&recovery_directory)?;

    clean_directory(&archive_work_directory)?;

    clean_directory(&checkpoint_work_directory)?;

    remove_file_if_exists(&archive_index_path)?;

    remove_file_if_exists(&checkpoint_cid_path)?;

    let engine = RevmExecutionEngine::new();

    let sender = address!("1111111111111111111111111111111111111111");

    let recipient = address!("2222222222222222222222222222222222222222");

    let mut state = MemoryState::new();

    state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

    let mut producer = BlockProducer::new(VERSION, CHAIN_ID, GAS_LIMIT);

    let mut block_store = HybridBlockStore::new(
        &block_directory,
        &archive_index_path,
        &recovery_directory,
        &archive_work_directory,
        RETENTION_WINDOW,
    );

    /*
     * Produce blocks 0..4.
     *
     * Each block executes a genuine
     * EVM value transfer.
     */

    for height in 0u64..5 {
        let transactions = vec![SolizoneTransaction {
            sender,
            nonce: height,
            kind: TransactionKind::Call(recipient),
            value: U256::from(100),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let block =
            producer.produce_block(&engine, &mut state, &transactions, 1_800_000_000 + height);

        block
            .validate()
            .map_err(|error| format!("block #{height} validation failed: {error:?}"))?;

        println!("Produced block #{}: {}", block.header.height, block.hash(),);

        block_store.insert(block)?;

        println!(
            "Local={}, archived={}",
            block_store.local_len()?,
            block_store.archived_len()?,
        );
    }

    /*
     * With retention = 3:
     *
     * archived → 0,1
     * local    → 2,3,4
     */

    assert_eq!(block_store.local_len()?, 3,);

    assert_eq!(block_store.archived_len()?, 2,);

    assert_eq!(block_store.len()?, 5,);

    /*
     * Persist execution state +
     * producer chain head remotely.
     */

    let checkpoint = SolizoneCheckpoint::new(
        state.snapshot(),
        producer.next_height(),
        producer.parent_hash(),
    );

    let checkpoint_backend =
        LogosStorageCheckpointBackend::new(&checkpoint_work_directory, &checkpoint_cid_path);

    checkpoint_backend.save(&checkpoint)?;

    println!("Checkpoint next height: {}", producer.next_height(),);

    println!("Checkpoint parent hash: {}", producer.parent_hash(),);

    /*
     * Validate state before shutdown.
     */

    let sender_account = state
        .db()
        .cache
        .accounts
        .get(&sender)
        .and_then(|account| account.info())
        .ok_or_else(|| "sender missing before restart".to_string())?;

    let recipient_account = state
        .db()
        .cache
        .accounts
        .get(&recipient)
        .and_then(|account| account.info())
        .ok_or_else(|| "recipient missing before restart".to_string())?;

    assert_eq!(sender_account.nonce, 5,);

    assert_eq!(recipient_account.balance, U256::from(500),);

    println!("Sender nonce before restart: {}", sender_account.nonce,);

    println!(
        "Recipient balance before restart: {}",
        recipient_account.balance,
    );

    println!("PROCESS A complete");

    Ok(())
}

fn process_b() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let block_directory = root.join("hybrid-node-blocks");

    let archive_index_path = root.join("hybrid-node-archive-index.json");

    let recovery_directory = root.join("hybrid-node-recovery");

    let archive_work_directory = root.join("hybrid-node-archive-work");

    let checkpoint_work_directory = root.join("hybrid-node-checkpoint-work");

    let checkpoint_cid_path = root.join("hybrid-node-latest-checkpoint.cid");

    let sender = address!("1111111111111111111111111111111111111111");

    let recipient = address!("2222222222222222222222222222222222222222");

    /*
     * Fresh process:
     *
     * reconstruct storage objects only
     * from persistent paths.
     */

    let checkpoint_backend =
        LogosStorageCheckpointBackend::new(&checkpoint_work_directory, &checkpoint_cid_path);

    let checkpoint = checkpoint_backend
        .load()?
        .ok_or_else(|| "checkpoint missing".to_string())?;

    println!("Checkpoint recovered from Logos Storage");

    println!("Recovered next height: {}", checkpoint.next_height,);

    println!("Recovered parent hash: {}", checkpoint.parent_hash,);

    /*
     * Capture chain-head metadata before
     * moving the snapshot into MemoryState.
     */

    let next_height = checkpoint.next_height;

    let parent_hash = checkpoint.parent_hash;

    let mut state = MemoryState::from_snapshot(checkpoint.state);

    let mut producer =
        BlockProducer::resume(VERSION, CHAIN_ID, GAS_LIMIT, next_height, parent_hash);

    let mut block_store = HybridBlockStore::new(
        &block_directory,
        &archive_index_path,
        &recovery_directory,
        &archive_work_directory,
        RETENTION_WINDOW,
    );

    /*
     * Verify the persisted history BEFORE
     * producing anything new.
     */

    assert_eq!(block_store.local_len()?, 3,);

    assert_eq!(block_store.archived_len()?, 2,);

    assert_eq!(block_store.len()?, 5,);

    /*
     * Block #4 should still be local.
     */

    let block_4 = block_store
        .get_by_height(4)?
        .ok_or_else(|| "block #4 missing".to_string())?;

    println!("Recent block #4 recovered: {}", block_4.hash(),);

    assert_eq!(block_4.hash(), parent_hash,);

    /*
     * Block #0 was pruned in Process A.
     *
     * This should transparently fetch it
     * from Logos Storage.
     */

    let block_0 = block_store
        .get_by_height(0)?
        .ok_or_else(|| "archived block #0 missing".to_string())?;

    assert_eq!(block_0.header.height, 0,);

    println!("Archived block #0 recovered: {}", block_0.hash(),);

    /*
     * Produce the first post-restart block.
     */

    let engine = RevmExecutionEngine::new();

    let transactions = vec![SolizoneTransaction {
        sender,
        nonce: 5,
        kind: TransactionKind::Call(recipient),
        value: U256::from(100),
        data: Bytes::new(),
        gas_limit: 21_000,
    }];

    let block_5 = producer.produce_block(&engine, &mut state, &transactions, 1_800_000_005);

    block_5
        .validate()
        .map_err(|error| format!("block #5 validation failed: {error:?}"))?;

    /*
     * Critical chain continuity proof.
     */

    assert_eq!(block_5.header.height, 5,);

    assert_eq!(block_5.header.parent_hash, block_4.hash(),);

    println!("Post-restart block #5: {}", block_5.hash(),);

    println!("Block #5 parent: {}", block_5.header.parent_hash,);

    /*
     * Inserting #5 creates:
     *
     * local = 2,3,4,5
     *
     * retention=3 therefore #2
     * must automatically archive.
     */

    block_store.insert(block_5.clone())?;

    assert_eq!(block_store.local_len()?, 3,);

    assert_eq!(block_store.archived_len()?, 3,);

    assert_eq!(block_store.len()?, 6,);

    /*
     * #2 should now be remote too.
     */

    let block_2 = block_store
        .get_by_height(2)?
        .ok_or_else(|| "block #2 missing after retention".to_string())?;

    assert_eq!(block_2.header.height, 2,);

    println!("Newly archived block #2 recovered: {}", block_2.hash(),);

    /*
     * Save the new chain head checkpoint.
     */

    let new_checkpoint = SolizoneCheckpoint::new(
        state.snapshot(),
        producer.next_height(),
        producer.parent_hash(),
    );

    checkpoint_backend.save(&new_checkpoint)?;

    /*
     * Verify EVM state continuity across
     * the actual process boundary.
     */

    let sender_account = state
        .db()
        .cache
        .accounts
        .get(&sender)
        .and_then(|account| account.info())
        .ok_or_else(|| "sender missing after restart".to_string())?;

    let recipient_account = state
        .db()
        .cache
        .accounts
        .get(&recipient)
        .and_then(|account| account.info())
        .ok_or_else(|| "recipient missing after restart".to_string())?;

    assert_eq!(sender_account.nonce, 6,);

    assert_eq!(recipient_account.balance, U256::from(600),);

    assert_eq!(producer.next_height(), 6,);

    assert_eq!(producer.parent_hash(), block_5.hash(),);

    println!("Final local blocks: {}", block_store.local_len()?,);

    println!("Final archived blocks: {}", block_store.archived_len()?,);

    println!("Total canonical blocks: {}", block_store.len()?,);

    println!("Final sender nonce: {}", sender_account.nonce,);

    println!("Final recipient balance: {}", recipient_account.balance,);

    println!("PROCESS B recovery verified");

    println!("Hybrid node restart verified");

    Ok(())
}

fn clean_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to clean {}: {error}", path.display(),))?;
    }

    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display(),))
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display(),))?;
    }

    Ok(())
}
