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
