use std::{env, fs};

use revm::primitives::B256;

use solizone_evm::{
    block::{SolizoneBlock, SolizoneBlockHeader},
    block_store::BlockStore,
    execution::transactions_root::compute_transactions_root,
    hybrid_block_store::HybridBlockStore,
};

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let local_directory = root.join("hybrid-auto-blocks");

    let index_path = root.join("hybrid-auto-index.json");

    let recovery_directory = root.join("hybrid-auto-recovery");

    let archive_work_directory = root.join("hybrid-auto-archive-work");

    for path in [
        &local_directory,
        &recovery_directory,
        &archive_work_directory,
    ] {
        if path.exists() {
            fs::remove_dir_all(path).map_err(|error| error.to_string())?;
        }
    }

    if index_path.exists() {
        fs::remove_file(&index_path).map_err(|error| error.to_string())?;
    }

    let mut store = HybridBlockStore::new(
        &local_directory,
        &index_path,
        &recovery_directory,
        &archive_work_directory,
        3,
    );

    let mut parent_hash = B256::ZERO;

    for height in 0..10 {
        let transactions = vec![];

        let block = SolizoneBlock {
            header: SolizoneBlockHeader {
                version: 1,
                chain_id: 9001,
                height,
                parent_hash,
                timestamp: 1_800_000_000 + height,
                state_root: B256::from([(height + 1) as u8; 32]),
                transactions_root: compute_transactions_root(&transactions),
                receipts_root: B256::ZERO,
                gas_limit: 30_000_000,
                gas_used: 0,
            },
            transactions,
        };

        parent_hash = block.hash();

        store.insert(block)?;

        println!("Inserted block #{height}");

        println!("Local count: {}", store.local_len()?,);
    }

    assert_eq!(store.local_len()?, 3,);

    assert_eq!(store.archived_len()?, 7,);

    assert_eq!(store.len()?, 10,);

    let block_8 = store
        .get_by_height(8)?
        .ok_or_else(|| "block #8 missing".to_string())?;

    let block_2 = store
        .get_by_height(2)?
        .ok_or_else(|| "block #2 missing".to_string())?;

    assert_eq!(block_8.header.height, 8,);

    assert_eq!(block_2.header.height, 2,);

    println!("Final local blocks: {}", store.local_len()?,);

    println!("Final archived blocks: {}", store.archived_len()?,);

    println!("Total canonical blocks: {}", store.len()?,);

    println!("Recent block #8 loaded");

    println!("Archived block #2 loaded");

    println!("Automatic retention verified");

    Ok(())
}
