use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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

const BLOCK_INTERVAL_SECONDS: u64 = 5;

struct NodePaths {
    blocks: PathBuf,
    archive_index: PathBuf,
    recovery: PathBuf,
    archive_work: PathBuf,
    checkpoint_work: PathBuf,
    checkpoint_cid: PathBuf,
}

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let paths = NodePaths {
        blocks: root.join("solizone-node-blocks"),

        archive_index: root.join("solizone-node-archive-index.json"),

        recovery: root.join("solizone-node-recovery"),

        archive_work: root.join("solizone-node-archive-work"),

        checkpoint_work: root.join("solizone-node-checkpoint-work"),

        checkpoint_cid: root.join("solizone-node-latest-checkpoint.cid"),
    };

    /*
     * Optional demo reset:
     *
     * cargo run --bin solizone_node -- reset
     */

    if env::args().nth(1).as_deref() == Some("reset") {
        reset_node(&paths)?;

        println!("Solizone node demo state reset");

        return Ok(());
    }

    run_node(paths)
}

fn run_node(paths: NodePaths) -> Result<(), String> {
    fs::create_dir_all(&paths.blocks).map_err(|error| error.to_string())?;

    fs::create_dir_all(&paths.recovery).map_err(|error| error.to_string())?;

    fs::create_dir_all(&paths.archive_work).map_err(|error| error.to_string())?;

    fs::create_dir_all(&paths.checkpoint_work).map_err(|error| error.to_string())?;

    let checkpoint_backend =
        LogosStorageCheckpointBackend::new(&paths.checkpoint_work, &paths.checkpoint_cid);

    let sender = address!("1111111111111111111111111111111111111111");

    let recipient = address!("2222222222222222222222222222222222222222");

    /*
     * STARTUP:
     *
     * Automatically decide whether this is
     * a new node or a restart.
     */

    let (mut state, mut producer) = if paths.checkpoint_cid.exists() {
        println!("\n========================================");

        println!("Existing Solizone checkpoint detected");

        println!("Recovering node from Logos Storage...");

        println!("========================================\n");

        let checkpoint = checkpoint_backend.load()?.ok_or_else(|| {
            "checkpoint CID exists but checkpoint could not be loaded".to_string()
        })?;

        let next_height = checkpoint.next_height;

        let parent_hash = checkpoint.parent_hash;

        let state = MemoryState::from_snapshot(checkpoint.state);

        let producer =
            BlockProducer::resume(VERSION, CHAIN_ID, GAS_LIMIT, next_height, parent_hash);

        println!("Node recovery complete");

        println!("Recovered next height: {}", next_height,);

        println!("Recovered parent hash: {}", parent_hash,);

        (state, producer)
    } else {
        println!("\n========================================");

        println!("No checkpoint found");

        println!("Starting fresh Solizone node");

        println!("========================================\n");

        let mut state = MemoryState::new();

        /*
         * Large balance so the demo can
         * continue producing transactions.
         */

        state.insert_account_info(
            sender,
            AccountInfo::from_balance(U256::from(1_000_000_000u64)),
        );

        let producer = BlockProducer::new(VERSION, CHAIN_ID, GAS_LIMIT);

        (state, producer)
    };

    let mut block_store = HybridBlockStore::new(
        &paths.blocks,
        &paths.archive_index,
        &paths.recovery,
        &paths.archive_work,
        RETENTION_WINDOW,
    );

    /*
     * If this is a restart, verify that
     * our recovered chain head agrees
     * with stored block history.
     */

    if producer.next_height() > 0 {
        let previous_height = producer.next_height() - 1;

        let previous_block = block_store
            .get_by_height(previous_height)?
            .ok_or_else(|| format!("chain-head block #{previous_height} missing"))?;

        if previous_block.hash() != producer.parent_hash() {
            return Err("checkpoint parent hash does not match stored chain head".to_string());
        }

        println!("Chain-head verification passed");

        println!("Head block: #{}", previous_height,);

        println!("Head hash:  {}", previous_block.hash(),);

        println!();
    }

    let engine = RevmExecutionEngine::new();

    println!("Solizone node is running");

    println!("Block interval: {} seconds", BLOCK_INTERVAL_SECONDS,);

    println!("Local retention window: {} blocks", RETENTION_WINDOW,);

    println!("Press Ctrl+C after a completed checkpoint to stop the demo.");

    println!();

    /*
     * LONG-RUNNING NODE LOOP
     */

    loop {
        let height = producer.next_height();

        /*
         * One deterministic transfer per block.
         *
         * Because every produced block contains
         * exactly one sender transaction:
         *
         * transaction nonce == block height.
         */

        let transactions = vec![SolizoneTransaction {
            sender,

            nonce: height,

            kind: TransactionKind::Call(recipient),

            value: U256::from(100),

            data: Bytes::new(),

            gas_limit: 21_000,
        }];

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_secs();

        let block = producer.produce_block(&engine, &mut state, &transactions, timestamp);

        block
            .validate()
            .map_err(|error| format!("block #{height} failed validation: {error:?}"))?;

        let block_hash = block.hash();

        let parent_hash = block.header.parent_hash;

        /*
         * HybridBlockStore automatically:
         *
         * 1. persists block locally
         * 2. checks retention
         * 3. archives oldest block if needed
         * 4. records CID
         * 5. prunes old local .szb
         */

        block_store.insert(block)?;

        /*
         * Persist current EVM state +
         * new chain head to Logos Storage.
         */

        let checkpoint = SolizoneCheckpoint::new(
            state.snapshot(),
            producer.next_height(),
            producer.parent_hash(),
        );

        checkpoint_backend.save(&checkpoint)?;

        /*
         * Demo-friendly status output.
         */

        println!("\n----------------------------------------");

        println!("Produced block #{}", height,);

        println!("Block hash:  {}", block_hash,);

        println!("Parent hash: {}", parent_hash,);

        println!("State root:  {}", state.state_root(),);

        println!("Local blocks:    {}", block_store.local_len()?,);

        println!("Archived blocks: {}", block_store.archived_len()?,);

        println!("Total history:   {}", block_store.len()?,);

        println!("Next height:     {}", producer.next_height(),);

        println!("Checkpoint:      Logos Storage validated");

        println!("----------------------------------------\n");

        /*
         * This makes continuous production
         * visually clear in the video.
         */

        thread::sleep(Duration::from_secs(BLOCK_INTERVAL_SECONDS));
    }
}

fn reset_node(paths: &NodePaths) -> Result<(), String> {
    for directory in [
        &paths.blocks,
        &paths.recovery,
        &paths.archive_work,
        &paths.checkpoint_work,
    ] {
        remove_directory_if_exists(directory)?;
    }

    remove_file_if_exists(&paths.archive_index)?;

    remove_file_if_exists(&paths.checkpoint_cid)?;

    Ok(())
}

fn remove_directory_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display(),))?;
    }

    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display(),))?;
    }

    Ok(())
}
