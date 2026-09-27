use std::{env, fs};

use revm::primitives::B256;

use solizone_evm::{
    block::{SolizoneBlock, SolizoneBlockHeader},
    block_store::BlockStore,
    file_block_store::FileBlockStore,
    logos_storage_client::LogosStorageClient,
};

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let block_directory = root.join("storage-block-archive-test");

    if block_directory.exists() {
        fs::remove_dir_all(&block_directory)
            .map_err(|error| format!("failed to clean old block directory: {error}"))?;
    }

    /*
     * Build one canonical Solizone block.
     */

    let block = SolizoneBlock {
        header: SolizoneBlockHeader {
            version: 1,
            chain_id: 9001,
            height: 7,
            parent_hash: B256::from([0x11; 32]),
            timestamp: 1_800_000_000,
            state_root: B256::from([0x22; 32]),
            transactions_root: B256::ZERO,
            receipts_root: B256::from([0x44; 32]),
            gas_limit: 30_000_000,
            gas_used: 0,
        },
        transactions: vec![],
    };

    /*
     * Empty transactions mean the correct
     * transactions root must come from the
     * actual block encoding rules.
     *
     * Build the block again using its actual
     * computed transactions root.
     */

    let block = SolizoneBlock {
        header: SolizoneBlockHeader {
            transactions_root:
                solizone_evm::execution::transactions_root::compute_transactions_root(
                    &block.transactions,
                ),
            ..block.header
        },
        transactions: block.transactions,
    };

    let expected_hash = block.hash();

    /*
     * Persist through the real FileBlockStore.
     */

    let mut store = FileBlockStore::new(&block_directory);

    store
        .insert(block)
        .map_err(|error| format!("failed to persist block: {error}"))?;

    let original_path = block_directory.join("7.szb");

    if !original_path.exists() {
        return Err("FileBlockStore did not create 7.szb".to_string());
    }

    println!("Local block created: {}", original_path.display(),);

    println!("Original block hash: {}", expected_hash,);

    /*
     * Archive it to Logos Storage.
     */

    let client = LogosStorageClient::new();

    let cid = client.upload(&original_path)?;

    println!("Archived block CID: {cid}");

    /*
     * Recover into a separate file.
     */

    let recovered_path = block_directory.join("7-recovered.szb");

    client.download(&cid, &recovered_path)?;

    /*
     * Exact-byte verification.
     */

    let original_bytes = fs::read(&original_path)
        .map_err(|error| format!("failed to read original block: {error}"))?;

    let recovered_bytes = fs::read(&recovered_path)
        .map_err(|error| format!("failed to read recovered block: {error}"))?;

    assert_eq!(
        original_bytes, recovered_bytes,
        "recovered block bytes differ from original",
    );

    /*
     * Protocol-level decode + validation.
     */

    let recovered_block = SolizoneBlock::decode_and_validate(&recovered_bytes)
        .map_err(|error| format!("recovered block failed validation: {error:?}"))?;

    assert_eq!(recovered_block.header.height, 7,);

    assert_eq!(recovered_block.hash(), expected_hash,);

    println!("Recovered block height: {}", recovered_block.header.height,);

    println!("Recovered block hash: {}", recovered_block.hash(),);

    println!("Block bytes are identical");

    println!("Recovered block validated");

    println!("Logos Storage block archive verified");

    Ok(())
}
