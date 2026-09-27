use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::logos_storage_client::LogosStorageClient;

use super::{CheckpointBackend, SolizoneCheckpoint};

#[derive(Debug, Clone)]
pub struct LogosStorageCheckpointBackend {
    client: LogosStorageClient,
    work_dir: PathBuf,
    cid_path: PathBuf,
}

impl LogosStorageCheckpointBackend {
    pub fn new(work_dir: impl Into<PathBuf>, cid_path: impl Into<PathBuf>) -> Self {
        Self {
            client: LogosStorageClient::new(),
            work_dir: work_dir.into(),
            cid_path: cid_path.into(),
        }
    }

    pub fn cid_path(&self) -> &Path {
        &self.cid_path
    }

    fn create_upload_path(&self, checkpoint: &SolizoneCheckpoint) -> Result<PathBuf, String> {
        fs::create_dir_all(&self.work_dir)
            .map_err(|error| format!("failed to create checkpoint work directory: {error}"))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_nanos();

        Ok(self.work_dir.join(format!(
            "solizone-checkpoint-{}-{timestamp}.json",
            checkpoint.next_height,
        )))
    }

    fn download_path(&self) -> PathBuf {
        self.work_dir.join("solizone-checkpoint-recovered.json")
    }
}

impl CheckpointBackend for LogosStorageCheckpointBackend {
    type Error = String;

    fn save(&self, checkpoint: &SolizoneCheckpoint) -> Result<(), Self::Error> {
        let upload_path = self.create_upload_path(checkpoint)?;

        let json = checkpoint
            .encode_json()
            .map_err(|error| format!("failed to encode checkpoint: {error}"))?;

        fs::write(&upload_path, json)
            .map_err(|error| format!("failed to write checkpoint upload file: {error}"))?;

        let cid = self.client.upload(&upload_path)?;

        if let Some(parent) = self.cid_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create CID directory: {error}"))?;
        }

        fs::write(&self.cid_path, &cid)
            .map_err(|error| format!("failed to persist checkpoint CID: {error}"))?;

        let _ = fs::remove_file(&upload_path);

        println!("Checkpoint uploaded to Logos Storage");

        println!("Checkpoint CID: {cid}");

        Ok(())
    }

    fn load(&self) -> Result<Option<SolizoneCheckpoint>, Self::Error> {
        if !self.cid_path.exists() {
            return Ok(None);
        }

        let cid = fs::read_to_string(&self.cid_path)
            .map_err(|error| format!("failed to read checkpoint CID: {error}"))?;

        let cid = cid.trim();

        if cid.is_empty() {
            return Ok(None);
        }

        fs::create_dir_all(&self.work_dir)
            .map_err(|error| format!("failed to create checkpoint work directory: {error}"))?;

        let download_path = self.download_path();

        self.client.download(cid, &download_path)?;

        let json = fs::read_to_string(&download_path)
            .map_err(|error| format!("failed to read downloaded checkpoint: {error}"))?;

        let checkpoint = SolizoneCheckpoint::decode_json(&json)
            .map_err(|error| format!("failed to decode downloaded checkpoint: {error}"))?;

        let _ = fs::remove_file(&download_path);

        println!("Checkpoint recovered from Logos Storage");

        println!("Checkpoint CID: {cid}");

        Ok(Some(checkpoint))
    }
}
