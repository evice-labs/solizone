use revm::primitives::{Address, B256, Bytes, U256, keccak256};

const TX_MAGIC: &[u8; 4] = b"SZT1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionKind {
    Call(Address),
    Create,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolizoneTransaction {
    pub sender: Address,
    pub nonce: u64,
    pub kind: TransactionKind,
    pub value: U256,
    pub data: Bytes,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionDecodeError {
    UnexpectedEof,
    InvalidMagic,
    InvalidKind(u8),
    TrailingBytes,
}

fn take<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], TransactionDecodeError> {
    if input.len() < len {
        return Err(TransactionDecodeError::UnexpectedEof);
    }

    let (head, tail) = input.split_at(len);
    *input = tail;

    Ok(head)
}

impl SolizoneTransaction {
    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(TX_MAGIC);

        bytes.extend_from_slice(self.sender.as_slice());

        bytes.extend_from_slice(&self.nonce.to_be_bytes());

        match self.kind {
            TransactionKind::Create => {
                bytes.push(0);
            }

            TransactionKind::Call(address) => {
                bytes.push(1);

                bytes.extend_from_slice(address.as_slice());
            }
        }

        bytes.extend_from_slice(&self.value.to_be_bytes::<32>());

        bytes.extend_from_slice(&self.gas_limit.to_be_bytes());

        let data_len = u32::try_from(self.data.len()).expect("transaction data too large");

        bytes.extend_from_slice(&data_len.to_be_bytes());

        bytes.extend_from_slice(self.data.as_ref());

        bytes
    }

    pub fn revm_kind(&self) -> revm::primitives::TxKind {
        match self.kind {
            TransactionKind::Create => revm::primitives::TxKind::Create,

            TransactionKind::Call(address) => revm::primitives::TxKind::Call(address),
        }
    }

    pub fn hash(&self) -> B256 {
        keccak256(self.encode_canonical())
    }

    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, TransactionDecodeError> {
        let mut input = bytes;

        let magic = take(&mut input, 4)?;

        if magic != TX_MAGIC {
            return Err(TransactionDecodeError::InvalidMagic);
        }

        let sender = Address::from_slice(take(&mut input, 20)?);

        let nonce = u64::from_be_bytes(take(&mut input, 8)?.try_into().unwrap());

        let kind_tag = take(&mut input, 1)?[0];

        let kind = match kind_tag {
            0 => TransactionKind::Create,

            1 => {
                let address = Address::from_slice(take(&mut input, 20)?);

                TransactionKind::Call(address)
            }

            other => {
                return Err(TransactionDecodeError::InvalidKind(other));
            }
        };

        let value = U256::from_be_slice(take(&mut input, 32)?);

        let gas_limit = u64::from_be_bytes(take(&mut input, 8)?.try_into().unwrap());

        let data_len = u32::from_be_bytes(take(&mut input, 4)?.try_into().unwrap()) as usize;

        let data = Bytes::copy_from_slice(take(&mut input, data_len)?);

        if !input.is_empty() {
            return Err(TransactionDecodeError::TrailingBytes);
        }

        Ok(Self {
            sender,
            nonce,
            kind,
            value,
            data,
            gas_limit,
        })
    }
}
