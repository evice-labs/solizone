//! Solizone state commitment logic.
//!
//! Snapshot commitments provide a deterministic fingerprint of a
//! canonical persisted EVM state snapshot.
//!
//! This is intentionally separate from the existing block state-root
//! implementation in `execution::state_root`.

use revm::primitives::{B256, keccak256};

use super::StateSnapshot;

/// Computes a deterministic commitment for a canonical state snapshot.
///
/// `MemoryState::snapshot()` guarantees canonical account and storage
/// ordering. We therefore hash the deterministic serialized snapshot.
pub fn snapshot_commitment(snapshot: &StateSnapshot) -> Result<B256, String> {
    let encoded = snapshot.encode_json()?;

    Ok(keccak256(encoded.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use revm::primitives::{U256, address};

    use crate::state::{AccountSnapshot, StorageSlotSnapshot};

    fn snapshot_with_value(value: U256) -> StateSnapshot {
        StateSnapshot {
            accounts: vec![AccountSnapshot {
                address: address!("1111111111111111111111111111111111111111"),
                balance: U256::from(100),
                nonce: 1,
                code: None,
                storage: vec![StorageSlotSnapshot {
                    key: U256::ZERO,
                    value,
                }],
            }],
        }
    }

    #[test]
    fn same_snapshot_produces_same_commitment() {
        let snapshot = snapshot_with_value(U256::from(10));

        let commitment_a = snapshot_commitment(&snapshot).expect("failed to compute commitment A");

        let commitment_b = snapshot_commitment(&snapshot).expect("failed to compute commitment B");

        assert_eq!(
            commitment_a, commitment_b,
            "same snapshot produced different commitments"
        );

        println!("Snapshot commitment: {}", commitment_a);
    }

    #[test]
    fn changed_state_produces_different_commitment() {
        let snapshot_a = snapshot_with_value(U256::from(10));

        let snapshot_b = snapshot_with_value(U256::from(11));

        let commitment_a =
            snapshot_commitment(&snapshot_a).expect("failed to compute commitment A");

        let commitment_b =
            snapshot_commitment(&snapshot_b).expect("failed to compute commitment B");

        assert_ne!(
            commitment_a, commitment_b,
            "different state produced the same commitment"
        );

        println!("Commitment before state change: {}", commitment_a);

        println!("Commitment after state change:  {}", commitment_b);
    }
}
