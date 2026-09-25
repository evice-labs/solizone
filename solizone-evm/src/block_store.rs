use revm::primitives::B256;

use crate::block::SolizoneBlock;

pub trait BlockStore {
    type Error;

    fn insert(&mut self, block: SolizoneBlock) -> Result<(), Self::Error>;

    fn get_by_height(&self, height: u64) -> Result<Option<SolizoneBlock>, Self::Error>;

    fn get_by_hash(&self, hash: B256) -> Result<Option<SolizoneBlock>, Self::Error>;

    fn len(&self) -> Result<usize, Self::Error>;

    fn is_empty(&self) -> Result<bool, Self::Error> {
        Ok(self.len()? == 0)
    }
}
