use std::collections::HashMap;

use revm::primitives::B256;

use crate::{block::SolizoneBlock, block_store::BlockStore};

pub struct MemoryBlockStore {
    blocks_by_height: HashMap<u64, SolizoneBlock>,
    height_by_hash: HashMap<B256, u64>,
}

impl MemoryBlockStore {
    pub fn new() -> Self {
        Self {
            blocks_by_height: HashMap::new(),
            height_by_hash: HashMap::new(),
        }
    }
}

impl Default for MemoryBlockStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockStore for MemoryBlockStore {
    type Error = std::convert::Infallible;

    fn insert(&mut self, block: SolizoneBlock) -> Result<(), Self::Error> {
        let height = block.header.height;
        let hash = block.hash();

        /*
         * If a block already exists at this height,
         * remove its old hash index.
         */
        if let Some(previous_block) = self.blocks_by_height.get(&height) {
            let previous_hash = previous_block.hash();

            self.height_by_hash.remove(&previous_hash);
        }

        self.height_by_hash.insert(hash, height);

        self.blocks_by_height.insert(height, block);

        Ok(())
    }

    fn get_by_height(&self, height: u64) -> Result<Option<SolizoneBlock>, Self::Error> {
        Ok(self.blocks_by_height.get(&height).cloned())
    }

    fn get_by_hash(&self, hash: B256) -> Result<Option<SolizoneBlock>, Self::Error> {
        let Some(height) = self.height_by_hash.get(&hash) else {
            return Ok(None);
        };

        Ok(self.blocks_by_height.get(height).cloned())
    }

    fn len(&self) -> Result<usize, Self::Error> {
        Ok(self.blocks_by_height.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use revm::primitives::B256;

    use crate::block::SolizoneBlockHeader;

    #[test]
    fn stores_and_retrieves_block_by_height_and_hash() {
        let mut store = MemoryBlockStore::new();

        assert!(store.is_empty().unwrap());
        assert_eq!(store.len().unwrap(), 0);

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

        store.insert(block).unwrap();

        assert!(!store.is_empty().unwrap());
        assert_eq!(store.len().unwrap(), 1);

        /*
         * Lookup by block height.
         */

        let by_height = store
            .get_by_height(7)
            .unwrap()
            .expect("block missing by height");

        assert_eq!(by_height.header.height, 7,);

        assert_eq!(by_height.hash(), block_hash,);

        /*
         * Lookup by block hash.
         */

        let by_hash = store
            .get_by_hash(block_hash)
            .unwrap()
            .expect("block missing by hash");

        assert_eq!(by_hash.header.height, 7,);

        assert_eq!(by_hash.hash(), block_hash,);

        /*
         * Unknown blocks should return None.
         */

        assert!(store.get_by_height(999).unwrap().is_none());

        assert!(store.get_by_hash(B256::from([0xff; 32])).unwrap().is_none());

        println!("Stored block height: {}", by_height.header.height);

        println!("Stored block hash:   {}", block_hash);

        println!("Block store size:    {}", store.len().unwrap());
    }

    #[test]
    fn replacing_block_removes_old_hash_index() {
        let mut store = MemoryBlockStore::new();

        /*
         * First block at height 7.
         */

        let block_a = SolizoneBlock {
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

        let old_hash = block_a.hash();

        store.insert(block_a).unwrap();

        assert_eq!(store.len().unwrap(), 1,);

        assert!(store.get_by_hash(old_hash).unwrap().is_some());

        /*
         * Different block at the same height.
         */

        let block_b = SolizoneBlock {
            header: SolizoneBlockHeader {
                version: 1,
                chain_id: 9001,
                height: 7,
                parent_hash: B256::from([0x11; 32]),
                timestamp: 1_800_000_001,
                state_root: B256::from([0x55; 32]),
                transactions_root: B256::from([0x66; 32]),
                receipts_root: B256::from([0x77; 32]),
                gas_limit: 30_000_000,
                gas_used: 0,
            },
            transactions: vec![],
        };

        let new_hash = block_b.hash();

        assert_ne!(old_hash, new_hash,);

        store.insert(block_b).unwrap();

        /*
         * Still only one canonical block
         * at height 7.
         */

        assert_eq!(store.len().unwrap(), 1,);

        /*
         * Old hash must no longer resolve.
         */

        assert!(
            store.get_by_hash(old_hash).unwrap().is_none(),
            "old block hash should have been removed"
        );

        /*
         * New block must resolve through
         * both indexes.
         */

        let by_height = store
            .get_by_height(7)
            .unwrap()
            .expect("replacement block missing by height");

        assert_eq!(by_height.hash(), new_hash,);

        let by_hash = store
            .get_by_hash(new_hash)
            .unwrap()
            .expect("replacement block missing by hash");

        assert_eq!(by_hash.header.height, 7,);

        assert_eq!(by_hash.hash(), new_hash,);

        println!("Old hash removed: {}", old_hash);

        println!("New canonical hash: {}", new_hash);

        println!("Store size: {}", store.len().unwrap());
    }
}
