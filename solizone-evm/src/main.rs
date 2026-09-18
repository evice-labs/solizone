mod block;
mod execution;

use execution::RevmExecutionEngine;
use revm::primitives::{U256, address};

fn main() {
    println!("=== Solizone EVM ===");
    println!("Generic EVM value transfer\n");

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

    println!("=== Resulting State ===");
    println!("Sender balance:    {}", outcome.sender_balance);
    println!("Sender nonce:      {}", outcome.sender_nonce);
    println!("Recipient balance: {}", outcome.recipient_balance);

    println!();
    println!("✅ Generic Solizone EVM transfer executed");
}
