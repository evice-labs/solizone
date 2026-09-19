use revm::{
    database::InMemoryDB,
    primitives::{B256, KECCAK_EMPTY, keccak256},
};

const STATE_DOMAIN: &[u8] = b"SOLIZONE_STATE_V1";
const ACCOUNT_DOMAIN: &[u8] = b"SOLIZONE_ACCOUNT_V1";
const STORAGE_DOMAIN: &[u8] = b"SOLIZONE_STORAGE_V1";

pub fn compute_state_root(db: &InMemoryDB) -> B256 {
    let mut accounts: Vec<_> = db.cache.accounts.iter().collect();

    // Consensus-critical: deterministic ordering.
    accounts.sort_by_key(|(address, _)| **address);

    let mut account_hashes = Vec::new();

    for (address, account) in accounts {
        let Some(info) = account.info() else {
            continue;
        };

        // Ignore completely empty accounts.
        let has_nonzero_storage = account.storage.values().any(|value| !value.is_zero());

        let is_empty = info.balance.is_zero()
            && info.nonce == 0
            && info.code_hash == KECCAK_EMPTY
            && !has_nonzero_storage;

        if is_empty {
            continue;
        }

        /*
         * Storage commitment
         */
        let mut storage: Vec<_> = account
            .storage
            .iter()
            .filter(|(_, value)| !value.is_zero())
            .collect();

        storage.sort_by_key(|(slot, _)| **slot);

        let mut storage_bytes = Vec::new();

        storage_bytes.extend_from_slice(STORAGE_DOMAIN);

        storage_bytes.extend_from_slice(&(storage.len() as u32).to_be_bytes());

        for (slot, value) in storage {
            storage_bytes.extend_from_slice(&slot.to_be_bytes::<32>());

            storage_bytes.extend_from_slice(&value.to_be_bytes::<32>());
        }

        let storage_root = keccak256(storage_bytes);

        /*
         * Account commitment
         */
        let mut account_bytes = Vec::new();

        account_bytes.extend_from_slice(ACCOUNT_DOMAIN);

        account_bytes.extend_from_slice(address.as_slice());

        account_bytes.extend_from_slice(&info.balance.to_be_bytes::<32>());

        account_bytes.extend_from_slice(&info.nonce.to_be_bytes());

        account_bytes.extend_from_slice(info.code_hash.as_slice());

        account_bytes.extend_from_slice(storage_root.as_slice());

        account_hashes.push(keccak256(account_bytes));
    }

    //Final state commitment

    let mut state_bytes = Vec::new();

    state_bytes.extend_from_slice(STATE_DOMAIN);

    state_bytes.extend_from_slice(&(account_hashes.len() as u32).to_be_bytes());

    for account_hash in account_hashes {
        state_bytes.extend_from_slice(account_hash.as_slice());
    }

    keccak256(state_bytes)
}
