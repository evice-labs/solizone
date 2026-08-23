use std::{
    fs,
    path::Path,
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
    let data = serde_json::to_vec_pretty(checkpoint)
        .expect("failed to encode checkpoint");

    let temp_path = format!("{}.tmp", CHECKPOINT_PATH);

    fs::write(&temp_path, data)
        .expect("failed to write checkpoint");

    fs::rename(&temp_path, CHECKPOINT_PATH)
        .expect("failed to commit checkpoint");
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
        "08a9dee9b06a2ec6fae91af10ef96b965ed38714fbdd2c53667d2e0555a36b14",
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
        max_tx_fee: 1_000_000.into(),
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

 let inscription =
    Inscription::new_unchecked(b"hello-solizone".to_vec());

println!("Publishing hello-solizone...");

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

loop {
    let event = sequencer.next_event().await;

    match event {
        Event::MempoolPending(tx_hash) => {
            println!("📨 Mempool pending: {:?}", tx_hash);

            if tx_hash == published_tx_hash {
                println!("✅ hello-solizone accepted by node / mempool");
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
                    println!("🎉 hello-solizone FINALIZED");
                    println!("Tx hash: {:?}", tx.tx_hash);
                    println!("L1 slot: {:?}", tx.l1_slot);
                    println!(
                        "Channel ID: {}",
                        hex::encode(channel_bytes)
                    );

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