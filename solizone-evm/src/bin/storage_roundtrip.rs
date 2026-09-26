use std::{fs, path::PathBuf};

use storage_bindings::{
    DownloadStreamOptions, LogLevel, StorageConfig, StorageNode, UploadOptions, download_stream,
    upload_file,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_dir: PathBuf = std::env::temp_dir().join("solizone-storage-roundtrip");

    if base_dir.exists() {
        fs::remove_dir_all(&base_dir)?;
    }

    fs::create_dir_all(&base_dir)?;

    let source_path = base_dir.join("source.txt");

    let download_path = base_dir.join("downloaded.txt");

    let payload = b"solizone logos storage roundtrip";

    fs::write(&source_path, payload)?;

    let config = StorageConfig::new()
        .log_level(LogLevel::Info)
        .data_dir(base_dir.join("storage-data"))
        .storage_quota(100 * 1024 * 1024)
        .max_peers(50)
        .discovery_port(8090);

    let node = StorageNode::new(config).await?;

    node.start().await?;

    /*
     * Upload.
     */

    let upload_options = UploadOptions::new().filepath(&source_path);

    let upload_result = upload_file(&node, upload_options).await?;

    println!("Uploaded CID: {}", upload_result.cid,);

    println!("Uploaded size: {} bytes", upload_result.size,);

    /*
     * Download using the CID.
     */

    let download_options = DownloadStreamOptions::new(&upload_result.cid).filepath(&download_path);

    let download_result = download_stream(&node, &upload_result.cid, download_options).await?;

    println!("Downloaded size: {} bytes", download_result.size,);

    /*
     * Verify exact content.
     */

    let original = fs::read(&source_path)?;

    let downloaded = fs::read(&download_path)?;

    assert_eq!(
        original, downloaded,
        "downloaded Logos Storage payload differs from original",
    );

    println!("Roundtrip verified ");

    node.stop().await?;
    node.destroy().await?;

    fs::remove_dir_all(&base_dir)?;

    Ok(())
}
