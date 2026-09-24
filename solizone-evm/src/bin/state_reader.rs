use revm::primitives::{Address, Bytes, U256, address};

use solizone_evm::{
    execution::{
        revm_engine::RevmExecutionEngine,
        transaction::{SolizoneTransaction, TransactionKind},
    },
    state::{FileStateBackend, MemoryState, StateBackend},
};

fn main() {
    let state_path = "solizone-state.json";

    let backend = FileStateBackend::new(state_path);

    // Load the snapshot created by Process A.
    let snapshot = backend
        .load()
        .expect("failed to load Solizone state")
        .expect("no persisted Solizone state found");

    println!("Loaded snapshot with {} accounts.", snapshot.accounts.len());

    // Reconstruct a completely new REVM state.
    let mut state = MemoryState::from_snapshot(snapshot);

    let engine = RevmExecutionEngine::new();

    let deployer = address!("1111111111111111111111111111111111111111");

    let contract_address: Address = "0x8f7a45ebde059392e46a46dcc14ab24681a961ea"
        .parse()
        .expect("invalid Counter address");

    // Read Counter.count().
    //
    // This is read-only, so nonce 2 is fine and nothing
    // will be committed.
    let read_tx = SolizoneTransaction {
        sender: deployer,
        nonce: 2,
        kind: TransactionKind::Call(contract_address),
        value: U256::ZERO,
        data: Bytes::from_static(&[0x06, 0x66, 0x1a, 0xbd]),
        gas_limit: 100_000,
    };

    let read_receipt = engine.execute_call(&mut state, &read_tx);

    let count = U256::from_be_slice(read_receipt.output.as_ref());

    println!("Counter address: {}", contract_address);

    println!("Recovered Counter value: {}", count);

    assert_eq!(count, U256::from(1), "persisted Counter value is incorrect");

    // Continue execution from the recovered state.
    //
    // Process A used deployer nonces 0 and 1.
    // The recovered deployer nonce is therefore 2.
    let increment_tx = SolizoneTransaction {
        sender: deployer,
        nonce: 2,
        kind: TransactionKind::Call(contract_address),
        value: U256::ZERO,
        data: Bytes::from_static(&[
            0xd0, 0x9d, 0xe0, 0x8a, // increment()
        ]),
        gas_limit: 100_000,
    };

    let increment_receipt = engine.execute_transaction(&mut state, &increment_tx);

    let read_after_increment = SolizoneTransaction {
        sender: deployer,
        nonce: 3,
        kind: TransactionKind::Call(contract_address),
        value: U256::ZERO,
        data: Bytes::from_static(&[
            0x06, 0x66, 0x1a, 0xbd, // count()
        ]),
        gas_limit: 100_000,
    };

    let read_after_receipt = engine.execute_call(&mut state, &read_after_increment);

    let count_after = U256::from_be_slice(read_after_receipt.output.as_ref());

    let updated_snapshot = state.snapshot();

    backend
        .save(&updated_snapshot)
        .expect("failed to persist updated Solizone state");

    println!("Recovered Counter value: {}", count);

    println!(
        "Post-restart increment gas used: {}",
        increment_receipt.gas_used
    );

    println!(
        "Counter value after post-restart increment: {}",
        count_after
    );

    println!("Updated state persisted.");
    println!("Process B exiting with Counter = 2.");

    assert_eq!(
        count_after,
        U256::from(2),
        "Counter did not continue from recovered state"
    );

    println!(
        "Counter value after post-restart increment: {}",
        count_after
    );
}
