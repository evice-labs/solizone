use std::{env, fs, path::Path};

use revm::primitives::{B256, U256, address};

use solizone_evm::state::{
    AccountSnapshot, SolizoneCheckpoint, StateSnapshot, StorageSlotSnapshot,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("create") => {
            let path = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("solizone-checkpoint.json");

            create_checkpoint(path)?;
        }

        Some("verify") => {
            let path = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("solizone-checkpoint-downloaded.json");

            verify_checkpoint(path)?;
        }

        _ => {
            println!(
                "Usage:\n\
                 cargo run --bin checkpoint_fixture -- create <path>\n\
                 cargo run --bin checkpoint_fixture -- verify <path>"
            );
        }
    }

    Ok(())
}

fn expected_checkpoint() -> SolizoneCheckpoint {
    let state = StateSnapshot {
        accounts: vec![AccountSnapshot {
            address: address!("1111111111111111111111111111111111111111"),
            balance: U256::from(1000),
            nonce: 2,
            code: None,
            storage: vec![StorageSlotSnapshot {
                key: U256::ZERO,
                value: U256::from(42),
            }],
        }],
    };

    SolizoneCheckpoint::new(state, 10, B256::from([0x22; 32]))
}

fn create_checkpoint(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let checkpoint = expected_checkpoint();

    let json = checkpoint.encode_json().map_err(std::io::Error::other)?;

    fs::write(path.as_ref(), json)?;

    println!("Checkpoint written: {}", path.as_ref().display(),);

    println!("Next height: {}", checkpoint.next_height,);

    println!("Parent hash: {:#x}", checkpoint.parent_hash,);

    Ok(())
}

fn verify_checkpoint(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path.as_ref())?;

    let recovered = SolizoneCheckpoint::decode_json(&json).map_err(std::io::Error::other)?;

    let expected = expected_checkpoint();

    assert_eq!(
        recovered, expected,
        "downloaded checkpoint differs from original checkpoint",
    );

    println!("Recovered checkpoint: {}", path.as_ref().display(),);

    println!("Next height: {}", recovered.next_height,);

    println!("Parent hash: {:#x}", recovered.parent_hash,);

    println!("Checkpoint verified");

    Ok(())
}
