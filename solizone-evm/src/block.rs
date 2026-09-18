use revm::primitives::{B256, keccak256};

pub const HEADER_SIZE: usize = 170;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolizoneBlockHeader {
    pub version: u16,
    pub chain_id: u64,
    pub height: u64,
    pub parent_hash: B256,
    pub timestamp: u64,
    pub state_root: B256,
    pub transactions_root: B256,
    pub receipts_root: B256,
    pub gas_limit: u64,
    pub gas_used: u64,
}

impl SolizoneBlockHeader {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_SIZE);

        bytes.extend_from_slice(&self.version.to_be_bytes());

        bytes.extend_from_slice(&self.chain_id.to_be_bytes());

        bytes.extend_from_slice(&self.height.to_be_bytes());

        bytes.extend_from_slice(self.parent_hash.as_slice());

        bytes.extend_from_slice(&self.timestamp.to_be_bytes());

        bytes.extend_from_slice(self.state_root.as_slice());

        bytes.extend_from_slice(self.transactions_root.as_slice());

        bytes.extend_from_slice(self.receipts_root.as_slice());

        bytes.extend_from_slice(&self.gas_limit.to_be_bytes());

        bytes.extend_from_slice(&self.gas_used.to_be_bytes());

        bytes
    }

    pub fn hash(&self) -> B256 {
        keccak256(self.encode())
    }
}
