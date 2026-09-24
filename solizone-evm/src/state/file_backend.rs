use std::{
    io,
    path::{Path, PathBuf},
};

use super::{StateBackend, StateSnapshot};

/// Local filesystem persistence backend for Solizone state snapshots.
pub struct FileStateBackend {
    path: PathBuf,
}

impl FileStateBackend {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl StateBackend for FileStateBackend {
    type Error = io::Error;

    fn save(&self, snapshot: &StateSnapshot) -> Result<(), Self::Error> {
        let json = snapshot.encode_json().map_err(io::Error::other)?;

        std::fs::write(&self.path, json)
    }

    fn load(&self) -> Result<Option<StateSnapshot>, Self::Error> {
        if !self.path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&self.path)?;

        let snapshot = StateSnapshot::decode_json(&json).map_err(io::Error::other)?;

        Ok(Some(snapshot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revm::primitives::{U256, address};

    use crate::state::{AccountSnapshot, StorageSlotSnapshot};

    #[test]
    fn saves_and_loads_state_snapshot() {
        let snapshot_path = std::env::temp_dir().join("solizone-file-backend-test.json");

        // Make sure an old test file cannot affect this run.
        if snapshot_path.exists() {
            std::fs::remove_file(&snapshot_path).expect("failed to remove old snapshot");
        }

        let backend = FileStateBackend::new(snapshot_path.clone());

        let account_address = address!("1111111111111111111111111111111111111111");

        let snapshot = StateSnapshot {
            accounts: vec![AccountSnapshot {
                address: account_address,
                balance: U256::from(1000),
                nonce: 7,
                code: None,
                storage: vec![StorageSlotSnapshot {
                    key: U256::ZERO,
                    value: U256::from(42),
                }],
            }],
        };

        // No state exists yet.
        let missing_snapshot = backend.load().expect("failed to check empty backend");

        assert!(
            missing_snapshot.is_none(),
            "backend should initially contain no snapshot"
        );

        // Persist it.
        backend.save(&snapshot).expect("failed to save snapshot");

        assert!(backend.path().exists(), "snapshot file was not created");

        // Load it back.
        let loaded_snapshot = backend
            .load()
            .expect("failed to load snapshot")
            .expect("snapshot missing after save");

        assert_eq!(loaded_snapshot, snapshot);

        std::fs::remove_file(backend.path()).expect("failed to remove test snapshot");
    }
}
