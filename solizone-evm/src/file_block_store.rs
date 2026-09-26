use std::{
    fs, io,
    path::{Path, PathBuf},
};

use revm::primitives::B256;

use crate::{block::SolizoneBlock, block_store::BlockStore};

pub struct FileBlockStore {
    directory: PathBuf,
}

impl FileBlockStore {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    fn block_path(&self, height: u64) -> PathBuf {
        self.directory.join(format!("{height}.szb"))
    }
}

impl BlockStore for FileBlockStore {
    type Error = io::Error;

    fn insert(&mut self, block: SolizoneBlock) -> Result<(), Self::Error> {
        fs::create_dir_all(&self.directory)?;

        let path = self.block_path(block.header.height);

        fs::write(path, block.encode())?;

        Ok(())
    }

    fn get_by_height(&self, height: u64) -> Result<Option<SolizoneBlock>, Self::Error> {
        let path = self.block_path(height);

        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(path)?;

        let block = SolizoneBlock::decode(&bytes).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to decode block: {error:?}"),
            )
        })?;

        Ok(Some(block))
    }

    fn get_by_hash(&self, hash: B256) -> Result<Option<SolizoneBlock>, Self::Error> {
        if !self.directory.exists() {
            return Ok(None);
        }

        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;

            let path = entry.path();

            /*
             * Ignore anything that is not
             * a Solizone block file.
             */
            if path.extension().and_then(|extension| extension.to_str()) != Some("szb") {
                continue;
            }

            let bytes = fs::read(&path)?;

            let block = SolizoneBlock::decode(&bytes).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to decode block {}: {error:?}", path.display()),
                )
            })?;

            if block.hash() == hash {
                return Ok(Some(block));
            }
        }

        Ok(None)
    }

    fn len(&self) -> Result<usize, Self::Error> {
        if !self.directory.exists() {
            return Ok(0);
        }

        let count = fs::read_dir(&self.directory)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "szb")
            })
            .count();

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use revm::primitives::B256;

    use crate::block::SolizoneBlockHeader;

    #[test]
    fn persists_and_recovers_block_by_height() {
        let directory = std::env::temp_dir().join("solizone-file-block-store-test");

        /*
         * Start with a clean directory.
         */

        if directory.exists() {
            std::fs::remove_dir_all(&directory).expect("failed to remove old block store");
        }

        let block = SolizoneBlock {
            header: SolizoneBlockHeader {
                version: 1,
                chain_id: 9001,
                height: 7,
                parent_hash: B256::from([0x11; 32]),
                timestamp: 1_800_000_000,
                state_root: B256::from([0x22; 32]),
                transactions_root: B256::from([0x33; 32]),
                receipts_root: B256::from([0x44; 32]),
                gas_limit: 30_000_000,
                gas_used: 0,
            },
            transactions: vec![],
        };

        let block_hash = block.hash();

        /*
         * PROCESS A:
         *
         * Persist the block.
         */

        {
            let mut store = FileBlockStore::new(&directory);

            store.insert(block).expect("failed to persist block");

            assert_eq!(store.len().unwrap(), 1,);

            assert!(directory.join("7.szb").exists());
        }

        /*
         * PROCESS B:
         *
         * Create a completely new store
         * instance pointing at the same
         * directory.
         */

        let store = FileBlockStore::new(&directory);

        let recovered = store
            .get_by_height(7)
            .expect("failed to read block")
            .expect("persisted block missing");

        assert_eq!(recovered.header.height, 7,);

        assert_eq!(recovered.hash(), block_hash,);

        assert_eq!(store.len().unwrap(), 1,);

        println!("Recovered block height: {}", recovered.header.height);

        println!("Recovered block hash:   {}", recovered.hash());

        println!("Block history size:     {}", store.len().unwrap());

        std::fs::remove_dir_all(&directory).expect("failed to clean block store");
    }

    #[test]
    fn persists_and_recovers_block_by_hash() {
        let directory = std::env::temp_dir().join("solizone-file-block-store-hash-test");

        if directory.exists() {
            std::fs::remove_dir_all(&directory).expect("failed to remove old block store");
        }

        let block = SolizoneBlock {
            header: SolizoneBlockHeader {
                version: 1,
                chain_id: 9001,
                height: 7,
                parent_hash: B256::from([0x11; 32]),
                timestamp: 1_800_000_000,
                state_root: B256::from([0x22; 32]),
                transactions_root: B256::from([0x33; 32]),
                receipts_root: B256::from([0x44; 32]),
                gas_limit: 30_000_000,
                gas_used: 0,
            },
            transactions: vec![],
        };

        let block_hash = block.hash();

        /*
         * PROCESS A:
         *
         * Persist the block.
         */

        {
            let mut store = FileBlockStore::new(&directory);

            store.insert(block).expect("failed to persist block");
        }

        /*
         * PROCESS B:
         *
         * New FileBlockStore with no
         * in-memory hash index.
         */

        let store = FileBlockStore::new(&directory);

        let recovered = store
            .get_by_hash(block_hash)
            .expect("hash lookup failed")
            .expect("block missing by hash");

        assert_eq!(recovered.header.height, 7,);

        assert_eq!(recovered.hash(), block_hash,);

        /*
         * Unknown hashes must return None.
         */

        assert!(
            store
                .get_by_hash(B256::from([0xff; 32]),)
                .unwrap()
                .is_none()
        );

        println!("Recovered by hash: {}", recovered.hash());

        println!("Recovered height:  {}", recovered.header.height);

        std::fs::remove_dir_all(&directory).expect("failed to clean block store");
    }

    #[test]
    fn replacing_block_removes_old_canonical_hash() {
        let directory = std::env::temp_dir().join("solizone-file-block-store-replacement-test");

        if directory.exists() {
            std::fs::remove_dir_all(&directory).expect("failed to remove old block store");
        }

        let old_block = SolizoneBlock {
            header: SolizoneBlockHeader {
                version: 1,
                chain_id: 9001,
                height: 7,
                parent_hash: B256::ZERO,
                timestamp: 1_800_000_000,
                state_root: B256::from([0x11; 32]),
                transactions_root: B256::from([0x22; 32]),
                receipts_root: B256::from([0x33; 32]),
                gas_limit: 30_000_000,
                gas_used: 0,
            },
            transactions: vec![],
        };

        let new_block = SolizoneBlock {
            header: SolizoneBlockHeader {
                version: 1,
                chain_id: 9001,
                height: 7,
                parent_hash: B256::ZERO,
                timestamp: 1_800_000_001,
                state_root: B256::from([0x44; 32]),
                transactions_root: B256::from([0x55; 32]),
                receipts_root: B256::from([0x66; 32]),
                gas_limit: 30_000_000,
                gas_used: 0,
            },
            transactions: vec![],
        };

        let old_hash = old_block.hash();

        let new_hash = new_block.hash();

        assert_ne!(old_hash, new_hash,);

        /*
         * Write block #7, then replace the
         * canonical block at the same height.
         */

        {
            let mut store = FileBlockStore::new(&directory);

            store
                .insert(old_block)
                .expect("failed to persist old block");

            store
                .insert(new_block)
                .expect("failed to persist replacement block");

            assert_eq!(store.len().unwrap(), 1,);
        }

        /*
         * Reopen the store from disk.
         */

        let store = FileBlockStore::new(&directory);

        let canonical = store
            .get_by_height(7)
            .unwrap()
            .expect("canonical block missing");

        assert_eq!(canonical.hash(), new_hash,);

        /*
         * The replaced block must no longer
         * be reachable by its old hash.
         */

        assert!(store.get_by_hash(old_hash).unwrap().is_none());

        assert_eq!(
            store
                .get_by_hash(new_hash)
                .unwrap()
                .expect("replacement block missing by hash",)
                .hash(),
            new_hash,
        );

        assert_eq!(store.len().unwrap(), 1,);

        println!("Old canonical hash removed: {}", old_hash);

        println!("New canonical hash:         {}", new_hash);

        println!("Canonical history size:     {}", store.len().unwrap());

        std::fs::remove_dir_all(&directory).expect("failed to clean block store");
    }
}
