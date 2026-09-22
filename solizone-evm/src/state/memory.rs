use revm::{database::InMemoryDB, primitives::Address, state::AccountInfo};

/// In-memory Solizone EVM state.
///
/// For now this wraps REVM's InMemoryDB.
/// Later we will add a persistent StateBackend without forcing
/// the execution engine to know how persistence works.
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
}

impl Default for MemoryState {
    fn default() -> Self {
        Self::new()
    }
}
