use solizone_evm::{execution::RevmExecutionEngine, state::MemoryState};

use revm::{
    primitives::{U256, address},
    state::AccountInfo,
};

fn main() {
    println!("=== Solizone EVM ===");
    println!("Generic EVM value transfer\n");

    let engine = RevmExecutionEngine::new();

    let alice = address!("1111111111111111111111111111111111111111");
    let bob = address!("2222222222222222222222222222222222222222");

    let mut state = MemoryState::new();

    state.insert_account_info(alice, AccountInfo::from_balance(U256::from(1_000_000)));

    state.insert_account_info(bob, AccountInfo::from_balance(U256::ZERO));

    let outcome = engine.execute_transfer(&mut state, alice, bob, U256::from(100));

    println!("=== Resulting State ===");
    println!("Sender balance:    {}", outcome.sender_balance);
    println!("Sender nonce:      {}", outcome.sender_nonce);
    println!("Recipient balance: {}", outcome.recipient_balance);

    println!();
    println!("Generic Solizone EVM transfer executed");
}
