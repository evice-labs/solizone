use std::env;

use solizone_evm::logos_storage_client::LogosStorageClient;

fn main() -> Result<(), String> {
    let cid = env::args()
        .nth(1)
        .ok_or_else(|| "usage: storage_client_download <cid> <destination>".to_string())?;

    let destination = env::args()
        .nth(2)
        .ok_or_else(|| "usage: storage_client_download <cid> <destination>".to_string())?;

    let client = LogosStorageClient::new();

    client.download(&cid, &destination)?;

    println!("Downloaded successfully");

    println!("Destination: {destination}");

    Ok(())
}
