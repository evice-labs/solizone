use std::{
    env, fs,
    path::Path,
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

const TOTAL_BLOCKS: u64 = 10;
const RETENTION_WINDOW: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveEntry {
    height: u64,
    block_hash: String,
    cid: String,
}

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;

    let block_directory = root.join("retention-window-blocks");

    let archive_work_directory = root.join("retention-window-archive-work");

    let recovery_directory = root.join("retention-window-recovery");

    let index_path = root.join("retention-window-index.json");

    clean_directory(&block_directory)?;

    clean_directory(&archive_work_directory)?;

    clean_directory(&recovery_directory)?;

    if index_path.exists() {
        fs::remove_file(&index_path)
            .map_err(|error| format!("failed to remove old archive index: {error}"))?;
    }

    fs::create_dir_all(&archive_work_directory)
        .map_err(|error| format!("failed to create archive work directory: {error}"))?;

    fs::create_dir_all(&recovery_directory)
        .map_err(|error| format!("failed to create recovery directory: {error}"))?;

    /*
     * STEP 1:
     *
     * Build and persist canonical blocks 0..9.
     */

    let mut store = FileBlockStore::new(&block_directory);

    let mut parent_hash = B256::ZERO;

    for height in 0..TOTAL_BLOCKS {
        let transactions = vec![];

        let state_byte = u8::try_from(height + 1).map_err(|error| error.to_string())?;

        let block = SolizoneBlock {
            header: SolizoneBlockHeader {
                version: 1,
                chain_id: 9001,
                height,
                parent_hash,
                timestamp: 1_800_000_000 + height,
                state_root: B256::from([state_byte; 32]),
                transactions_root: compute_transactions_root(&transactions),
                receipts_root: B256::ZERO,
                gas_limit: 30_000_000,
                gas_used: 0,
            },
            transactions,
        };

        block
            .validate()
            .map_err(|error| format!("block {height} failed validation: {error:?}"))?;

        parent_hash = block.hash();

        store
            .insert(block)
            .map_err(|error| format!("failed to persist block {height}: {error}"))?;
    }

    assert_eq!(store.len().map_err(|error| { error.to_string() })?, 10,);

    println!("Produced local blocks: 0..9");

    println!(
        "Local blocks before pruning: {}",
        store.len().map_err(|error| { error.to_string() })?,
    );

    /*
     * STEP 2:
     *
     * Calculate retention floor.
     *
     * Latest height = 9
     * Retention = 3
     *
     * Keep 7, 8, 9.
     */

    let latest_height = TOTAL_BLOCKS - 1;

    let local_floor = latest_height
        .saturating_add(1)
        .saturating_sub(RETENTION_WINDOW);

    println!("Retention window: {RETENTION_WINDOW}");

    println!("Keep local from height: {local_floor}");

    /*
     * STEP 3:
     *
     * Archive and prune everything
     * below local_floor.
     */

    let client = LogosStorageClient::new();

    let mut archive_index = Vec::new();

    for height in 0..local_floor {
        let block = store
            .get_by_height(height)
            .map_err(|error| format!("failed to read block {height}: {error}"))?
            .ok_or_else(|| format!("block {height} missing before archive"))?;

        let block_hash = block.hash();

        let local_path = block_directory.join(format!("{height}.szb"));

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_nanos();

        /*
         * Unique upload filename avoids
         * matching an old Storage manifest.
         */

        let archive_upload_path =
            archive_work_directory.join(format!("block-{height}-{timestamp}.szb"));

        fs::copy(&local_path, &archive_upload_path)
            .map_err(|error| format!("failed to prepare block {height} for archive: {error}"))?;

        let cid = client.upload(&archive_upload_path)?;

        archive_index.push(ArchiveEntry {
            height,
            block_hash: block_hash.to_string(),
            cid: cid.clone(),
        });

        /*
         * Only prune after CID exists.
         */

        fs::remove_file(&local_path)
            .map_err(|error| format!("failed to prune block {height}: {error}"))?;

        let _ = fs::remove_file(&archive_upload_path);

        println!("Archived + pruned block #{height}: {cid}");
    }

    /*
     * STEP 4:
     *
     * Persist archive index.
     */

    let index_json = serde_json::to_string_pretty(&archive_index)
        .map_err(|error| format!("failed to encode archive index: {error}"))?;

    fs::write(&index_path, index_json)
        .map_err(|error| format!("failed to persist archive index: {error}"))?;

    /*
     * STEP 5:
     *
     * Verify only blocks 7,8,9 remain.
     */

    let local_count = store.len().map_err(|error| error.to_string())?;

    assert_eq!(local_count, RETENTION_WINDOW as usize,);

    for height in 0..local_floor {
        assert!(
            store
                .get_by_height(height)
                .map_err(|error| { error.to_string() })?
                .is_none(),
            "old block {height} still exists locally",
        );
    }

    for height in local_floor..TOTAL_BLOCKS {
        assert!(
            store
                .get_by_height(height)
                .map_err(|error| { error.to_string() })?
                .is_some(),
            "recent block {height} should remain local",
        );
    }

    assert_eq!(archive_index.len(), 7,);

    println!("Local blocks after pruning: {local_count}");

    println!("Archived blocks: {}", archive_index.len(),);

    /*
     * STEP 6:
     *
     * Prove recent block #8 is local.
     */

    let recent = store
        .get_by_height(8)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "recent block #8 missing locally".to_string())?;

    println!("Recent block #8 served locally: {}", recent.hash(),);

    /*
     * STEP 7:
     *
     * Prove old block #2 is NOT local.
     */

    assert!(
        store
            .get_by_height(2)
            .map_err(|error| { error.to_string() })?
            .is_none(),
    );

    println!("Old block #2 is not local");

    /*
     * STEP 8:
     *
     * Resolve block #2 through archive index.
     */

    let archived = archive_index
        .iter()
        .find(|entry| entry.height == 2)
        .ok_or_else(|| "block #2 missing from archive index".to_string())?;

    println!("Block #2 archive CID: {}", archived.cid,);

    /*
     * STEP 9:
     *
     * Download old block without putting it
     * back into the retained local block store.
     */

    let recovered_path = recovery_directory.join("2.szb");

    client.download(&archived.cid, &recovered_path)?;

    let recovered_bytes = fs::read(&recovered_path)
        .map_err(|error| format!("failed to read recovered block: {error}"))?;

    let recovered = SolizoneBlock::decode_and_validate(&recovered_bytes)
        .map_err(|error| format!("recovered block failed validation: {error:?}"))?;

    /*
     * STEP 10:
     *
     * Verify canonical identity.
     */

    assert_eq!(recovered.header.height, 2,);

    assert_eq!(recovered.hash().to_string(), archived.block_hash,);

    println!(
        "Recovered archived block height: {}",
        recovered.header.height,
    );

    println!("Recovered archived block hash: {}", recovered.hash(),);

    println!("Indexed canonical hash: {}", archived.block_hash,);

    println!("Retention window verified");

    println!("Bounded local block storage verified");

    Ok(())
}

fn clean_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to clean {}: {error}", path.display(),))?;
    }

    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display(),))?;

    Ok(())
}
