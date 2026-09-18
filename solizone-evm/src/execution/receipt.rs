use revm::{
    context::result::ExecutionResult,
    primitives::{Address, B256, Bytes, keccak256},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    Success,
    Revert,
    Halt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionLog {
    /// Contract that emitted the log.
    pub address: Address,

    /// Event signature and indexed parameters.
    pub topics: Vec<B256>,

    /// ABI-encoded non-indexed event data.
    pub data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReceipt {
    /// High-level result of execution.
    pub status: ExecutionStatus,

    /// EVM transaction gas consumed.
    pub gas_used: u64,

    /// Return or revert data produced by execution.
    pub output: Bytes,

    /// Address created by a CREATE transaction.
    ///
    /// None for normal calls and transfers.
    pub contract_address: Option<Address>,

    pub logs: Vec<ExecutionLog>,
}

const RECEIPT_MAGIC: &[u8; 4] = b"SZR1";

impl ExecutionStatus {
    fn as_byte(&self) -> u8 {
        match self {
            ExecutionStatus::Success => 0,
            ExecutionStatus::Revert => 1,
            ExecutionStatus::Halt => 2,
        }
    }
}

impl ExecutionReceipt {
    pub fn from_revm(result: &ExecutionResult) -> Self {
        let status = match result {
            ExecutionResult::Success { .. } => ExecutionStatus::Success,
            ExecutionResult::Revert { .. } => ExecutionStatus::Revert,
            ExecutionResult::Halt { .. } => ExecutionStatus::Halt,
        };

        let logs = result
            .logs()
            .iter()
            .map(|log| ExecutionLog {
                address: log.address,
                topics: log.data.topics().to_vec(),
                data: log.data.data.clone(),
            })
            .collect();

        Self {
            status,
            gas_used: result.tx_gas_used(),
            output: result.output().cloned().unwrap_or_default(),
            contract_address: result.created_address(),
            logs,
        }
    }

    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Solizone Receipt V1
        bytes.extend_from_slice(RECEIPT_MAGIC);

        // Status
        bytes.push(self.status.as_byte());

        // Gas
        bytes.extend_from_slice(&self.gas_used.to_be_bytes());

        // Optional created contract address
        match self.contract_address {
            Some(address) => {
                bytes.push(1);
                bytes.extend_from_slice(address.as_slice());
            }
            None => {
                bytes.push(0);
            }
        }

        // Output
        let output_len = u32::try_from(self.output.len()).expect("receipt output too large");

        bytes.extend_from_slice(&output_len.to_be_bytes());
        bytes.extend_from_slice(self.output.as_ref());

        // Logs
        let log_count = u32::try_from(self.logs.len()).expect("too many receipt logs");

        bytes.extend_from_slice(&log_count.to_be_bytes());

        for log in &self.logs {
            bytes.extend_from_slice(log.address.as_slice());

            let topic_count = u8::try_from(log.topics.len()).expect("too many log topics");

            bytes.push(topic_count);

            for topic in &log.topics {
                bytes.extend_from_slice(topic.as_slice());
            }

            let data_len = u32::try_from(log.data.len()).expect("log data too large");

            bytes.extend_from_slice(&data_len.to_be_bytes());
            bytes.extend_from_slice(log.data.as_ref());
        }

        bytes
    }

    pub fn hash(&self) -> B256 {
        keccak256(self.encode_canonical())
    }
}
