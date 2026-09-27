use std::{fs, path::Path, process::Command, thread, time::Duration};

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LogosStorageClient {
    logosctl_bin: String,
    chunk_size: u64,
}

impl Default for LogosStorageClient {
    fn default() -> Self {
        Self {
            logosctl_bin: "logosctl".to_string(),
            chunk_size: 65_536,
        }
    }
}

impl LogosStorageClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upload(&self, path: impl AsRef<Path>) -> Result<String, String> {
        let path = path
            .as_ref()
            .canonicalize()
            .map_err(|error| format!("failed to resolve upload path: {error}"))?;

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "upload path has no valid filename".to_string())?;

        let path_string = path
            .to_str()
            .ok_or_else(|| "upload path is not valid UTF-8".to_string())?;

        let chunk_size = self.chunk_size.to_string();

        let upload = self.call(&[
            "call",
            "storage_module",
            "uploadUrl",
            path_string,
            &chunk_size,
        ])?;

        let result = upload
            .get("result")
            .ok_or_else(|| "logosctl response missing result".to_string())?;

        if !result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(format!(
                "Logos Storage upload rejected: {}",
                result
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error"),
            ));
        }

        println!(
            "Upload session accepted: {}",
            result
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>"),
        );

        self.wait_for_cid(filename)
    }

    pub fn download(&self, cid: &str, destination: impl AsRef<Path>) -> Result<(), String> {
        let destination = destination.as_ref();

        let destination = if destination.is_absolute() {
            destination.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|error| format!("failed to resolve current directory: {error}"))?
                .join(destination)
        };

        if destination.exists() {
            fs::remove_file(&destination).map_err(|error| {
                format!(
                    "failed to remove existing destination {}: {error}",
                    destination.display(),
                )
            })?;
        }

        let destination_string = destination
            .to_str()
            .ok_or_else(|| "download destination is not valid UTF-8".to_string())?;

        let chunk_size = self.chunk_size.to_string();

        let download = self.call(&[
            "call",
            "storage_module",
            "downloadToUrl",
            cid,
            destination_string,
            "false",
            &chunk_size,
        ])?;

        let result = download
            .get("result")
            .ok_or_else(|| "logosctl response missing result".to_string())?;

        if !result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(format!(
                "Logos Storage download rejected: {}",
                result
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error"),
            ));
        }

        println!("Download accepted for CID: {cid}");

        for _ in 0..40 {
            if destination.exists() {
                let metadata = fs::metadata(&destination)
                    .map_err(|error| format!("failed to inspect downloaded file: {error}"))?;

                if metadata.len() > 0 {
                    return Ok(());
                }
            }

            thread::sleep(Duration::from_millis(250));
        }

        Err(format!(
            "download was accepted but file did not appear: {}",
            destination.display(),
        ))
    }

    fn wait_for_cid(&self, filename: &str) -> Result<String, String> {
        for _ in 0..20 {
            let manifests = self.call(&["call", "storage_module", "manifests"])?;

            if let Some(result) = manifests.get("result") {
                if result
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    if let Some(items) = result.get("value").and_then(Value::as_array) {
                        for item in items {
                            let matches_filename =
                                item.get("filename").and_then(Value::as_str) == Some(filename);

                            if matches_filename {
                                if let Some(cid) = item.get("cid").and_then(Value::as_str) {
                                    return Ok(cid.to_string());
                                }
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(250));
            }
        }
        Err(format!(
            "upload completed but CID was not found for {filename}"
        ))
    }

    fn call(&self, args: &[&str]) -> Result<Value, String> {
        let output = Command::new(&self.logosctl_bin)
            .args(args)
            .output()
            .map_err(|error| format!("failed to run logosctl: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "logosctl failed: {}",
                String::from_utf8_lossy(&output.stderr).trim(),
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("logosctl returned invalid UTF-8: {error}"))?;

        serde_json::from_str(stdout.trim()).map_err(|error| {
            format!("failed to decode logosctl response: {error}\nresponse: {stdout}")
        })
    }
}
