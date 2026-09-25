use super::receipt::ExecutionReceipt;

use revm::{
    Context, ExecuteCommitEvm, ExecuteEvm, MainBuilder, MainContext,
    context::TxEnv,
    database::InMemoryDB,
    primitives::{Address, B256, Bytes, TxKind, U256},
    state::AccountInfo,
};

use crate::execution::transaction::{SolizoneTransaction, TransactionKind};

use crate::execution::receipt::ExecutionStatus;

use crate::execution::receipts_root::{
    build_receipt_proof, compute_receipts_root, verify_receipt_proof,
};

use crate::state::MemoryState;

#[derive(Debug)]
pub struct TransferOutcome {
    pub sender_balance: U256,
    pub sender_nonce: u64,
    pub recipient_balance: U256,
}

#[derive(Debug)]
pub struct DeploymentOutcome {
    pub contract_address: Address,
    pub deployer_balance: U256,
    pub deployer_nonce: u64,
    pub contract_code_hash: B256,
    pub gas_used: u64,
}

#[derive(Debug)]
pub struct ContractCallOutcome {
    pub contract_address: Address,
    pub output: Bytes,
    pub deploy_gas_used: u64,
    pub call_gas_used: u64,
}

#[derive(Debug)]
pub struct SolidityCounterOutcome {
    pub contract_address: Address,
    pub count: U256,
    pub state_root: B256,
    pub transactions: Vec<SolizoneTransaction>,
    pub deploy_receipt: ExecutionReceipt,
    pub increment_receipt: ExecutionReceipt,
    pub read_receipt: ExecutionReceipt,
    pub revert_receipt: ExecutionReceipt,
    pub halt_receipt: ExecutionReceipt,
}

#[derive(Debug)]
pub struct BlockExecutionOutcome {
    pub receipts: Vec<ExecutionReceipt>,
    pub state_root: B256,
}

pub struct RevmExecutionEngine;

fn counter_creation_bytecode() -> Bytes {
    let hex_source = include_str!("../../contracts/Counter.creation.hex").trim();

    let hex_source = hex_source.strip_prefix("0x").unwrap_or(hex_source);

    let bytes = hex::decode(hex_source).expect("invalid Counter creation bytecode");

    Bytes::from(bytes)
}

impl RevmExecutionEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_transaction(
        &self,
        state: &mut MemoryState,
        tx: &SolizoneTransaction,
    ) -> ExecutionReceipt {
        let tx_env = TxEnv::builder()
            .caller(tx.sender)
            .kind(tx.revm_kind())
            .value(tx.value)
            .data(tx.data.clone())
            .gas_limit(tx.gas_limit)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(tx.nonce)
            .build()
            .expect("failed to build REVM transaction");

        let result = {
            let mut evm = Context::mainnet().with_db(state.db_mut()).build_mainnet();

            evm.transact_commit(tx_env).expect("EVM transaction failed")
        };

        ExecutionReceipt::from_revm(&result)
    }

    pub fn execute_call(
        &self,
        state: &mut MemoryState,
        tx: &SolizoneTransaction,
    ) -> ExecutionReceipt {
        let tx_env = TxEnv::builder()
            .caller(tx.sender)
            .kind(tx.revm_kind())
            .value(tx.value)
            .data(tx.data.clone())
            .gas_limit(tx.gas_limit)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(tx.nonce)
            .build()
            .expect("failed to build REVM call");

        let result = {
            let mut evm = Context::mainnet().with_db(state.db_mut()).build_mainnet();

            let output = evm.transact(tx_env).expect("EVM call failed");

            output.result
        };

        ExecutionReceipt::from_revm(&result)
    }

    pub fn execute_transfer(
        &self,
        state: &mut MemoryState,
        sender: Address,
        recipient: Address,
        value: U256,
    ) -> TransferOutcome {
        let sender_nonce = state
            .db()
            .cache
            .accounts
            .get(&sender)
            .and_then(|account| account.info())
            .map(|info| info.nonce)
            .unwrap_or(0);

        let tx = SolizoneTransaction {
            sender,
            nonce: sender_nonce,
            kind: TransactionKind::Call(recipient),
            value,
            data: Bytes::new(),
            gas_limit: 21_000,
        };

        let receipt = self.execute_transaction(state, &tx);

        println!("Execution receipt:");
        println!("{:#?}", receipt);
        println!();

        let sender_after = state
            .db()
            .cache
            .accounts
            .get(&sender)
            .and_then(|account| account.info())
            .expect("sender missing from resulting state");

        let recipient_after = state
            .db()
            .cache
            .accounts
            .get(&recipient)
            .and_then(|account| account.info())
            .expect("recipient missing from resulting state");

        TransferOutcome {
            sender_balance: sender_after.balance,
            sender_nonce: sender_after.nonce,
            recipient_balance: recipient_after.balance,
        }
    }

    pub fn deploy_test_contract(
        &self,
        deployer: Address,
        deployer_starting_balance: U256,
    ) -> DeploymentOutcome {
        let mut db = InMemoryDB::default();

        db.insert_account_info(
            deployer,
            AccountInfo::from_balance(deployer_starting_balance),
        );

        // Runtime bytecode:
        //
        // PUSH1 0x2a
        // PUSH1 0x00
        // MSTORE
        // PUSH1 0x20
        // PUSH1 0x00
        // RETURN
        //
        // Calling this contract will eventually return 42.
        //
        // The first 12 bytes are creation code that copies the
        // 10-byte runtime code into memory and returns it.
        let deployment_bytecode = Bytes::from_static(&[
            0x60, 0x0a, 0x60, 0x0c, 0x60, 0x00, 0x39, 0x60, 0x0a, 0x60, 0x00, 0xf3, 0x60, 0x2a,
            0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3,
        ]);

        let tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Create)
            .data(deployment_bytecode)
            .value(U256::ZERO)
            .gas_limit(200_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .build()
            .expect("failed to build deployment transaction");

        let mut evm = Context::mainnet().with_db(db).build_mainnet();

        let output = evm.transact(tx).expect("contract deployment failed");

        let contract_address = output
            .result
            .created_address()
            .expect("deployment succeeded without contract address");

        let gas_used = output.result.gas().tx_gas_used();

        println!("Deployment result:");
        println!("{:#?}", output.result);
        println!();

        let deployer_after = output
            .state
            .get(&deployer)
            .expect("deployer missing from resulting state");

        let contract_after = output
            .state
            .get(&contract_address)
            .expect("created contract missing from resulting state");

        DeploymentOutcome {
            contract_address,
            deployer_balance: deployer_after.info.balance,
            deployer_nonce: deployer_after.info.nonce,
            contract_code_hash: contract_after.info.code_hash,
            gas_used,
        }
    }

    pub fn deploy_and_call_test_contract(
        &self,
        deployer: Address,
        deployer_starting_balance: U256,
    ) -> ContractCallOutcome {
        let mut db = InMemoryDB::default();

        db.insert_account_info(
            deployer,
            AccountInfo::from_balance(deployer_starting_balance),
        );

        // Creation code + runtime code.
        //
        // Runtime:
        // PUSH1 0x2a
        // PUSH1 0x00
        // MSTORE
        // PUSH1 0x20
        // PUSH1 0x00
        // RETURN
        //
        // Calling the deployed contract returns 32-byte value 42.
        let deployment_bytecode = Bytes::from_static(&[
            0x60, 0x0a, 0x60, 0x0c, 0x60, 0x00, 0x39, 0x60, 0x0a, 0x60, 0x00, 0xf3, 0x60, 0x2a,
            0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3,
        ]);

        let deploy_tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Create)
            .data(deployment_bytecode)
            .value(U256::ZERO)
            .gas_limit(200_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(0)
            .build()
            .expect("failed to build deployment transaction");

        let mut evm = Context::mainnet().with_db(db).build_mainnet();

        // Transaction #1: deploy and COMMIT the contract.
        let deploy_result = evm
            .transact_commit(deploy_tx)
            .expect("contract deployment failed");

        let contract_address = deploy_result
            .created_address()
            .expect("deployment succeeded without contract address");

        let deploy_gas_used = deploy_result.gas().tx_gas_used();

        // Transaction #2: call the newly deployed contract.
        // The deploy transaction already incremented the deployer's nonce,
        // so this transaction uses nonce 1.
        let call_tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Call(contract_address))
            .data(Bytes::new())
            .value(U256::ZERO)
            .gas_limit(100_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(1)
            .build()
            .expect("failed to build contract-call transaction");

        let call_result = evm.transact_commit(call_tx).expect("contract call failed");

        let call_gas_used = call_result.gas().tx_gas_used();

        let output = call_result
            .output()
            .expect("contract call returned no output")
            .clone();

        println!("Contract call result:");
        println!("{:#?}", call_result);
        println!();

        ContractCallOutcome {
            contract_address,
            output,
            deploy_gas_used,
            call_gas_used,
        }
    }

    pub fn deploy_compiled_counter(
        &self,
        deployer: Address,
        deployer_starting_balance: U256,
    ) -> DeploymentOutcome {
        let mut db = InMemoryDB::default();

        db.insert_account_info(
            deployer,
            AccountInfo::from_balance(deployer_starting_balance),
        );

        let deployment_bytecode = counter_creation_bytecode();

        println!(
            "Counter creation bytecode: {} bytes",
            deployment_bytecode.len()
        );

        let tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Create)
            .data(deployment_bytecode)
            .value(U256::ZERO)
            .gas_limit(1_000_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(0)
            .build()
            .expect("failed to build Counter deployment transaction");

        let mut evm = Context::mainnet().with_db(db).build_mainnet();

        let output = evm.transact(tx).expect("Counter deployment failed");

        let contract_address = output
            .result
            .created_address()
            .expect("Counter deployment returned no contract address");

        let gas_used = output.result.gas().tx_gas_used();

        let deployer_after = output
            .state
            .get(&deployer)
            .expect("deployer missing from resulting state");

        let contract_after = output
            .state
            .get(&contract_address)
            .expect("Counter missing from resulting state");

        println!("Compiled Counter deployment:");
        println!("{:#?}", output.result);

        DeploymentOutcome {
            contract_address,
            deployer_balance: deployer_after.info.balance,
            deployer_nonce: deployer_after.info.nonce,
            contract_code_hash: contract_after.info.code_hash,
            gas_used,
        }
    }

    pub fn deploy_increment_and_read_counter(
        &self,
        deployer: Address,
        deployer_starting_balance: U256,
    ) -> SolidityCounterOutcome {
        let mut db = InMemoryDB::default();

        db.insert_account_info(
            deployer,
            AccountInfo::from_balance(deployer_starting_balance),
        );

        let mut evm = Context::mainnet().with_db(db).build_mainnet();

        // TX #1 — Deploy the compiled Solidity Counter

        let deploy_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 0,
            kind: TransactionKind::Create,
            value: U256::ZERO,
            data: counter_creation_bytecode(),
            gas_limit: 500_000, // keep the same deployment gas limit you were already using
        };

        println!("Deploy transaction hash: {}", deploy_tx.hash());

        let deploy_env = TxEnv::builder()
            .caller(deploy_tx.sender)
            .kind(deploy_tx.revm_kind())
            .value(deploy_tx.value)
            .data(deploy_tx.data.clone())
            .gas_limit(deploy_tx.gas_limit)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(deploy_tx.nonce)
            .build()
            .unwrap();

        let deploy_result = evm
            .transact_commit(deploy_env)
            .expect("Counter deployment failed");

        let contract_address = deploy_result
            .created_address()
            .expect("Counter deployment returned no contract address");

        let deploy_receipt = ExecutionReceipt::from_revm(&deploy_result);

        // TX #2 — Call increment()
        //
        // increment() selector:
        // 0xd09de08a

        let increment_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 1,
            kind: TransactionKind::Call(contract_address),
            value: U256::ZERO,
            data: Bytes::from(vec![
                0xd0, 0x9d, 0xe0, 0x8a, // increment()
            ]),
            gas_limit: 100_000,
        };

        println!("Increment transaction hash: {}", increment_tx.hash());

        let increment_env = TxEnv::builder()
            .caller(increment_tx.sender)
            .kind(increment_tx.revm_kind())
            .value(increment_tx.value)
            .data(increment_tx.data.clone())
            .gas_limit(increment_tx.gas_limit)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(increment_tx.nonce)
            .build()
            .unwrap();

        let increment_result = evm
            .transact_commit(increment_env)
            .expect("increment() call failed");

        let increment_receipt = ExecutionReceipt::from_revm(&increment_result);

        println!(
            "\n=== REVM committed state ===\n{}",
            evm.ctx.journaled_state.database.pretty_print()
        );

        let state_root =
            crate::execution::state_root::compute_state_root(&evm.ctx.journaled_state.database);

        println!("State root: {}", state_root);

        let fail_tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Call(contract_address))
            .data(Bytes::from(vec![0xa9, 0xcc, 0x47, 0x18]))
            .gas_limit(100_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(2)
            .build()
            .unwrap();

        let fail_result = evm
            .transact(fail_tx)
            .expect("fail() transaction should execute");

        let revert_receipt = ExecutionReceipt::from_revm(&fail_result.result);

        let halt_tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Call(contract_address))
            .data(Bytes::from(vec![
                0xd0, 0x9d, 0xe0, 0x8a, // increment()
            ]))
            .gas_limit(25_100)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(2)
            .build()
            .unwrap();

        let halt_result = evm
            .transact(halt_tx)
            .expect("out-of-gas call should execute");

        let halt_receipt = ExecutionReceipt::from_revm(&halt_result.result);

        // TX #3 - Read count()
        //
        // count() selector:
        // 0x06661abd
        //
        // We do NOT commit this transaction because this is
        // acting like a read-only query.

        let count_tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Call(contract_address))
            .data(Bytes::from_static(&[0x06, 0x66, 0x1a, 0xbd]))
            .value(U256::ZERO)
            .gas_limit(100_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(2)
            .build()
            .expect("failed to build count transaction");

        let count_result = evm.transact(count_tx).expect("count() call failed");

        let read_receipt = ExecutionReceipt::from_revm(&count_result.result);

        let count_output = count_result
            .result
            .output()
            .expect("count() returned no output");

        let count = U256::from_be_slice(count_output.as_ref());

        SolidityCounterOutcome {
            contract_address,
            count,
            state_root,
            transactions: vec![deploy_tx, increment_tx],
            deploy_receipt,
            increment_receipt,
            read_receipt,
            revert_receipt,
            halt_receipt,
        }
    }

    pub fn execute_block(
        &self,
        state: &mut MemoryState,
        transactions: &[SolizoneTransaction],
    ) -> BlockExecutionOutcome {
        let mut receipts = Vec::with_capacity(transactions.len());

        for tx in transactions {
            let receipt = self.execute_transaction(state, tx);

            receipts.push(receipt);
        }

        let state_root = state.state_root();

        BlockExecutionOutcome {
            receipts,
            state_root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revm::primitives::address;

    #[test]
    fn executes_value_transfer() {
        let engine = RevmExecutionEngine::new();

        let alice = address!("1111111111111111111111111111111111111111");
        let bob = address!("2222222222222222222222222222222222222222");

        let mut state = MemoryState::new();

        state.insert_account_info(alice, AccountInfo::from_balance(U256::from(1_000_000)));

        state.insert_account_info(bob, AccountInfo::from_balance(U256::ZERO));

        let outcome = engine.execute_transfer(&mut state, alice, bob, U256::from(100));

        assert_eq!(outcome.sender_balance, U256::from(999_900));
        assert_eq!(outcome.sender_nonce, 1);
        assert_eq!(outcome.recipient_balance, U256::from(100));
    }

    #[test]
    fn deploys_contract() {
        let engine = RevmExecutionEngine::new();

        let deployer = address!("1111111111111111111111111111111111111111");

        let outcome = engine.deploy_test_contract(deployer, U256::from(1_000_000));

        assert_eq!(outcome.deployer_nonce, 1);
        assert_eq!(outcome.deployer_balance, U256::from(1_000_000));

        println!("Contract address: {}", outcome.contract_address);
        println!("Contract code hash: {}", outcome.contract_code_hash);
        println!("Deployment gas used: {}", outcome.gas_used);
    }

    #[test]
    fn deploys_and_calls_contract() {
        let engine = RevmExecutionEngine::new();

        let deployer = address!("1111111111111111111111111111111111111111");

        let outcome = engine.deploy_and_call_test_contract(deployer, U256::from(1_000_000));

        let mut expected = vec![0u8; 32];
        expected[31] = 42;

        assert_eq!(outcome.output.as_ref(), expected.as_slice());

        println!("Contract address: {}", outcome.contract_address);
        println!("Returned bytes:   {:?}", outcome.output);
        println!("Deploy gas:       {}", outcome.deploy_gas_used);
        println!("Call gas:         {}", outcome.call_gas_used);
    }

    #[test]
    fn deploys_compiled_solidity_counter() {
        let engine = RevmExecutionEngine::new();

        let deployer = address!("1111111111111111111111111111111111111111");

        let outcome = engine.deploy_compiled_counter(deployer, U256::from(10_000_000));

        assert_eq!(outcome.deployer_nonce, 1);

        println!("Counter address:   {}", outcome.contract_address);

        println!("Counter code hash: {}", outcome.contract_code_hash);

        println!("Deployment gas:    {}", outcome.gas_used);
    }

    #[test]
    fn executes_solidity_counter() {
        let engine = RevmExecutionEngine::new();

        let deployer = address!("1111111111111111111111111111111111111111");

        let outcome = engine.deploy_increment_and_read_counter(deployer, U256::from(10_000_000));

        let transactions_root =
            crate::execution::transactions_root::compute_transactions_root(&outcome.transactions);

        println!("Transactions root: {}", transactions_root);

        let receipts = vec![
            outcome.deploy_receipt.clone(),
            outcome.increment_receipt.clone(),
        ];

        let receipts_root = compute_receipts_root(&receipts);

        let encoded_tx = outcome.transactions[1].encode_canonical();

        let decoded_tx =
            SolizoneTransaction::decode_canonical(&encoded_tx).expect("transaction should decode");

        assert_eq!(decoded_tx, outcome.transactions[1]);

        println!(
            "Transaction round-trip valid: {}",
            decoded_tx == outcome.transactions[1]
        );

        let block = crate::block_builder::build_block(
            1,             // version
            9001,          // chain_id
            0,             // height
            B256::ZERO,    // parent_hash
            1_800_000_000, // timestamp
            outcome.state_root,
            &outcome.transactions,
            &receipts,
            30_000_000, // gas_limit
        );

        let publication = crate::publisher::prepare_block_for_publication(&block);

        assert_eq!(publication.block_hash, block.hash());

        assert_eq!(publication.bytes, block.encode());

        println!(
            "Publication payload size: {} bytes",
            publication.bytes.len()
        );

        println!("Publication block hash: {}", publication.block_hash);

        let encoded_header = block.header.encode();

        let decoded_header = crate::block::SolizoneBlockHeader::decode(&encoded_header)
            .expect("header should decode");

        assert_eq!(decoded_header, block.header);

        println!(
            "Header round-trip valid: {}",
            decoded_header == block.header
        );

        let encoded_block = block.encode();

        let imported_block = crate::block::SolizoneBlock::decode_and_validate(&encoded_block)
            .expect("encoded block should import");

        assert_eq!(imported_block, block);

        println!("Block import valid: {}", imported_block == block);

        let mut tampered_bytes = encoded_block.clone();

        // Flip one byte inside the last transaction.
        let last = tampered_bytes.len() - 1;

        tampered_bytes[last] ^= 0x01;

        let tampered_import = crate::block::SolizoneBlock::decode_and_validate(&tampered_bytes);

        println!("Tampered block import: {:?}", tampered_import);

        assert!(tampered_import.is_err());

        let decoded_block =
            crate::block::SolizoneBlock::decode(&encoded_block).expect("block should decode");

        assert_eq!(decoded_block, block);

        assert!(decoded_block.validate().is_ok());

        println!("Full block round-trip valid: {}", decoded_block == block);

        println!("Decoded block validation: {:?}", decoded_block.validate());

        let mut tampered_block = block.clone();

        tampered_block.transactions[1].gas_limit += 1;

        assert_eq!(&encoded_block[..4], b"SZB1");

        assert_eq!(block.transactions.len(), 2);

        assert!(block.validate_transactions_root());

        assert!(!tampered_block.validate_transactions_root());

        assert!(block.validate().is_ok());

        assert_eq!(
            tampered_block.validate(),
            Err(crate::block::BlockValidationError::TransactionsRootMismatch)
        );

        println!(
            "Block transactions valid: {}",
            block.validate_transactions_root()
        );

        println!(
            "Tampered block transactions valid: {}",
            tampered_block.validate_transactions_root()
        );

        println!("Encoded block size: {} bytes", encoded_block.len());

        println!("Full block hash: {}", block.hash());

        println!("Full block validation: {:?}", block.validate());

        println!("Tampered block validation: {:?}", tampered_block.validate());

        assert_eq!(block.header.receipts_root, receipts_root);

        assert_eq!(block.header.state_root, outcome.state_root);

        assert_eq!(block.header.transactions_root, transactions_root);

        assert_eq!(
            block.header.gas_used,
            outcome.deploy_receipt.gas_used + outcome.increment_receipt.gas_used
        );

        assert_eq!(block.header.encode().len(), 170);

        println!("Block receipts root: {}", block.header.receipts_root);

        println!("Block gas used: {}", block.header.gas_used);

        println!("Block hash: {}", block.header.hash());

        let increment_proof =
            build_receipt_proof(&receipts, 1).expect("failed to build increment receipt proof");

        let proof_valid = verify_receipt_proof(&receipts[1], &increment_proof, receipts_root);

        assert_eq!(outcome.count, U256::from(1));

        assert_eq!(outcome.deploy_receipt.status, ExecutionStatus::Success);

        assert_eq!(outcome.increment_receipt.status, ExecutionStatus::Success);

        assert_eq!(outcome.read_receipt.status, ExecutionStatus::Success);

        assert_eq!(
            outcome.deploy_receipt.contract_address,
            Some(outcome.contract_address)
        );

        assert_eq!(outcome.increment_receipt.contract_address, None);

        assert_eq!(outcome.read_receipt.contract_address, None);

        assert_eq!(outcome.increment_receipt.logs.len(), 1);

        assert_eq!(outcome.deploy_receipt.logs.len(), 0);

        assert_eq!(outcome.read_receipt.logs.len(), 0);

        assert!(proof_valid);

        assert_eq!(outcome.revert_receipt.status, ExecutionStatus::Revert);

        assert_eq!(outcome.halt_receipt.status, ExecutionStatus::Halt);

        println!("Revert receipt: {:?}", outcome.revert_receipt);
        println!("Halt receipt: {:?}", outcome.halt_receipt);

        let mut tampered_receipt = receipts[1].clone();

        tampered_receipt.gas_used += 1;

        let tampered_valid =
            verify_receipt_proof(&tampered_receipt, &increment_proof, receipts_root);

        assert!(!tampered_valid);

        println!("Tampered receipt proof valid: {}", tampered_valid);

        println!("Counter address: {}", outcome.contract_address);

        println!("Counter value:   {}", outcome.count);

        println!("Deploy receipt:  {:?}", outcome.deploy_receipt);

        println!("Increment receipt: {:?}", outcome.increment_receipt);

        println!("Read receipt:    {:?}", outcome.read_receipt);

        println!("Increment log: {:?}", outcome.increment_receipt.logs[0]);

        println!(
            "Increment receipt hash: {}",
            outcome.increment_receipt.hash()
        );

        println!("Receipts root: {}", receipts_root);

        println!("Increment proof: {:?}", increment_proof);

        println!("Increment receipt proof valid: {}", proof_valid);

        println!("Block state root: {}", block.header.state_root);

        println!(
            "Block transactions root: {}",
            block.header.transactions_root
        );
    }

    #[test]
    fn preserves_state_across_multiple_transfers() {
        let engine = RevmExecutionEngine::new();

        let alice = address!("1111111111111111111111111111111111111111");
        let bob = address!("2222222222222222222222222222222222222222");
        let charlie = address!("3333333333333333333333333333333333333333");

        let mut state = MemoryState::new();

        state.insert_account_info(alice, AccountInfo::from_balance(U256::from(1_000_000)));

        state.insert_account_info(bob, AccountInfo::from_balance(U256::ZERO));

        state.insert_account_info(charlie, AccountInfo::from_balance(U256::ZERO));

        let first = engine.execute_transfer(&mut state, alice, bob, U256::from(100));

        assert_eq!(first.sender_balance, U256::from(999_900));
        assert_eq!(first.sender_nonce, 1);
        assert_eq!(first.recipient_balance, U256::from(100));

        let second = engine.execute_transfer(&mut state, alice, charlie, U256::from(200));

        assert_eq!(second.sender_balance, U256::from(999_700));
        assert_eq!(second.sender_nonce, 2);
        assert_eq!(second.recipient_balance, U256::from(200));

        let bob_after = state
            .db()
            .cache
            .accounts
            .get(&bob)
            .and_then(|account| account.info())
            .expect("Bob missing from state");

        assert_eq!(bob_after.balance, U256::from(100));
    }

    #[test]
    fn executes_counter_through_generic_transaction_executor() {
        let engine = RevmExecutionEngine::new();

        let deployer = address!("1111111111111111111111111111111111111111");

        let mut state = MemoryState::new();

        state.insert_account_info(deployer, AccountInfo::from_balance(U256::from(10_000_000)));

        // TX #1 — deploy the compiled Solidity Counter
        let deploy_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 0,
            kind: TransactionKind::Create,
            value: U256::ZERO,
            data: counter_creation_bytecode(),
            gas_limit: 500_000,
        };

        let deploy_receipt = engine.execute_transaction(&mut state, &deploy_tx);

        assert_eq!(deploy_receipt.status, ExecutionStatus::Success);

        let contract_address = deploy_receipt
            .contract_address
            .expect("Counter deployment returned no contract address");

        // TX #2 — increment()
        let increment_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 1,
            kind: TransactionKind::Call(contract_address),
            value: U256::ZERO,
            data: Bytes::from_static(&[
                0xd0, 0x9d, 0xe0, 0x8a, // increment()
            ]),
            gas_limit: 100_000,
        };

        let increment_receipt = engine.execute_transaction(&mut state, &increment_tx);

        assert_eq!(increment_receipt.status, ExecutionStatus::Success);

        assert_eq!(increment_receipt.contract_address, None);

        // Read count() without committing state.
        let read_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 2,
            kind: TransactionKind::Call(contract_address),
            value: U256::ZERO,
            data: Bytes::from_static(&[
                0x06, 0x66, 0x1a, 0xbd, // count()
            ]),
            gas_limit: 100_000,
        };

        let read_receipt = engine.execute_call(&mut state, &read_tx);

        assert_eq!(read_receipt.status, ExecutionStatus::Success);

        let count = U256::from_be_slice(read_receipt.output.as_ref());

        assert_eq!(count, U256::from(1));

        let snapshot = state.snapshot();

        let counter_snapshot = snapshot
            .accounts
            .iter()
            .find(|account| account.address == contract_address)
            .expect("Counter missing from snapshot");

        assert!(
            counter_snapshot.code.is_some(),
            "Counter bytecode missing from snapshot"
        );

        let counter_slot_zero = counter_snapshot
            .storage
            .iter()
            .find(|slot| slot.key == U256::ZERO)
            .expect("Counter storage slot 0 missing from snapshot");

        assert_eq!(counter_slot_zero.value, U256::from(1));

        let deployer_snapshot = snapshot
            .accounts
            .iter()
            .find(|account| account.address == deployer)
            .expect("deployer missing from snapshot");

        assert_eq!(deployer_snapshot.nonce, 2);

        // Verify the ORIGINAL state before destroying it.

        let deployer_after = state
            .db()
            .cache
            .accounts
            .get(&deployer)
            .and_then(|account| account.info())
            .expect("deployer missing from state");

        assert_eq!(deployer_after.nonce, 2);

        let contract_after = state
            .db()
            .cache
            .accounts
            .get(&contract_address)
            .and_then(|account| account.info())
            .expect("Counter contract missing from state");

        assert_ne!(contract_after.code_hash, B256::ZERO);

        println!("Counter value: {}", count);
        println!("Snapshot accounts: {}", snapshot.accounts.len());
        println!("Counter snapshot storage[0]: {}", counter_slot_zero.value);
        println!("Counter address: {}", contract_address);
        println!("Deployer nonce: {}", deployer_after.nonce);
        println!("Counter code hash: {}", contract_after.code_hash);

        // Simulate restart.
        //
        // Destroy the original REVM database and rebuild an entirely
        // new MemoryState from the snapshot.

        drop(state);

        let mut restored_state = MemoryState::from_snapshot(snapshot);

        // Read count() from the RESTORED state.

        let restored_read_receipt = engine.execute_call(&mut restored_state, &read_tx);

        assert_eq!(restored_read_receipt.status, ExecutionStatus::Success);

        let restored_count = U256::from_be_slice(restored_read_receipt.output.as_ref());

        assert_eq!(restored_count, U256::from(1));

        println!("Counter value after state restore: {}", restored_count);

        println!("Deploy receipt: {:?}", deploy_receipt);
        println!("Increment receipt: {:?}", increment_receipt);
    }

    #[test]
    fn contract_state_root_survives_snapshot_restore() {
        let engine = RevmExecutionEngine::new();

        let deployer = address!("1111111111111111111111111111111111111111");

        let mut state = MemoryState::new();

        state.insert_account_info(deployer, AccountInfo::from_balance(U256::from(10_000_000)));

        // TX #1 — deploy Counter
        let deploy_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 0,
            kind: TransactionKind::Create,
            value: U256::ZERO,
            data: counter_creation_bytecode(),
            gas_limit: 500_000,
        };

        let deploy_receipt = engine.execute_transaction(&mut state, &deploy_tx);

        assert_eq!(deploy_receipt.status, ExecutionStatus::Success,);

        let contract_address = deploy_receipt
            .contract_address
            .expect("Counter deployment returned no address");

        // TX #2 — increment Counter to 1
        let increment_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 1,
            kind: TransactionKind::Call(contract_address),
            value: U256::ZERO,
            data: Bytes::from_static(&[0xd0, 0x9d, 0xe0, 0x8a]),
            gas_limit: 100_000,
        };

        let increment_receipt = engine.execute_transaction(&mut state, &increment_tx);

        assert_eq!(increment_receipt.status, ExecutionStatus::Success,);

        // Protocol root before restart.
        let root_before = state.state_root();

        // Simulate persistence + process restart.
        let snapshot = state.snapshot();

        drop(state);

        let mut restored = MemoryState::from_snapshot(snapshot);

        // Protocol root must remain identical.
        let root_after = restored.state_root();

        assert_eq!(
            root_before, root_after,
            "contract state root changed after snapshot restore"
        );

        // Read count() from reconstructed state.
        let read_tx = SolizoneTransaction {
            sender: deployer,
            nonce: 2,
            kind: TransactionKind::Call(contract_address),
            value: U256::ZERO,
            data: Bytes::from_static(&[0x06, 0x66, 0x1a, 0xbd]),
            gas_limit: 100_000,
        };

        let read_receipt = engine.execute_call(&mut restored, &read_tx);

        assert_eq!(read_receipt.status, ExecutionStatus::Success,);

        let count = U256::from_be_slice(read_receipt.output.as_ref());

        assert_eq!(
            count,
            U256::from(1),
            "Counter storage was not restored correctly"
        );

        // Contract bytecode must also exist after restart.
        let contract = restored
            .db()
            .cache
            .accounts
            .get(&contract_address)
            .and_then(|account| account.info())
            .expect("restored Counter account missing");

        let code = contract
            .code
            .as_ref()
            .expect("restored Counter bytecode missing");

        assert!(
            !code.original_bytes().is_empty(),
            "restored Counter bytecode is empty"
        );

        println!("Contract address:          {}", contract_address);

        println!("State root before restart: {}", root_before);

        println!("State root after restart:  {}", root_after);

        println!("Counter after restart:     {}", count);

        println!(
            "Runtime bytecode size:     {} bytes",
            code.original_bytes().len()
        );
    }

    #[test]
    fn generic_execution_changes_state_root() {
        let engine = RevmExecutionEngine::new();
        let mut state = MemoryState::new();

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(1000)));

        let root_before = state.state_root();

        let tx = SolizoneTransaction {
            sender,
            nonce: 0,
            kind: TransactionKind::Call(recipient),
            value: U256::from(100),
            data: Bytes::new(),
            gas_limit: 21_000,
        };

        let receipt = engine.execute_transaction(&mut state, &tx);

        let root_after = state.state_root();

        assert_ne!(
            root_before, root_after,
            "state root should change after committed execution"
        );

        assert_eq!(
            state
                .db()
                .cache
                .accounts
                .get(&sender)
                .expect("sender missing")
                .info
                .nonce,
            1,
        );

        println!("State root before: {}", root_before);
        println!("State root after:  {}", root_after);
        println!("Gas used: {}", receipt.gas_used);
    }

    #[test]
    fn executes_ordered_transactions_as_block() {
        let engine = RevmExecutionEngine::new();
        let mut state = MemoryState::new();

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

        let root_before = state.state_root();

        let transactions = vec![
            SolizoneTransaction {
                sender,
                nonce: 0,
                kind: TransactionKind::Call(recipient),
                value: U256::from(100),
                data: Bytes::new(),
                gas_limit: 21_000,
            },
            SolizoneTransaction {
                sender,
                nonce: 1,
                kind: TransactionKind::Call(recipient),
                value: U256::from(200),
                data: Bytes::new(),
                gas_limit: 21_000,
            },
        ];

        let outcome = engine.execute_block(&mut state, &transactions);

        assert_eq!(outcome.receipts.len(), 2);

        assert_ne!(root_before, outcome.state_root,);

        assert_eq!(
            outcome.state_root,
            state.state_root(),
            "block outcome must contain final post-block state root"
        );

        assert_eq!(
            state
                .db()
                .cache
                .accounts
                .get(&sender)
                .expect("sender missing")
                .info
                .nonce,
            2,
        );

        println!("Pre-block state root:  {}", root_before);

        println!("Post-block state root: {}", outcome.state_root);

        println!("Executed transactions: {}", outcome.receipts.len());
    }

    #[test]
    fn builds_canonical_block_from_generic_execution() {
        let engine = RevmExecutionEngine::new();
        let mut state = MemoryState::new();

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

        let transactions = vec![
            SolizoneTransaction {
                sender,
                nonce: 0,
                kind: TransactionKind::Call(recipient),
                value: U256::from(100),
                data: Bytes::new(),
                gas_limit: 21_000,
            },
            SolizoneTransaction {
                sender,
                nonce: 1,
                kind: TransactionKind::Call(recipient),
                value: U256::from(200),
                data: Bytes::new(),
                gas_limit: 21_000,
            },
        ];

        let outcome = engine.execute_block(&mut state, &transactions);

        let block = crate::block_builder::build_block(
            1,
            9001,
            0,
            B256::ZERO,
            1_800_000_000,
            outcome.state_root,
            &transactions,
            &outcome.receipts,
            30_000_000,
        );

        assert_eq!(block.header.state_root, outcome.state_root,);

        assert_eq!(
            block.header.transactions_root,
            crate::execution::transactions_root::compute_transactions_root(&transactions,),
        );

        assert_eq!(
            block.header.receipts_root,
            crate::execution::receipts_root::compute_receipts_root(&outcome.receipts,),
        );

        let expected_gas_used: u64 = outcome
            .receipts
            .iter()
            .map(|receipt| receipt.gas_used)
            .sum();

        assert_eq!(block.header.gas_used, expected_gas_used,);

        block
            .validate()
            .expect("execution-backed block should be valid");

        println!("Block state root:        {}", block.header.state_root);

        println!(
            "Block transactions root: {}",
            block.header.transactions_root
        );

        println!("Block receipts root:     {}", block.header.receipts_root);

        println!("Block gas used:          {}", block.header.gas_used);

        println!("Block hash:              {}", block.hash());
    }

    #[test]
    fn progresses_state_across_linked_blocks() {
        let engine = RevmExecutionEngine::new();

        let mut state = MemoryState::new();

        let sender = address!("1111111111111111111111111111111111111111");

        let recipient = address!("2222222222222222222222222222222222222222");

        state.insert_account_info(sender, AccountInfo::from_balance(U256::from(10_000)));

        /*
         * ============================================================
         * BLOCK #0
         * ============================================================
         */

        let block_0_transactions = vec![SolizoneTransaction {
            sender,
            nonce: 0,
            kind: TransactionKind::Call(recipient),
            value: U256::from(100),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let outcome_0 = engine.execute_block(&mut state, &block_0_transactions);

        let block_0 = crate::block_builder::build_block(
            1,
            9001,
            0,
            B256::ZERO,
            1_800_000_000,
            outcome_0.state_root,
            &block_0_transactions,
            &outcome_0.receipts,
            30_000_000,
        );

        block_0.validate().expect("block #0 should be valid");

        let block_0_hash = block_0.hash();

        /*
         * ============================================================
         * BLOCK #1
         * ============================================================
         */

        let block_1_transactions = vec![SolizoneTransaction {
            sender,
            nonce: 1,
            kind: TransactionKind::Call(recipient),
            value: U256::from(200),
            data: Bytes::new(),
            gas_limit: 21_000,
        }];

        let outcome_1 = engine.execute_block(&mut state, &block_1_transactions);

        let block_1 = crate::block_builder::build_block(
            1,
            9001,
            1,
            block_0_hash,
            1_800_000_001,
            outcome_1.state_root,
            &block_1_transactions,
            &outcome_1.receipts,
            30_000_000,
        );

        block_1.validate().expect("block #1 should be valid");

        /*
         * ============================================================
         * CHAIN INVARIANTS
         * ============================================================
         */

        assert_eq!(block_0.header.height, 0,);

        assert_eq!(block_0.header.parent_hash, B256::ZERO,);

        assert_eq!(block_1.header.height, 1,);

        assert_eq!(block_1.header.parent_hash, block_0.hash(),);

        assert_ne!(
            block_0.header.state_root, block_1.header.state_root,
            "state root should change after block #1 execution"
        );

        assert_eq!(block_1.header.state_root, state.state_root(),);

        /*
         * Sender executed two transactions across two blocks.
         */
        let sender_account = state
            .db()
            .cache
            .accounts
            .get(&sender)
            .and_then(|account| account.info())
            .expect("sender missing");

        assert_eq!(sender_account.nonce, 2,);

        /*
         * Recipient received 100 + 200.
         */
        let recipient_account = state
            .db()
            .cache
            .accounts
            .get(&recipient)
            .and_then(|account| account.info())
            .expect("recipient missing");

        assert_eq!(recipient_account.balance, U256::from(300),);

        println!("Block #0 hash:       {}", block_0.hash());

        println!("Block #0 state root: {}", block_0.header.state_root);

        println!("Block #1 parent:     {}", block_1.header.parent_hash);

        println!("Block #1 hash:       {}", block_1.hash());

        println!("Block #1 state root: {}", block_1.header.state_root);

        println!("Final sender nonce:  {}", sender_account.nonce);

        println!("Recipient balance:   {}", recipient_account.balance);
    }
}
