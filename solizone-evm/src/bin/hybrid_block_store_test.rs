use std::env;

use solizone_evm::{block_store::BlockStore, hybrid_block_store::HybridBlockStore};

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let store = HybridBlockStore::new(
        root.join("retention-window-blocks"),
        root.join("retention-window-index.json"),
        root.join("hybrid-block-recovery"),
        root.join("hybrid-block-archive-work"),
        3,
    );

    println!("Local blocks: {}", store.local_len()?,);

    println!("Archived blocks: {}", store.archived_len()?,);

    println!("Total canonical blocks: {}", store.len()?,);

    /*
     * RECENT BLOCK:
     * should come directly from disk.
     */

    let block_8 = store
        .get_by_height(8)?
        .ok_or_else(|| "block #8 missing".to_string())?;

    println!("Block #8 loaded: {}", block_8.hash(),);

    /*
     * OLD BLOCK:
     * not local, so HybridBlockStore
     * must transparently use Logos Storage.
     */

    let block_2 = store
        .get_by_height(2)?
        .ok_or_else(|| "block #2 missing".to_string())?;

    println!("Block #2 loaded: {}", block_2.hash(),);

    assert_eq!(block_8.header.height, 8,);

    assert_eq!(block_2.header.height, 2,);

    /*
     * Prove archived hash lookup too.
     */

    let block_2_by_hash = store
        .get_by_hash(block_2.hash())?
        .ok_or_else(|| "block #2 missing by hash".to_string())?;

    assert_eq!(block_2_by_hash, block_2,);

    println!("Block #2 hash lookup verified");

    println!("HybridBlockStore verified");

    Ok(())
}
