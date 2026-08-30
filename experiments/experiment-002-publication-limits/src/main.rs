use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::Instant,
};

use lb_core::mantle::ops::channel::ChannelId;
use lb_groth16::fr_from_bytes;
use lb_key_management_system_service::keys::{
    Ed25519Key,
    ZkPublicKey,
};
use lb_zone_sdk::{
    CommonHttpClient,
    adapter::NodeHttpClient,
    sequencer::{
        Event,
        FundingConfig,
        SequencerCheckpoint,
        ZoneSequencer,
    },
};
use reqwest::Url;
use lb_core::mantle::ops::channel::inscribe::Inscription;

const CHECKPOINT_PATH: &str = ".state/sequencer-checkpoint.json";
const RESULTS_PATH: &str = ".results/experiment-002.jsonl";

fn read_hex_32(path: &str) -> [u8; 32] {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("failed to read {}", path));

    let bytes = hex::decode(raw.trim())
        .unwrap_or_else(|_| panic!("invalid hex in {}", path));

    bytes
        .try_into()
        .unwrap_or_else(|_| panic!("{} must contain exactly 32 bytes", path))
}

fn load_checkpoint() -> Option<SequencerCheckpoint> {
    if !Path::new(CHECKPOINT_PATH).exists() {
        println!("No checkpoint found — starting cold backfill.");
        return None;
    }

    let data = fs::read(CHECKPOINT_PATH)
        .expect("failed to read checkpoint");

    let checkpoint: SequencerCheckpoint =
        serde_json::from_slice(&data)
            .expect("failed to decode checkpoint");

    println!(
        "✅ Loaded checkpoint from {}",
        CHECKPOINT_PATH
    );

    Some(checkpoint)
}

fn save_checkpoint(checkpoint: &SequencerCheckpoint) {
    fs::create_dir_all(".state")
        .expect("failed to create checkpoint directory");

    let data = serde_json::to_vec_pretty(checkpoint)
        .expect("failed to encode checkpoint");

    let temp_path = format!("{}.tmp", CHECKPOINT_PATH);

    fs::write(&temp_path, data)
        .expect("failed to write checkpoint");

    fs::rename(&temp_path, CHECKPOINT_PATH)
        .expect("failed to commit checkpoint");
}


fn save_result(
    payload_size: usize,
    tx_hash: String,
    l1_slot: Option<String>,
    mempool_seconds: Option<f64>,
    finalization_seconds: Option<f64>,
    status: &str,
) {
    fs::create_dir_all(".results")
        .expect("failed to create results directory");

    let result = serde_json::json!({
        "payload_size": payload_size,
        "tx_hash": tx_hash,
        "status": status,
        "l1_slot": l1_slot,
        "mempool_seconds": mempool_seconds,
        "finalization_seconds": finalization_seconds,
    });

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(RESULTS_PATH)
        .expect("failed to open results file");

    writeln!(file, "{}", result)
        .expect("failed to write experiment result");
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {


    tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
    )
    .init();
    let channel_bytes = read_hex_32(".secrets/channel_id");
    let sequencer_secret = read_hex_32(".secrets/sequencer_key");

    let channel_id = ChannelId::from(channel_bytes);
    let sequencer_key = Ed25519Key::from_bytes(&sequencer_secret);

    let funding_bytes: [u8; 32] = hex::decode(
        "a7f2f8669ab39b14959118bf157196987942cc69056b7455132e572716aee412",
    )?
    .try_into()
    .map_err(|_| "funding key must be 32 bytes")?;

    let funding_pk = ZkPublicKey::new(
        fr_from_bytes(&funding_bytes)?
    );

    let node = NodeHttpClient::new(
        CommonHttpClient::new(None),
        Url::parse("http://localhost:8080")?,
    );

    let funding = FundingConfig {
        funding_pk,
        max_tx_fee: 20_000_000.into(),
           priority_fee: FundingConfig::DEFAULT_PRIORITY_FEE,
    };

    // NEW: restore previous progress if available.
    let checkpoint = load_checkpoint();

    let mut sequencer = ZoneSequencer::init(
        channel_id,
        sequencer_key,
        node,
        funding,
        checkpoint,
    );

    println!("Connecting to Logos node...");

    while !sequencer.is_ready() {
        let event = sequencer.next_event().await;

        match &event {
            Event::BlocksProcessed {
                checkpoint,
                ..
            } => {
                save_checkpoint(checkpoint);

                println!(
                    "Backfill progress: {:?}",
                    checkpoint.lib_slot
                );
            }

            Event::Ready => {
                println!("✅ ZoneSequencer emitted Ready");
            }

            _ => {
                println!("Event: {:?}", event);
            }
        }
    }

    println!("✅ ZoneSequencer is READY");
    println!(
        "Channel ID: {}",
        hex::encode(channel_bytes)
    );

let args: Vec<String> = std::env::args().collect();

let payload_size: usize = args
    .windows(2)
    .find(|w| w[0] == "--payload-size")
    .and_then(|w| w[1].parse().ok())
    .unwrap_or(32);

    let mempool_only = args
    .iter()
    .any(|arg| arg == "--mempool-only");

if mempool_only {
    println!("🧪 Measurement mode: mempool-only");
}

   if payload_size > 1024 * 1024 {
    return Err(
        "payload size exceeds Experiment 002 maximum of 1 MiB".into()
    );
}

let payload = vec![0x42; payload_size];

let inscription =
    Inscription::new_unchecked(payload);

println!("Publishing test payload...");
println!("Payload size: {} bytes", payload_size);


let publish_started = Instant::now();

let (publish_result, publish_checkpoint) = sequencer
    .handle()
    .publish(inscription)
    .await?;

let published_tx_hash = publish_result.inscription_id();

// IMPORTANT:
// publish() mutates sequencer state immediately.
// Persist this checkpoint before doing anything else.
save_checkpoint(&publish_checkpoint);

println!("✅ Publish queued");
println!("Tx hash: {:?}", published_tx_hash);
println!("Waiting for mempool / finalization...");

let mut mempool_seconds: Option<f64> = None;

loop {
    let event = sequencer.next_event().await;

    match event {
        Event::MempoolPending(tx_hash) => {
            println!("📨 Mempool pending: {:?}", tx_hash);

            
  if tx_hash == published_tx_hash {
    let elapsed = publish_started.elapsed().as_secs_f64();

    mempool_seconds = Some(elapsed);

    println!("✅ test payload accepted by node / mempool");
    println!("⏱ Publish → mempool: {:.3}s", elapsed);

    if mempool_only {
        save_result(
            payload_size,
            format!("{:?}", tx_hash),
            None,
            Some(elapsed),
            None,
            "mempool_accepted",
        );

        println!("💾 Mempool result saved to {}", RESULTS_PATH);
        println!("ℹ️ Finalization intentionally not measured in this run.");

        return Ok(());
    }
}
        }

        Event::BlocksProcessed {
            checkpoint,
            channel_update,
            finalized,
        } => {
            // Persist every state-mutating checkpoint.
            save_checkpoint(&checkpoint);

            println!(
                "Block processed | LIB slot: {:?} | pending: {} | adopted: {} | orphaned: {}",
                checkpoint.lib_slot,
                checkpoint.pending_txs.len(),
                channel_update.adopted.len(),
                channel_update.orphaned.len(),
            );

            for tx in finalized {
                println!(
                    "Finalized tx: {:?} at L1 slot {:?}",
                    tx.tx_hash,
                    tx.l1_slot
                );

                if tx.tx_hash == published_tx_hash {
                    println!("🎉 test payload FINALIZED");
                    println!("Tx hash: {:?}", tx.tx_hash);
                    println!("L1 slot: {:?}", tx.l1_slot);
                    println!(
                        "Channel ID: {}",
                        hex::encode(channel_bytes)
                    );

                    let finalization_seconds =
    publish_started.elapsed().as_secs_f64();

    println!(
    "⏱ Publish → finalization: {:.3}s",
    finalization_seconds
);

save_result(
    payload_size,
    format!("{:?}", tx.tx_hash),
    Some(format!("{:?}", tx.l1_slot)),
    mempool_seconds,
    Some(finalization_seconds),
    "finalized",
);

println!("💾 Result saved to {}", RESULTS_PATH);

                    return Ok(());
                }
            }
        }

        Event::Ready => {
            println!("Sequencer ready");
        }

        Event::TurnNotification { notification } => {
            println!(
                "Turn update: our_turn={}",
                notification.our_turn_to_write
            );
        }
    }
}

}