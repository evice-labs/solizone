use revm::{
    database::InMemoryDB,
    primitives::{Address, Bytes},
    state::AccountInfo,
};

use crate::state::snapshot::{AccountSnapshot, StateSnapshot, StorageSlotSnapshot};
use revm_bytecode::Bytecode;

pub struct MemoryState {
    db: InMemoryDB,
}

impl MemoryState {
    pub fn new() -> Self {
        Self {
            db: InMemoryDB::default(),
        }
    }

    pub fn from_db(db: InMemoryDB) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &InMemoryDB {
        &self.db
    }

    pub fn db_mut(&mut self) -> &mut InMemoryDB {
        &mut self.db
    }

    pub fn into_db(self) -> InMemoryDB {
        self.db
    }

    pub fn insert_account_info(&mut self, address: Address, account_info: AccountInfo) {
        self.db.insert_account_info(address, account_info);
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let mut accounts = Vec::new();

        for (address, account) in &self.db.cache.accounts {
            let code = account
                .info
                .code
                .as_ref()
                .map(|code| Bytes::copy_from_slice(code.original_bytes().as_ref()));

            let mut storage: Vec<StorageSlotSnapshot> = account
                .storage
                .iter()
                .map(|(key, value)| StorageSlotSnapshot {
                    key: *key,
                    value: *value,
                })
                .collect();

            // Canonical storage ordering:
            // lowest storage slot first.
            storage.sort_by(|a, b| a.key.cmp(&b.key));

            accounts.push(AccountSnapshot {
                address: *address,
                balance: account.info.balance,
                nonce: account.info.nonce,
                code,
                storage,
            });
        }

        // Canonical account ordering:
        // lowest address first.
        accounts.sort_by(|a, b| a.address.cmp(&b.address));

        StateSnapshot { accounts }
    }

    pub fn from_snapshot(snapshot: StateSnapshot) -> Self {
        let mut state = Self::new();

        for account in snapshot.accounts {
            let code = account.code.map(Bytecode::new_raw);

            let account_info = AccountInfo {
                balance: account.balance,
                nonce: account.nonce,
                code_hash: code
                    .as_ref()
                    .map(|code| code.hash_slow())
                    .unwrap_or_default(),
                account_id: None,
                code,
            };

            state.db.insert_account_info(account.address, account_info);

            for slot in account.storage {
                state
                    .db
                    .insert_account_storage(account.address, slot.key, slot.value)
                    .expect("failed to restore account storage");
            }
        }

        state
    }
}

impl Default for MemoryState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revm::primitives::{U256, address};
    use revm::state::AccountInfo;

    #[test]
    fn snapshot_is_deterministic_across_insertion_order() {
        use crate::state::commitment::snapshot_commitment;

        let address_a = address!("1111111111111111111111111111111111111111");

        let address_b = address!("2222222222222222222222222222222222222222");

        // State A:
        // accounts A → B
        // storage slots 0 → 5
        let mut state_a = MemoryState::new();

        state_a.insert_account_info(address_a, AccountInfo::from_balance(U256::from(100)));

        state_a.insert_account_info(address_b, AccountInfo::from_balance(U256::from(200)));

        state_a
            .db_mut()
            .insert_account_storage(address_a, U256::ZERO, U256::from(10))
            .expect("failed to insert storage");

        state_a
            .db_mut()
            .insert_account_storage(address_a, U256::from(5), U256::from(50))
            .expect("failed to insert storage");

        // State B:
        // accounts B → A
        // storage slots 5 → 0
        let mut state_b = MemoryState::new();

        state_b.insert_account_info(address_b, AccountInfo::from_balance(U256::from(200)));

        state_b.insert_account_info(address_a, AccountInfo::from_balance(U256::from(100)));

        state_b
            .db_mut()
            .insert_account_storage(address_a, U256::from(5), U256::from(50))
            .expect("failed to insert storage");

        state_b
            .db_mut()
            .insert_account_storage(address_a, U256::ZERO, U256::from(10))
            .expect("failed to insert storage");

        let snapshot_a = state_a.snapshot();
        let snapshot_b = state_b.snapshot();

        // First prove the logical snapshots are identical.
        assert_eq!(snapshot_a, snapshot_b);

        // Then prove their persisted representation is
        // byte-for-byte identical.
        let encoded_a = snapshot_a
            .encode_json()
            .expect("failed to encode snapshot A");

        let encoded_b = snapshot_b
            .encode_json()
            .expect("failed to encode snapshot B");

        assert_eq!(
            encoded_a, encoded_b,
            "canonical snapshots produced different bytes"
        );

        let commitment_a =
            snapshot_commitment(&snapshot_a).expect("failed to compute commitment A");

        let commitment_b =
            snapshot_commitment(&snapshot_b).expect("failed to compute commitment B");

        assert_eq!(
            commitment_a, commitment_b,
            "equivalent states produced different commitments"
        );

        println!("Canonical snapshot commitment: {}", commitment_a);

        println!("Canonical snapshot determinism verified.");
        println!("Encoded snapshot size: {} bytes", encoded_a.len());
    }
}
