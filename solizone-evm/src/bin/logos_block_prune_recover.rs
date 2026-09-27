use std::{
    env, fs,
    time::{SystemTime, UNIX_EPOCH},
};

use revm::primitives::B256;
use serde::{Deserialize, Serialize};

use solizone_evm::{
    block::{SolizoneBlock, SolizoneBlockHeader},
    block_store::BlockStore,
    execution::transactions_root::compute_transactions_root,
    file_block_store::FileBlockStore,
    logos_storage_client::LogosStorageClient,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveEntry {
    height: u64,
    block_hash: String,
    cid: String,
}

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let block_directory = root.join("storage-block-prune-test");

    let archive_work_directory = root.join("storage-block-archive-work");

    let index_path = root.join("block-archive-index.json");

    /*
     * Start clean.
     */

    if block_directory.exists() {
        fs::remove_dir_all(&block_directory)
            .map_err(|error| format!("failed to clean block directory: {error}"))?;
    }

    if archive_work_directory.exists() {
        fs::remove_dir_all(&archive_work_directory)
            .map_err(|error| format!("failed to clean archive directory: {error}"))?;
    }

    if index_path.exists() {
        fs::remove_file(&index_path)
            .map_err(|error| format!("failed to remove old index: {error}"))?;
    }

    fs::create_dir_all(&archive_work_directory)
        .map_err(|error| format!("failed to create archive work directory: {error}"))?;

    /*
     * Create canonical block #7.
     */

    let transactions = vec![];

    let block = SolizoneBlock {
        header: SolizoneBlockHeader {
            version: 1,
            chain_id: 9001,
            height: 7,
            parent_hash: B256::from([0x11; 32]),
            timestamp: 1_800_000_000,
            state_root: B256::from([0x22; 32]),
            transactions_root: compute_transactions_root(&transactions),
            receipts_root: B256::from([0x44; 32]),
            gas_limit: 30_000_000,
            gas_used: 0,
        },
        transactions,
    };

    let expected_hash = block.hash();

    /*
     * Persist using FileBlockStore.
     */

    let mut store = FileBlockStore::new(&block_directory);

    store
        .insert(block)
        .map_err(|error| format!("failed to persist block: {error}"))?;

    let local_path = block_directory.join("7.szb");

    if !local_path.exists() {
        return Err("7.szb was not created".to_string());
    }

    println!("Local block exists: {}", local_path.display(),);

    println!("Canonical block hash: {}", expected_hash,);

    /*
     * Make a uniquely named archive upload file.
     *
     * This avoids relying on an old manifest
     * with the same filename.
     */

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();

    let archive_upload_path = archive_work_directory.join(format!("block-7-{timestamp}.szb"));

    fs::copy(&local_path, &archive_upload_path)
        .map_err(|error| format!("failed to create archive upload file: {error}"))?;

    /*
     * Upload to Logos Storage.
     */

    let client = LogosStorageClient::new();

    let cid = client.upload(&archive_upload_path)?;

    println!("Archived CID: {cid}");

    /*
     * Persist archive index.
     */

    let entry = ArchiveEntry {
        height: 7,
        block_hash: expected_hash.to_string(),
        cid: cid.clone(),
    };

    let index = vec![entry];

    let index_json = serde_json::to_string_pretty(&index)
        .map_err(|error| format!("failed to encode archive index: {error}"))?;

    fs::write(&index_path, index_json)
        .map_err(|error| format!("failed to persist archive index: {error}"))?;

    println!("Archive index written: {}", index_path.display(),);

    /*
     * Important:
     *
     * Delete the canonical local block.
     */

    fs::remove_file(&local_path)
        .map_err(|error| format!("failed to prune local block: {error}"))?;

    assert!(
        !local_path.exists(),
        "local block still exists after pruning",
    );

    println!("Local block #7 pruned");

    /*
     * Simulate later recovery by HEIGHT.
     */

    let index_json = fs::read_to_string(&index_path)
        .map_err(|error| format!("failed to read archive index: {error}"))?;

    let index: Vec<ArchiveEntry> = serde_json::from_str(&index_json)
        .map_err(|error| format!("failed to decode archive index: {error}"))?;

    let archived = index
        .iter()
        .find(|entry| entry.height == 7)
        .ok_or_else(|| "height 7 missing from archive index".to_string())?;

    println!("Archive lookup height: {}", archived.height,);

    println!("Archive lookup CID: {}", archived.cid,);

    /*
     * Download archived block.
     */

    let recovered_path = block_directory.join("7.szb");

    client.download(&archived.cid, &recovered_path)?;

    /*
     * Decode + protocol validation.
     */

    let bytes = fs::read(&recovered_path)
        .map_err(|error| format!("failed to read recovered block: {error}"))?;

    let recovered = SolizoneBlock::decode_and_validate(&bytes)
        .map_err(|error| format!("recovered block failed validation: {error:?}"))?;

    let recovered_hash = recovered.hash();

    /*
     * Verify against archive index.
     */

    assert_eq!(recovered.header.height, archived.height,);

    assert_eq!(
        recovered_hash.to_string(),
        archived.block_hash,
        "recovered hash differs from archived canonical hash",
    );

    assert_eq!(recovered_hash, expected_hash,);

    println!("Recovered height: {}", recovered.header.height,);

    println!("Recovered hash: {}", recovered_hash,);

    println!("Indexed hash: {}", archived.block_hash,);

    println!("Pruned block recovered and verified");

    println!("Logos Storage prune-and-recover verified");

    Ok(())
}
