use std::{
    io,
    path::{Path, PathBuf},
};

use super::SolizoneCheckpoint;

pub struct FileCheckpointBackend {
    path: PathBuf,
}

impl FileCheckpointBackend {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self, checkpoint: &SolizoneCheckpoint) -> Result<(), io::Error> {
        let json = checkpoint.encode_json().map_err(io::Error::other)?;

        std::fs::write(&self.path, json)
    }

    pub fn load(&self) -> Result<Option<SolizoneCheckpoint>, io::Error> {
        if !self.path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&self.path)?;

        let checkpoint = SolizoneCheckpoint::decode_json(&json).map_err(io::Error::other)?;

        Ok(Some(checkpoint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use revm::primitives::{B256, U256, address};

    use crate::state::{AccountSnapshot, StateSnapshot, StorageSlotSnapshot};

    #[test]
    fn saves_and_loads_checkpoint() {
        let checkpoint_path = std::env::temp_dir().join("solizone-checkpoint-backend-test.json");

        if checkpoint_path.exists() {
            std::fs::remove_file(&checkpoint_path).expect("failed to remove old checkpoint");
        }

        let backend = FileCheckpointBackend::new(checkpoint_path.clone());

        /*
         * Nothing persisted yet.
         */

        let missing = backend.load().expect("failed to check empty backend");

        assert!(
            missing.is_none(),
            "backend should initially contain no checkpoint"
        );

        /*
         * Build checkpoint.
         */

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

        let checkpoint = SolizoneCheckpoint::new(state, 3, B256::from([0x22; 32]));

        /*
         * Persist complete checkpoint.
         */

        backend
            .save(&checkpoint)
            .expect("failed to save checkpoint");

        assert!(backend.path().exists(), "checkpoint file was not created");

        /*
         * Recover it from disk.
         */

        let loaded = backend
            .load()
            .expect("failed to load checkpoint")
            .expect("checkpoint missing after save");

        assert_eq!(loaded, checkpoint,);

        assert_eq!(loaded.next_height, 3,);

        assert_eq!(loaded.parent_hash, B256::from([0x22; 32]),);

        println!("Checkpoint path: {:?}", backend.path());

        println!("Recovered next height: {}", loaded.next_height);

        println!("Recovered parent hash: {}", loaded.parent_hash);

        std::fs::remove_file(backend.path()).expect("failed to remove checkpoint");
    }
}
