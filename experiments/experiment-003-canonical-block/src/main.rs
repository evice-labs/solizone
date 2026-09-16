use sha3::{Digest, Keccak256};
use std::fs;

type Hash32 = [u8; 32];

const HEADER_SIZE: usize = 170;
const BLOCK_MAGIC: &[u8; 4] = b"SZB1";

#[derive(Debug, Clone, PartialEq, Eq)]
struct SolizoneBlockHeader {
    version: u16,
    chain_id: u64,
    height: u64,
    parent_hash: Hash32,
    timestamp: u64,
    state_root: Hash32,
    transactions_root: Hash32,
    receipts_root: Hash32,
    gas_limit: u64,
    gas_used: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SolizoneBlock {
    header: SolizoneBlockHeader,
    transactions: Vec<Vec<u8>>,
}

impl SolizoneBlockHeader {
    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_SIZE);

        bytes.extend_from_slice(&self.version.to_be_bytes());
        bytes.extend_from_slice(&self.chain_id.to_be_bytes());
        bytes.extend_from_slice(&self.height.to_be_bytes());
        bytes.extend_from_slice(&self.parent_hash);
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());
        bytes.extend_from_slice(&self.state_root);
        bytes.extend_from_slice(&self.transactions_root);
        bytes.extend_from_slice(&self.receipts_root);
        bytes.extend_from_slice(&self.gas_limit.to_be_bytes());
        bytes.extend_from_slice(&self.gas_used.to_be_bytes());

        bytes
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != HEADER_SIZE {
            return Err(format!(
                "invalid header length: expected {}, got {}",
                HEADER_SIZE,
                bytes.len()
            ));
        }

        Ok(Self {
            version: u16::from_be_bytes(bytes[0..2].try_into().map_err(|_| "invalid version")?),

            chain_id: u64::from_be_bytes(bytes[2..10].try_into().map_err(|_| "invalid chain_id")?),

            height: u64::from_be_bytes(bytes[10..18].try_into().map_err(|_| "invalid height")?),

            parent_hash: bytes[18..50]
                .try_into()
                .map_err(|_| "invalid parent_hash")?,

            timestamp: u64::from_be_bytes(
                bytes[50..58].try_into().map_err(|_| "invalid timestamp")?,
            ),

            state_root: bytes[58..90].try_into().map_err(|_| "invalid state_root")?,

            transactions_root: bytes[90..122]
                .try_into()
                .map_err(|_| "invalid transactions_root")?,

            receipts_root: bytes[122..154]
                .try_into()
                .map_err(|_| "invalid receipts_root")?,

            gas_limit: u64::from_be_bytes(
                bytes[154..162]
                    .try_into()
                    .map_err(|_| "invalid gas_limit")?,
            ),

            gas_used: u64::from_be_bytes(
                bytes[162..170].try_into().map_err(|_| "invalid gas_used")?,
            ),
        })
    }

    fn hash(&self) -> Hash32 {
        keccak256(&self.encode())
    }
}

impl SolizoneBlock {
    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Identify the payload as a Solizone canonical block v1.
        bytes.extend_from_slice(BLOCK_MAGIC);

        // Canonical 170-byte block header.
        bytes.extend_from_slice(&self.header.encode());

        // Number of transactions.
        bytes.extend_from_slice(&(self.transactions.len() as u32).to_be_bytes());

        // Each transaction is length-prefixed.
        for tx in &self.transactions {
            bytes.extend_from_slice(&(tx.len() as u32).to_be_bytes());
            bytes.extend_from_slice(tx);
        }

        bytes
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let minimum_size = 4 + HEADER_SIZE + 4;

        if bytes.len() < minimum_size {
            return Err("block payload too short".to_string());
        }

        if &bytes[0..4] != BLOCK_MAGIC {
            return Err("invalid Solizone block magic".to_string());
        }

        let header_start = 4;
        let header_end = header_start + HEADER_SIZE;

        let header = SolizoneBlockHeader::decode(&bytes[header_start..header_end])?;

        let tx_count_start = header_end;
        let tx_count_end = tx_count_start + 4;

        let tx_count = u32::from_be_bytes(
            bytes[tx_count_start..tx_count_end]
                .try_into()
                .map_err(|_| "invalid transaction count")?,
        ) as usize;

        let mut cursor = tx_count_end;
        let mut transactions = Vec::with_capacity(tx_count);

        for _ in 0..tx_count {
            if cursor + 4 > bytes.len() {
                return Err("missing transaction length".to_string());
            }

            let tx_len = u32::from_be_bytes(
                bytes[cursor..cursor + 4]
                    .try_into()
                    .map_err(|_| "invalid transaction length")?,
            ) as usize;

            cursor += 4;

            if cursor + tx_len > bytes.len() {
                return Err("transaction exceeds block payload".to_string());
            }

            transactions.push(bytes[cursor..cursor + tx_len].to_vec());

            cursor += tx_len;
        }

        if cursor != bytes.len() {
            return Err("unexpected trailing block bytes".to_string());
        }

        let block = Self {
            header,
            transactions,
        };

        block.validate_transactions_root()?;

        Ok(block)
    }

    fn validate_transactions_root(&self) -> Result<(), String> {
        let calculated = compute_transactions_root(&self.transactions);

        if calculated != self.header.transactions_root {
            return Err("transactions_root does not match block body".to_string());
        }

        Ok(())
    }
}

fn keccak256(data: &[u8]) -> Hash32 {
    let digest = Keccak256::digest(data);

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);

    hash
}

fn compute_transactions_root(transactions: &[Vec<u8>]) -> Hash32 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"SOLIZONE_TX_ROOT_V1");

    bytes.extend_from_slice(&(transactions.len() as u32).to_be_bytes());

    for tx in transactions {
        bytes.extend_from_slice(&(tx.len() as u32).to_be_bytes());
        bytes.extend_from_slice(tx);
    }

    keccak256(&bytes)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn main() {
    println!("=== Solizone Experiment 003 ===");
    println!("Canonical full-block encoding\n");

    let transactions = vec![
        b"alice -> bob : 10".to_vec(),
        b"bob -> charlie : 4".to_vec(),
        b"charlie -> alice : 1".to_vec(),
    ];

    let transactions_root = compute_transactions_root(&transactions);

    let block = SolizoneBlock {
        header: SolizoneBlockHeader {
            version: 1,
            chain_id: 9001,
            height: 0,
            parent_hash: [0u8; 32],
            timestamp: 1_800_000_000,
            state_root: [1u8; 32],
            transactions_root,
            receipts_root: [3u8; 32],
            gas_limit: 30_000_000,
            gas_used: 21_000,
        },

        transactions,
    };

    let encoded = block.encode();

    fs::create_dir_all(".output").expect("failed to create output directory");

    fs::write(".output/block-0.szb", &encoded).expect("failed to write canonical block");

    println!("Saved canonical block: .output/block-0.szb");

    println!("Block height: {}", block.header.height);
    println!("Transactions: {}", block.transactions.len());
    println!("Header size: {} bytes", HEADER_SIZE);
    println!("Full encoded block: {} bytes", encoded.len());

    println!("\nBlock hash:");
    println!("0x{}", to_hex(&block.header.hash()));

    let decoded = SolizoneBlock::decode(&encoded).expect("canonical block should decode");

    assert_eq!(block, decoded);
    assert_eq!(block.header.hash(), decoded.header.hash());

    println!("\nDecoded transactions:");

    for (index, tx) in decoded.transactions.iter().enumerate() {
        println!("  tx {}: {}", index, String::from_utf8_lossy(tx));
    }

    println!("\n✅ Full block encoded");
    println!("✅ Full block decoded");
    println!("✅ Transaction commitment verified");
    println!("✅ Decoded block equals original block");
    println!("✅ Block hash preserved");

    println!("\n=== Tamper test ===");

    let mut tampered = encoded.clone();

    // Layout:
    // 4 bytes magic
    // 170 bytes header
    // 4 bytes transaction count
    // 4 bytes first transaction length
    // then first transaction data
    let first_tx_data_offset = 4 + HEADER_SIZE + 4 + 4;

    // Change one byte inside the first transaction.
    tampered[first_tx_data_offset] ^= 0x01;

    match SolizoneBlock::decode(&tampered) {
        Ok(_) => {
            println!("❌ Tampered block was incorrectly accepted");
        }

        Err(err) => {
            println!("✅ Tampered block rejected");
            println!("Reason: {}", err);
        }
    }
}
