use revm::primitives::B256;
use serde::{Deserialize, Serialize};

use super::StateSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolizoneCheckpoint {
    pub state: StateSnapshot,
    pub next_height: u64,
    pub parent_hash: B256,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedCheckpoint {
    state: serde_json::Value,
    next_height: u64,
    parent_hash: String,
}

impl SolizoneCheckpoint {
    pub fn new(state: StateSnapshot, next_height: u64, parent_hash: B256) -> Self {
        Self {
            state,
            next_height,
            parent_hash,
        }
    }

    pub fn encode_json(&self) -> Result<String, String> {
        let state_json = self.state.encode_json()?;

        let state = serde_json::from_str(&state_json).map_err(|error| error.to_string())?;

        let persisted = PersistedCheckpoint {
            state,
            next_height: self.next_height,
            parent_hash: format!("{:#x}", self.parent_hash),
        };

        serde_json::to_string_pretty(&persisted).map_err(|error| error.to_string())
    }

    pub fn decode_json(json: &str) -> Result<Self, String> {
        let persisted: PersistedCheckpoint =
            serde_json::from_str(json).map_err(|error| error.to_string())?;

        let state_json =
            serde_json::to_string(&persisted.state).map_err(|error| error.to_string())?;

        let state = StateSnapshot::decode_json(&state_json)?;

        let parent_hash = persisted
            .parent_hash
            .parse::<B256>()
            .map_err(|error| error.to_string())?;

        Ok(Self {
            state,
            next_height: persisted.next_height,
            parent_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use revm::primitives::{U256, address};

    use crate::state::{AccountSnapshot, StorageSlotSnapshot};

    #[test]
    fn checkpoint_json_round_trip() {
        let state = StateSnapshot {
            accounts: vec![AccountSnapshot {
                address: address!("1111111111111111111111111111111111111111"),
                balance: U256::from(1000),
                nonce: 2,
                code: None,
                storage: vec![StorageSlotSnapshot {
                    key: U256::ZERO,
                    value: U256::from(42),
                }],
            }],
        };

        let checkpoint = SolizoneCheckpoint::new(state, 3, B256::from([0x22; 32]));

        let json = checkpoint.encode_json().expect("checkpoint should encode");

        let decoded = SolizoneCheckpoint::decode_json(&json).expect("checkpoint should decode");

        assert_eq!(decoded, checkpoint,);

        assert_eq!(decoded.next_height, 3,);

        assert_eq!(decoded.parent_hash, B256::from([0x22; 32]),);

        println!("Checkpoint JSON:\n{}", json);
    }
}
