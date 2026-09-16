use revm::{
    context::TxEnv,
    database::{InMemoryDB, BENCH_CALLER, BENCH_TARGET},
    primitives::{TxKind, U256},
    state::AccountInfo,
    Context, ExecuteEvm, MainBuilder, MainContext,
};

fn main() {
    println!("Solizone EVM");
    println!("First deterministic EVM state transition\n");

    // For now, use REVM's fixed test addresses.
    // We'll introduce proper Solizone accounts later.
    let alice = BENCH_CALLER;
    let bob = BENCH_TARGET;

    // Temporary in-memory state database.
    let mut db = InMemoryDB::default();

    // Give Alice an initial balance.
    let alice_starting_balance = U256::from(1_000_000);

    db.insert_account_info(alice, AccountInfo::from_balance(alice_starting_balance));

    println!("Alice: {alice}");
    println!("Bob:   {bob}");
    println!();
    println!("Alice starting balance: {alice_starting_balance}");
    println!("Bob starting balance:   0");
    println!();

    // Alice sends 100 units to Bob.
    let tx = TxEnv::builder()
        .caller(alice)
        .kind(TxKind::Call(bob))
        .value(U256::from(100))
        .gas_limit(21_000)
        .gas_price(0)
        .gas_priority_fee(None)
        .build()
        .expect("failed to build transaction");

    // Build the EVM.
    let mut evm = Context::mainnet().with_db(db).build_mainnet();

    // Execute the transaction.
    let output = evm.transact(tx).expect("EVM transaction failed");

    println!("Execution result:");
    println!("{:#?}", output.result);
    println!();

    // REVM returns the accounts touched by execution.
    let alice_after = output
        .state
        .get(&alice)
        .expect("Alice missing from resulting state");

    let bob_after = output
        .state
        .get(&bob)
        .expect("Bob missing from resulting state");

    println!("=== Resulting State ===");
    println!("Alice balance: {}", alice_after.info.balance);
    println!("Alice nonce:   {}", alice_after.info.nonce);
    println!("Bob balance:   {}", bob_after.info.balance);

    println!();
    println!("✅ First Solizone EVM transaction executed");
}
