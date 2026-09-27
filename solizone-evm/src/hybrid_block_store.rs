use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use revm::primitives::B256;
use serde::{Deserialize, Serialize};

use crate::{
    block::SolizoneBlock, block_store::BlockStore, file_block_store::FileBlockStore,
    logos_storage_client::LogosStorageClient,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub height: u64,
    pub block_hash: String,
    pub cid: String,
}

pub struct HybridBlockStore {
    local: FileBlockStore,
    archive_index_path: PathBuf,
    recovery_directory: PathBuf,
    archive_work_directory: PathBuf,
    retention_window: usize,
    storage: LogosStorageClient,
}

impl HybridBlockStore {
    pub fn new(
        local_directory: impl Into<PathBuf>,
        archive_index_path: impl Into<PathBuf>,
        recovery_directory: impl Into<PathBuf>,
        archive_work_directory: impl Into<PathBuf>,
        retention_window: usize,
    ) -> Self {
        Self {
            local: FileBlockStore::new(local_directory),
            archive_index_path: archive_index_path.into(),
            recovery_directory: recovery_directory.into(),
            archive_work_directory: archive_work_directory.into(),
            retention_window,
            storage: LogosStorageClient::new(),
        }
    }

    pub fn local_store(&self) -> &FileBlockStore {
        &self.local
    }

    pub fn local_store_mut(&mut self) -> &mut FileBlockStore {
        &mut self.local
    }

    fn load_archive_index(&self) -> Result<Vec<ArchiveEntry>, String> {
        if !self.archive_index_path.exists() {
            return Ok(Vec::new());
        }

        let json = fs::read_to_string(&self.archive_index_path)
            .map_err(|error| format!("failed to read archive index: {error}"))?;

        serde_json::from_str(&json)
            .map_err(|error| format!("failed to decode archive index: {error}"))
    }

    fn recover_archived(&self, entry: &ArchiveEntry) -> Result<SolizoneBlock, String> {
        fs::create_dir_all(&self.recovery_directory)
            .map_err(|error| format!("failed to create recovery directory: {error}"))?;

        let path = self
            .recovery_directory
            .join(format!("{}-recovered.szb", entry.height,));

        self.storage.download(&entry.cid, &path)?;

        let bytes =
            fs::read(&path).map_err(|error| format!("failed to read recovered block: {error}"))?;

        let block = SolizoneBlock::decode_and_validate(&bytes)
            .map_err(|error| format!("archived block failed validation: {error:?}"))?;

        if block.header.height != entry.height {
            return Err(format!(
                "archived block height mismatch: expected {}, got {}",
                entry.height, block.header.height,
            ));
        }

        if block.hash().to_string() != entry.block_hash {
            return Err(format!(
                "archived block hash mismatch at height {}",
                entry.height,
            ));
        }

        Ok(block)
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<SolizoneBlock>, String> {
        /*
         * Fast path:
         * recent local history.
         */

        if let Some(block) = self
            .local
            .get_by_height(height)
            .map_err(|error| format!("local block lookup failed: {error}"))?
        {
            return Ok(Some(block));
        }

        /*
         * Slow path:
         * archived history.
         */

        let index = self.load_archive_index()?;

        let Some(entry) = index.iter().find(|entry| entry.height == height) else {
            return Ok(None);
        };

        let block = self.recover_archived(entry)?;

        Ok(Some(block))
    }

    fn get_block_by_hash(&self, hash: B256) -> Result<Option<SolizoneBlock>, String> {
        /*
         * Check retained local blocks first.
         */

        if let Some(block) = self
            .local
            .get_by_hash(hash)
            .map_err(|error| format!("local hash lookup failed: {error}"))?
        {
            return Ok(Some(block));
        }

        let expected_hash = hash.to_string();

        let index = self.load_archive_index()?;

        let Some(entry) = index.iter().find(|entry| entry.block_hash == expected_hash) else {
            return Ok(None);
        };

        let block = self.recover_archived(entry)?;

        Ok(Some(block))
    }

    fn insert_block(&mut self, block: SolizoneBlock) -> Result<(), String> {
        self.local
            .insert(block)
            .map_err(|error| format!("failed to insert local block: {error}"))?;

        self.enforce_retention()
    }

    pub fn local_len(&self) -> Result<usize, String> {
        self.local.len().map_err(|error| error.to_string())
    }

    pub fn archived_len(&self) -> Result<usize, String> {
        Ok(self.load_archive_index()?.len())
    }

    fn total_len(&self) -> Result<usize, String> {
        Ok(self.local_len()? + self.archived_len()?)
    }

    pub fn recovery_directory(&self) -> &Path {
        &self.recovery_directory
    }

    fn save_archive_index(&self, index: &[ArchiveEntry]) -> Result<(), String> {
        if let Some(parent) = self.archive_index_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create archive index directory: {error}"))?;
        }

        let json = serde_json::to_string_pretty(index)
            .map_err(|error| format!("failed to encode archive index: {error}"))?;

        fs::write(&self.archive_index_path, json)
            .map_err(|error| format!("failed to persist archive index: {error}"))
    }

    fn archive_block(&self, block: &SolizoneBlock) -> Result<(), String> {
        let height = block.header.height;

        let hash = block.hash();

        let local_path = self.local.directory().join(format!("{height}.szb"));

        if !local_path.exists() {
            return Err(format!("local block {height} missing before archive"));
        }

        fs::create_dir_all(&self.archive_work_directory)
            .map_err(|error| format!("failed to create archive work directory: {error}"))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_nanos();

        let upload_path = self
            .archive_work_directory
            .join(format!("block-{height}-{timestamp}.szb"));

        fs::copy(&local_path, &upload_path)
            .map_err(|error| format!("failed to prepare block {height} for archive: {error}"))?;

        let cid = self.storage.upload(&upload_path)?;

        let mut index = self.load_archive_index()?;

        index.retain(|entry| entry.height != height);

        index.push(ArchiveEntry {
            height,
            block_hash: hash.to_string(),
            cid: cid.clone(),
        });

        index.sort_by_key(|entry| entry.height);

        /*
         * Important ordering:
         *
         * Persist the CID before deleting
         * the canonical local block.
         */

        self.save_archive_index(&index)?;

        fs::remove_file(&local_path)
            .map_err(|error| format!("failed to prune archived block {height}: {error}"))?;

        let _ = fs::remove_file(&upload_path);

        println!("Archived + pruned block #{height}: {cid}");

        Ok(())
    }

    fn enforce_retention(&self) -> Result<(), String> {
        if self.retention_window == 0 {
            return Ok(());
        }

        loop {
            let local_count = self.local.len().map_err(|error| error.to_string())?;

            if local_count <= self.retention_window {
                break;
            }

            let mut heights = Vec::new();

            for entry in fs::read_dir(self.local.directory())
                .map_err(|error| format!("failed to inspect local block directory: {error}"))?
            {
                let entry = entry.map_err(|error| error.to_string())?;

                let path = entry.path();

                if path.extension().and_then(|value| value.to_str()) != Some("szb") {
                    continue;
                }

                let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };

                if let Ok(height) = stem.parse::<u64>() {
                    heights.push(height);
                }
            }

            heights.sort_unstable();

            let oldest_height = *heights.first().ok_or_else(|| {
                "local block count was non-zero but no block files were found".to_string()
            })?;

            let block = self
                .local
                .get_by_height(oldest_height)
                .map_err(|error| format!("failed to read block {oldest_height}: {error}"))?
                .ok_or_else(|| format!("oldest local block {oldest_height} missing"))?;

            self.archive_block(&block)?;
        }

        Ok(())
    }
}

impl BlockStore for HybridBlockStore {
    type Error = String;

    fn insert(&mut self, block: SolizoneBlock) -> Result<(), Self::Error> {
        self.insert_block(block)
    }

    fn get_by_height(&self, height: u64) -> Result<Option<SolizoneBlock>, Self::Error> {
        self.get_block_by_height(height)
    }

    fn get_by_hash(&self, hash: B256) -> Result<Option<SolizoneBlock>, Self::Error> {
        self.get_block_by_hash(hash)
    }

    fn len(&self) -> Result<usize, Self::Error> {
        self.total_len()
    }
}
