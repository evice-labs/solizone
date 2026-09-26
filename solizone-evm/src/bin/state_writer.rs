use revm::{
    primitives::{Bytes, U256, address},
    state::AccountInfo,
};

use solizone_evm::{
    execution::{
        revm_engine::RevmExecutionEngine,
        transaction::{SolizoneTransaction, TransactionKind},
    },
    state::{FileStateBackend, MemoryState, StateBackend},
};

fn counter_creation_bytecode() -> Bytes {
    let hex_source = include_str!("../../contracts/Counter.creation.hex").trim();

    let hex_source = hex_source.strip_prefix("0x").unwrap_or(hex_source);

    let bytes = hex::decode(hex_source).expect("invalid Counter creation bytecode");

    Bytes::from(bytes)
}

fn main() {
    let state_path = "solizone-state.json";

    let engine = RevmExecutionEngine::new();

    let deployer = address!("1111111111111111111111111111111111111111");

    let mut state = MemoryState::new();

    state.insert_account_info(deployer, AccountInfo::from_balance(U256::from(10_000_000)));

    // TX #1 — deploy Counter.
    let deploy_tx = SolizoneTransaction {
        sender: deployer,
        nonce: 0,
        kind: TransactionKind::Create,
        value: U256::ZERO,
        data: counter_creation_bytecode(),
        gas_limit: 500_000,
    };

    let deploy_receipt = engine.execute_transaction(&mut state, &deploy_tx);

    let contract_address = deploy_receipt
        .contract_address
        .expect("Counter deployment failed");

    // TX #2 — increment().
    let increment_tx = SolizoneTransaction {
        sender: deployer,
        nonce: 1,
        kind: TransactionKind::Call(contract_address),
        value: U256::ZERO,
        data: Bytes::from_static(&[0xd0, 0x9d, 0xe0, 0x8a]),
        gas_limit: 100_000,
    };

    engine.execute_transaction(&mut state, &increment_tx);

    let snapshot = state.snapshot();

    let backend = FileStateBackend::new(state_path);

    backend
        .save(&snapshot)
        .expect("failed to persist Solizone state");

    println!("Solizone state persisted.");
    println!("State file: {}", state_path);
    println!("Counter address: {}", contract_address);
    println!("Deployer nonce after execution: 2");
    println!("Process A exiting.");
}
