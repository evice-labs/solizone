use std::env;

use solizone_evm::logos_storage_client::LogosStorageClient;

fn main() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: storage_client_upload <file>".to_string())?;

    let client = LogosStorageClient::new();

    let cid = client.upload(&path)?;

    println!("Uploaded successfully");

    println!("CID: {cid}");

    Ok(())
}
