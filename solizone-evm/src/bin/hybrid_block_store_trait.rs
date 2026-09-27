use std::env;

use solizone_evm::{block_store::BlockStore, hybrid_block_store::HybridBlockStore};

fn inspect_store<S>(store: &S) -> Result<(), S::Error>
where
    S: BlockStore,
{
    println!("Store length: {}", store.len()?,);

    println!("Store empty: {}", store.is_empty()?,);

    Ok(())
}

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let store = HybridBlockStore::new(
        root.join("hybrid-auto-blocks"),
        root.join("hybrid-auto-index.json"),
        root.join("hybrid-trait-recovery"),
        root.join("hybrid-trait-archive-work"),
        3,
    );

    inspect_store(&store)?;

    let block_8 =
        BlockStore::get_by_height(&store, 8)?.ok_or_else(|| "block #8 missing".to_string())?;

    let block_2 =
        BlockStore::get_by_height(&store, 2)?.ok_or_else(|| "block #2 missing".to_string())?;

    println!("Block #8 via BlockStore: {}", block_8.hash(),);

    println!("Block #2 via BlockStore: {}", block_2.hash(),);

    println!("HybridBlockStore implements BlockStore");

    Ok(())
}
