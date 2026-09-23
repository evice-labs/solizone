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

            let storage = account
                .storage
                .iter()
                .map(|(key, value)| StorageSlotSnapshot {
                    key: *key,
                    value: *value,
                })
                .collect();

            accounts.push(AccountSnapshot {
                address: *address,
                balance: account.info.balance,
                nonce: account.info.nonce,
                code,
                storage,
            });
        }

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
