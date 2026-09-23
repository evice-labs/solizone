use revm::primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSnapshot {
    pub address: Address,
    pub balance: U256,
    pub nonce: u64,
    pub code: Option<Bytes>,
    pub storage: Vec<StorageSlotSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSlotSnapshot {
    pub key: U256,
    pub value: U256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshot {
    pub accounts: Vec<AccountSnapshot>,
}

// Persistent representation

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedAccountSnapshot {
    address: String,
    balance: String,
    nonce: u64,
    code: Option<String>,
    storage: Vec<PersistedStorageSlotSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedStorageSlotSnapshot {
    key: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedStateSnapshot {
    accounts: Vec<PersistedAccountSnapshot>,
}

impl StateSnapshot {
    pub fn encode_json(&self) -> Result<String, String> {
        let persisted = PersistedStateSnapshot {
            accounts: self
                .accounts
                .iter()
                .map(|account| PersistedAccountSnapshot {
                    address: format!("{:#x}", account.address),
                    balance: format!("{:#x}", account.balance),
                    nonce: account.nonce,
                    code: account
                        .code
                        .as_ref()
                        .map(|code| format!("0x{}", hex::encode(code))),
                    storage: account
                        .storage
                        .iter()
                        .map(|slot| PersistedStorageSlotSnapshot {
                            key: format!("{:#x}", slot.key),
                            value: format!("{:#x}", slot.value),
                        })
                        .collect(),
                })
                .collect(),
        };

        serde_json::to_string_pretty(&persisted).map_err(|error| error.to_string())
    }

    pub fn decode_json(json: &str) -> Result<Self, String> {
        let persisted: PersistedStateSnapshot =
            serde_json::from_str(json).map_err(|error| error.to_string())?;

        let accounts = persisted
            .accounts
            .into_iter()
            .map(|account| {
                let address = account
                    .address
                    .parse::<Address>()
                    .map_err(|error| error.to_string())?;

                let balance = account
                    .balance
                    .parse::<U256>()
                    .map_err(|error| error.to_string())?;

                let code = account
                    .code
                    .map(|code| {
                        let hex_code = code.strip_prefix("0x").unwrap_or(&code);

                        hex::decode(hex_code)
                            .map(Bytes::from)
                            .map_err(|error| error.to_string())
                    })
                    .transpose()?;

                let storage = account
                    .storage
                    .into_iter()
                    .map(|slot| {
                        let key = slot
                            .key
                            .parse::<U256>()
                            .map_err(|error| error.to_string())?;

                        let value = slot
                            .value
                            .parse::<U256>()
                            .map_err(|error| error.to_string())?;

                        Ok(StorageSlotSnapshot { key, value })
                    })
                    .collect::<Result<Vec<_>, String>>()?;

                Ok(AccountSnapshot {
                    address,
                    balance,
                    nonce: account.nonce,
                    code,
                    storage,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(Self { accounts })
    }
}
