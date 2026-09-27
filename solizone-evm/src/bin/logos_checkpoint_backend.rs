use std::env;

use revm::primitives::{B256, U256, address};

use solizone_evm::state::{
    AccountSnapshot, CheckpointBackend, LogosStorageCheckpointBackend, SolizoneCheckpoint,
    StateSnapshot, StorageSlotSnapshot,
};

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let backend = LogosStorageCheckpointBackend::new(
        root.join("storage-checkpoint-work"),
        root.join("latest-checkpoint.cid"),
    );

    let checkpoint = test_checkpoint();

    println!("Saving checkpoint...");

    backend.save(&checkpoint)?;

    println!("Loading checkpoint...");

    let recovered = backend
        .load()?
        .ok_or_else(|| "checkpoint was not recovered".to_string())?;

    assert_eq!(
        recovered, checkpoint,
        "recovered checkpoint differs from original",
    );

    println!("Next height: {}", recovered.next_height,);

    println!("Parent hash: {:#x}", recovered.parent_hash,);

    println!("Logos Storage checkpoint backend verified");

    Ok(())
}

fn test_checkpoint() -> SolizoneCheckpoint {
    let state = StateSnapshot {
        accounts: vec![AccountSnapshot {
            address: address!("1111111111111111111111111111111111111111"),
            balance: U256::from(1000),
            nonce: 2,
            code: None,
            storage: vec![StorageSlotSnapshot {
                key: U256::ZERO,
                value: U256::from(42),
            }],
        }],
    };

    SolizoneCheckpoint::new(state, 10, B256::from([0x22; 32]))
}
