use std::{
    fs,
    path::Path,
    time::Instant,
};

use sha3::{Digest, Keccak256};

use lb_core::mantle::ops::channel::{
    ChannelId,
    inscribe::Inscription,
};

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

const CHECKPOINT_PATH: &str =
    ".state/sequencer-checkpoint.json";

const BLOCK_PATH: &str =
    ".output/block-0.szb";

const HEADER_START: usize = 4;
const HEADER_SIZE: usize = 170;

fn read_hex_32(path: &str) -> [u8; 32] {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("failed to read {}", path));

    let bytes = hex::decode(raw.trim())
        .unwrap_or_else(|_| panic!("invalid hex in {}", path));

    bytes
        .try_into()
        .unwrap_or_else(|_| {
            panic!("{} must contain exactly 32 bytes", path)
        })
}

fn load_checkpoint() -> Option<SequencerCheckpoint> {
    if !Path::new(CHECKPOINT_PATH).exists() {
        println!("No checkpoint found — cold backfill.");
        return None;
    }

    let data = fs::read(CHECKPOINT_PATH)
        .expect("failed to read checkpoint");

    let checkpoint: SequencerCheckpoint =
        serde_json::from_slice(&data)
            .expect("failed to decode checkpoint");

    println!("✅ Loaded sequencer checkpoint");

    Some(checkpoint)
}

fn save_checkpoint(checkpoint: &SequencerCheckpoint) {
    fs::create_dir_all(".state")
        .expect("failed to create state directory");

    let data = serde_json::to_vec_pretty(checkpoint)
        .expect("failed to encode checkpoint");

    let temp_path =
        format!("{}.tmp", CHECKPOINT_PATH);

    fs::write(&temp_path, data)
        .expect("failed to write checkpoint");

    fs::rename(&temp_path, CHECKPOINT_PATH)
        .expect("failed to commit checkpoint");
}

fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();

    println!("=== Solizone Experiment 003 Publisher ===\n");

    let args: Vec<String> =
        std::env::args().collect();

    let send = args.iter().any(|arg| arg == "--send");

    /*
     * STEP 1
     * Load the exact canonical Solizone block.
     */
    let payload = fs::read(BLOCK_PATH)?;

    println!("Block artifact: {}", BLOCK_PATH);
    println!("Payload size: {} bytes", payload.len());

    /*
     * STEP 2
     * Validate outer Solizone block format.
     */
    if payload.len() < HEADER_START + HEADER_SIZE {
        return Err("Solizone block payload is too short".into());
    }

    if &payload[0..4] != b"SZB1" {
        return Err("invalid Solizone block magic".into());
    }

    println!("✅ Magic verified: SZB1");

    /*
     * STEP 3
     * Recompute canonical Solizone block hash.
     *
     * Our block hash is:
     *
     * Keccak256(canonical 170-byte header)
     */
    let header_end =
        HEADER_START + HEADER_SIZE;

    let block_hash =
        Keccak256::digest(&payload[HEADER_START..header_end]);

    println!(
        "Solizone Block #0 hash:\n0x{}\n",
        to_hex(&block_hash)
    );

    /*
     * STEP 4
     * Load Zone credentials.
     */
    let channel_bytes =
        read_hex_32(".secrets/channel_id");

    let sequencer_secret =
        read_hex_32(".secrets/sequencer_key");

    let channel_id =
        ChannelId::from(channel_bytes);

    let sequencer_key =
        Ed25519Key::from_bytes(&sequencer_secret);

    /*
     * Funding address.
     *
     * This is a public Logos wallet address,
     * not the sequencer private key.
     *
     * Can be changed without modifying code:
     *
     * --funding-address <hex>
     */
    let funding_hex = args
        .windows(2)
        .find(|w| w[0] == "--funding-address")
        .map(|w| w[1].as_str())
        .unwrap_or(
            "3d35166b34064b008ae1551b1dbb3cae1fb26dde4f3093516c161d7bd48e7a05"
        );

    let funding_bytes: [u8; 32] =
        hex::decode(funding_hex)?
            .try_into()
            .map_err(|_| {
                "funding address must be 32 bytes"
            })?;

    let funding_pk =
        ZkPublicKey::new(
            fr_from_bytes(&funding_bytes)?
        );

    println!(
        "Funding address: {}",
        funding_hex
    );

    /*
     * STEP 5
     * Connect to Logos node.
     */
    let node =
        NodeHttpClient::new(
            CommonHttpClient::new(None),
            Url::parse("http://localhost:8080")?,
        );

    /*
     * Block #0 is tiny (245 bytes), so we do not
     * need the huge Experiment 002 fee ceiling.
     */
    let funding =
        FundingConfig {
            funding_pk,
            max_tx_fee: 1_000_000.into(),
            priority_fee:
                FundingConfig::DEFAULT_PRIORITY_FEE,
        };

    let checkpoint =
        load_checkpoint();

    let mut sequencer =
        ZoneSequencer::init(
            channel_id,
            sequencer_key,
            node,
            funding,
            checkpoint,
        );

    println!("Connecting to Logos...\n");

    /*
     * STEP 6
     * Recover channel state and reach Ready.
     */
    while !sequencer.is_ready() {
        let event =
            sequencer.next_event().await;

        match &event {
            Event::BlocksProcessed {
                checkpoint,
                channel_update,
                ..
            } => {
                save_checkpoint(checkpoint);

                println!(
                    "BlocksProcessed | LIB {:?} | pending {} | adopted {} | orphaned {}",
                    checkpoint.lib_slot,
                    checkpoint.pending_txs.len(),
                    channel_update.adopted.len(),
                    channel_update.orphaned.len(),
                );
            }

            Event::Ready => {
                println!("✅ ZoneSequencer emitted Ready");
            }

            Event::MempoolPending(tx_hash) => {
                println!(
                    "Existing pending transaction: {:?}",
                    tx_hash
                );
            }

            Event::TurnNotification { notification } => {
                println!(
                    "Turn update: our_turn={}",
                    notification.our_turn_to_write
                );
            }
        }
    }

    println!("\n✅ ZoneSequencer READY");

    /*
     * Default behavior is intentionally safe.
     */
    if !send {
        println!();
        println!("🧪 DRY RUN COMPLETE");
        println!("No Logos transaction was created.");
        println!();
        println!("To publish Block #0:");
        println!("cargo run --bin publish -- --send");

        return Ok(());
    }

    /*
     * STEP 7
     * Publish the exact canonical block bytes.
     */
    println!("\nPublishing canonical Solizone Block #0...");

    let inscription =
        Inscription::new_unchecked(payload.clone());

    let started =
        Instant::now();

    let (publish_result, publish_checkpoint) =
        sequencer
            .handle()
            .publish(inscription)
            .await?;

    let published_tx_hash =
        publish_result.inscription_id();

    /*
     * publish() mutates sequencer state immediately.
     */
    save_checkpoint(&publish_checkpoint);

    println!("✅ Publication queued");
    println!(
        "Logos tx hash: {:?}",
        published_tx_hash
    );

    println!(
        "Canonical payload: {} bytes",
        payload.len()
    );

    /*
     * LIB is currently zero, therefore this
     * experiment intentionally stops at mempool
     * acceptance.
     */
    loop {
        let event =
            sequencer.next_event().await;

        match event {
            Event::MempoolPending(tx_hash) => {
                println!(
                    "📨 MempoolPending: {:?}",
                    tx_hash
                );

                if tx_hash == published_tx_hash {
                    let elapsed =
                        started.elapsed().as_secs_f64();

                    println!();
                    println!(
                        "✅ Solizone Block #0 accepted by Logos mempool"
                    );

                    println!(
                        "⏱ Publish → mempool: {:.3}s",
                        elapsed
                    );

                    println!(
                        "Solizone block hash:\n0x{}",
                        to_hex(&block_hash)
                    );

                    return Ok(());
                }
            }

            Event::BlocksProcessed {
                checkpoint,
                channel_update,
                ..
            } => {
                save_checkpoint(&checkpoint);

                println!(
                    "BlocksProcessed | LIB {:?} | pending {} | adopted {} | orphaned {}",
                    checkpoint.lib_slot,
                    checkpoint.pending_txs.len(),
                    channel_update.adopted.len(),
                    channel_update.orphaned.len(),
                );
            }

            Event::Ready => {}

            Event::TurnNotification { notification } => {
                println!(
                    "Turn update: our_turn={}",
                    notification.our_turn_to_write
                );
            }
        }
    }
}
