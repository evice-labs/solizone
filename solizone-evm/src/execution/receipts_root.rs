use revm::primitives::{B256, keccak256};

use super::receipt::ExecutionReceipt;

#[derive(Debug, Clone)]
pub struct ReceiptMerkleProof {
    pub receipt_index: usize,
    pub receipt_count: usize,
    pub siblings: Vec<B256>,
}

const NODE_DOMAIN: &[u8] = b"SOLIZONE_RECEIPT_NODE_V1";
const EMPTY_DOMAIN: &[u8] = b"SOLIZONE_RECEIPTS_EMPTY_V1";
const ROOT_DOMAIN: &[u8] = b"SOLIZONE_RECEIPTS_ROOT_V1";

fn hash_pair(left: B256, right: B256) -> B256 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(NODE_DOMAIN);
    bytes.extend_from_slice(left.as_slice());
    bytes.extend_from_slice(right.as_slice());

    keccak256(bytes)
}

fn finalize_root(tree_root: B256, receipt_count: usize) -> B256 {
    let count = u32::try_from(receipt_count).expect("too many receipts");

    let mut bytes = Vec::new();

    bytes.extend_from_slice(ROOT_DOMAIN);
    bytes.extend_from_slice(&count.to_be_bytes());
    bytes.extend_from_slice(tree_root.as_slice());

    keccak256(bytes)
}

pub fn compute_receipts_root(receipts: &[ExecutionReceipt]) -> B256 {
    if receipts.is_empty() {
        let empty_tree = keccak256(EMPTY_DOMAIN);
        return finalize_root(empty_tree, 0);
    }

    let mut level: Vec<B256> = receipts.iter().map(ExecutionReceipt::hash).collect();

    while level.len() > 1 {
        let mut next_level = Vec::new();

        for pair in level.chunks(2) {
            let left = pair[0];

            let right = if pair.len() == 2 { pair[1] } else { pair[0] };

            next_level.push(hash_pair(left, right));
        }

        level = next_level;
    }

    finalize_root(level[0], receipts.len())
}

pub fn build_receipt_proof(
    receipts: &[ExecutionReceipt],
    receipt_index: usize,
) -> Option<ReceiptMerkleProof> {
    if receipt_index >= receipts.len() {
        return None;
    }

    let mut level: Vec<B256> = receipts.iter().map(ExecutionReceipt::hash).collect();

    let mut index = receipt_index;
    let mut siblings = Vec::new();

    while level.len() > 1 {
        let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };

        let sibling = if sibling_index < level.len() {
            level[sibling_index]
        } else {
            // Odd final node: duplicated, same rule as root construction.
            level[index]
        };

        siblings.push(sibling);

        let mut next_level = Vec::new();

        for pair in level.chunks(2) {
            let left = pair[0];

            let right = if pair.len() == 2 { pair[1] } else { pair[0] };

            next_level.push(hash_pair(left, right));
        }

        index /= 2;
        level = next_level;
    }

    Some(ReceiptMerkleProof {
        receipt_index,
        receipt_count: receipts.len(),
        siblings,
    })
}

pub fn verify_receipt_proof(
    receipt: &ExecutionReceipt,
    proof: &ReceiptMerkleProof,
    expected_root: B256,
) -> bool {
    if proof.receipt_count == 0 || proof.receipt_index >= proof.receipt_count {
        return false;
    }

    let mut hash = receipt.hash();
    let mut index = proof.receipt_index;

    for sibling in &proof.siblings {
        hash = if index % 2 == 0 {
            hash_pair(hash, *sibling)
        } else {
            hash_pair(*sibling, hash)
        };

        index /= 2;
    }

    let computed_root = finalize_root(hash, proof.receipt_count);

    computed_root == expected_root
}
