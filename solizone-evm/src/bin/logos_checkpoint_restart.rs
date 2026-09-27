use std::env;

use revm::{
    primitives::{Bytes, U256, address},
    state::AccountInfo,
};

use solizone_evm::{
    block_producer::BlockProducer,
    execution::{
        RevmExecutionEngine,
        transaction::{SolizoneTransaction, TransactionKind},
    },
    state::{CheckpointBackend, LogosStorageCheckpointBackend, MemoryState, SolizoneCheckpoint},
};

const VERSION: u16 = 1;
const CHAIN_ID: u64 = 9001;
const GAS_LIMIT: u64 = 30_000_000;

fn main() -> Result<(), String> {
    let mode = env::args()
        .nth(1)
        .ok_or_else(|| "usage: logos_checkpoint_restart <write|recover>".to_string())?;

    match mode.as_str() {
        "write" => write_checkpoint(),
        "recover" => recover_checkpoint(),

        _ => Err("usage: logos_checkpoint_restart <write|recover>".to_string()),
    }
}

fn backend() -> Result<LogosStorageCheckpointBackend, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    Ok(LogosStorageCheckpointBackend::new(
        root.join("storage-checkpoint-restart-work"),
        root.join("latest-checkpoint.cid"),
    ))
}

fn write_checkpoint() -> Result<(), String> {
    let engine = RevmExecutionEngine::new();

    let sender = address!("1111111111111111111111111111111111111111");

    let recipient = address!("2222222222222222222222222222222222222222");

    let mut state = MemoryState::new();

    state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

    let mut producer = BlockProducer::new(VERSION, CHAIN_ID, GAS_LIMIT);

    /*
     * BLOCK #0
     */

    let tx_0 = SolizoneTransaction {
        sender,
        nonce: 0,
        kind: TransactionKind::Call(recipient),
        value: U256::from(100),
        data: Bytes::new(),
        gas_limit: 21_000,
    };

    let block_0 = producer.produce_block(&engine, &mut state, &[tx_0], 1_800_000_000);

    /*
     * BLOCK #1
     */

    let tx_1 = SolizoneTransaction {
        sender,
        nonce: 1,
        kind: TransactionKind::Call(recipient),
        value: U256::from(200),
        data: Bytes::new(),
        gas_limit: 21_000,
    };

    let block_1 = producer.produce_block(&engine, &mut state, &[tx_1], 1_800_000_001);

    let checkpoint = SolizoneCheckpoint::new(
        state.snapshot(),
        producer.next_height(),
        producer.parent_hash(),
    );

    let backend = backend()?;

    backend.save(&checkpoint)?;

    println!("Block #0 hash: {}", block_0.hash(),);

    println!("Block #1 hash: {}", block_1.hash(),);

    println!("Checkpoint next height: {}", checkpoint.next_height,);

    println!("Checkpoint parent hash: {}", checkpoint.parent_hash,);

    println!("PROCESS A complete");

    Ok(())
}

fn recover_checkpoint() -> Result<(), String> {
    let backend = backend()?;

    let checkpoint = backend
        .load()?
        .ok_or_else(|| "no checkpoint available".to_string())?;

    /*
     * This is a completely new MemoryState.
     */

    let mut state = MemoryState::from_snapshot(checkpoint.state.clone());

    /*
     * And a completely new BlockProducer.
     */

    let mut producer = BlockProducer::resume(
        VERSION,
        CHAIN_ID,
        GAS_LIMIT,
        checkpoint.next_height,
        checkpoint.parent_hash,
    );

    let engine = RevmExecutionEngine::new();

    let sender = address!("1111111111111111111111111111111111111111");

    let recipient = address!("2222222222222222222222222222222222222222");

    /*
     * BLOCK #2
     */

    let tx_2 = SolizoneTransaction {
        sender,
        nonce: 2,
        kind: TransactionKind::Call(recipient),
        value: U256::from(300),
        data: Bytes::new(),
        gas_limit: 21_000,
    };

    let expected_parent = checkpoint.parent_hash;

    let block_2 = producer.produce_block(&engine, &mut state, &[tx_2], 1_800_000_002);

    assert_eq!(block_2.header.height, 2,);

    assert_eq!(
        block_2.header.parent_hash, expected_parent,
        "block #2 does not link to recovered block #1",
    );

    let sender_account = state
        .db()
        .cache
        .accounts
        .get(&sender)
        .and_then(|account| account.info())
        .ok_or_else(|| "sender missing after recovery".to_string())?;

    let recipient_account = state
        .db()
        .cache
        .accounts
        .get(&recipient)
        .and_then(|account| account.info())
        .ok_or_else(|| "recipient missing after recovery".to_string())?;

    assert_eq!(sender_account.nonce, 3,);

    assert_eq!(recipient_account.balance, U256::from(600),);

    println!("Recovered next height: {}", checkpoint.next_height,);

    println!("Recovered parent hash: {}", checkpoint.parent_hash,);

    println!("Block #2 height: {}", block_2.header.height,);

    println!("Block #2 parent: {}", block_2.header.parent_hash,);

    println!("Block #2 hash: {}", block_2.hash(),);

    println!("Final sender nonce: {}", sender_account.nonce,);

    println!("Recipient balance: {}", recipient_account.balance,);

    println!("PROCESS B recovery verified");

    Ok(())
}
