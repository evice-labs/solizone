use revm::{
    Context, ExecuteCommitEvm, ExecuteEvm, MainBuilder, MainContext,
    context::TxEnv,
    database::InMemoryDB,
    primitives::{Address, B256, Bytes, TxKind, U256},
    state::AccountInfo,
};

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
    pub deploy_gas_used: u64,
    pub increment_gas_used: u64,
    pub read_gas_used: u64,
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

    pub fn execute_transfer(
        &self,
        sender: Address,
        recipient: Address,
        sender_starting_balance: U256,
        recipient_starting_balance: U256,
        value: U256,
    ) -> TransferOutcome {
        let mut db = InMemoryDB::default();

        db.insert_account_info(sender, AccountInfo::from_balance(sender_starting_balance));

        db.insert_account_info(
            recipient,
            AccountInfo::from_balance(recipient_starting_balance),
        );

        let tx = TxEnv::builder()
            .caller(sender)
            .kind(TxKind::Call(recipient))
            .value(value)
            .gas_limit(21_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .build()
            .expect("failed to build transaction");

        let mut evm = Context::mainnet().with_db(db).build_mainnet();

        let output = evm.transact(tx).expect("EVM transaction failed");

        println!("Execution result:");
        println!("{:#?}", output.result);
        println!();

        let sender_after = output
            .state
            .get(&sender)
            .expect("sender missing from resulting state");

        let recipient_after = output
            .state
            .get(&recipient)
            .expect("recipient missing from resulting state");

        TransferOutcome {
            sender_balance: sender_after.info.balance,
            sender_nonce: sender_after.info.nonce,
            recipient_balance: recipient_after.info.balance,
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

        let deploy_tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Create)
            .data(counter_creation_bytecode())
            .value(U256::ZERO)
            .gas_limit(1_000_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(0)
            .build()
            .expect("failed to build Counter deployment transaction");

        let deploy_result = evm
            .transact_commit(deploy_tx)
            .expect("Counter deployment failed");

        let contract_address = deploy_result
            .created_address()
            .expect("Counter deployment returned no contract address");

        let deploy_gas_used = deploy_result.gas().tx_gas_used();

        // TX #2 — Call increment()
        //
        // increment() selector:
        // 0xd09de08a

        let increment_tx = TxEnv::builder()
            .caller(deployer)
            .kind(TxKind::Call(contract_address))
            .data(Bytes::from_static(&[0xd0, 0x9d, 0xe0, 0x8a]))
            .value(U256::ZERO)
            .gas_limit(100_000)
            .gas_price(0)
            .gas_priority_fee(None)
            .nonce(1)
            .build()
            .expect("failed to build increment transaction");

        let increment_result = evm
            .transact_commit(increment_tx)
            .expect("increment() call failed");

        let increment_gas_used = increment_result.gas().tx_gas_used();

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

        let read_gas_used = count_result.result.gas().tx_gas_used();

        let count_output = count_result
            .result
            .output()
            .expect("count() returned no output");

        let count = U256::from_be_slice(count_output.as_ref());

        SolidityCounterOutcome {
            contract_address,
            count,
            deploy_gas_used,
            increment_gas_used,
            read_gas_used,
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

        let outcome = engine.execute_transfer(
            alice,
            bob,
            U256::from(1_000_000),
            U256::from(0),
            U256::from(100),
        );

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

        assert_eq!(outcome.count, U256::from(1));

        println!("Counter address: {}", outcome.contract_address);

        println!("Counter value:   {}", outcome.count);

        println!("Deploy gas:      {}", outcome.deploy_gas_used);

        println!("Increment gas:   {}", outcome.increment_gas_used);

        println!("Read gas:        {}", outcome.read_gas_used);
    }
}
