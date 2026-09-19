use revm::primitives::{B256, keccak256};

use crate::execution::transaction::SolizoneTransaction;

const NODE_DOMAIN: &[u8] = b"SOLIZONE_TRANSACTION_NODE_V1";

const EMPTY_DOMAIN: &[u8] = b"SOLIZONE_TRANSACTIONS_EMPTY_V1";

const ROOT_DOMAIN: &[u8] = b"SOLIZONE_TRANSACTIONS_ROOT_V1";

fn hash_pair(left: B256, right: B256) -> B256 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(NODE_DOMAIN);
    bytes.extend_from_slice(left.as_slice());
    bytes.extend_from_slice(right.as_slice());

    keccak256(bytes)
}

fn finalize_root(tree_root: B256, transaction_count: usize) -> B256 {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(ROOT_DOMAIN);

    bytes.extend_from_slice(&(transaction_count as u32).to_be_bytes());

    bytes.extend_from_slice(tree_root.as_slice());

    keccak256(bytes)
}

pub fn compute_transactions_root(transactions: &[SolizoneTransaction]) -> B256 {
    if transactions.is_empty() {
        return finalize_root(keccak256(EMPTY_DOMAIN), 0);
    }

    // Important: preserve block transaction order.
    let mut level: Vec<B256> = transactions.iter().map(|tx| tx.hash()).collect();

    while level.len() > 1 {
        let mut next = Vec::new();

        for pair in level.chunks(2) {
            let left = pair[0];

            let right = if pair.len() == 2 { pair[1] } else { pair[0] };

            next.push(hash_pair(left, right));
        }

        level = next;
    }

    finalize_root(level[0], transactions.len())
}
