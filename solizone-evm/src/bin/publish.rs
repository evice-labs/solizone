use std::{fs, path::Path, time::Instant};

use lb_groth16::fr_from_bytes;

use lb_key_management_system_service::keys::{Ed25519Key, ZkPublicKey};

use lb_zone_sdk::{
    CommonHttpClient,
    adapter::NodeHttpClient,
    node_types::{ChannelId, Inscription},
    sequencer::{Event, FundingConfig, SequencerCheckpoint, ZoneSequencer},
};

use revm::primitives::{B256, U256, address};

use solizone_evm::{
    block_builder::build_block, execution::RevmExecutionEngine,
    publisher::prepare_block_for_publication,
};

const CHANNEL_ID_PATH: &str = "../experiments/experiment-003-canonical-block/.secrets/channel_id";

const SEQUENCER_KEY_PATH: &str =
    "../experiments/experiment-003-canonical-block/.secrets/sequencer_key";

const CHECKPOINT_PATH: &str = ".state/sequencer-checkpoint.json";

const LEGACY_CHECKPOINT_PATH: &str =
    "../experiments/experiment-003-canonical-block/.state/sequencer-checkpoint.json";

/*
 * Public Logos wallet address.
 *
 * This is NOT a private key.
 *
 * It matches the funding wallet currently configured
 * in the Basecamp node's SDP wallet configuration.
 */
const FUNDING_ADDRESS: &str = "ae5a6c8a01b95f2cc40f596882b3f7b03e02b6a37d4376bc43bda47369ccbe2a";

fn read_hex_32(path: &str) -> [u8; 32] {
    let raw = fs::read_to_string(path).unwrap_or_else(|_| panic!("failed to read {}", path));

    let value = raw.trim();

    let value = value.strip_prefix("0x").unwrap_or(value);

    let bytes = hex::decode(value).unwrap_or_else(|_| panic!("invalid hex in {}", path));

    bytes
        .try_into()
        .unwrap_or_else(|_| panic!("{} must contain exactly 32 bytes", path))
}

fn try_load_checkpoint(path: &str) -> Option<SequencerCheckpoint> {
    if !Path::new(path).exists() {
        return None;
    }

    let data = fs::read(path).ok()?;

    match serde_json::from_slice::<SequencerCheckpoint>(&data) {
        Ok(checkpoint) => {
            println!("Loaded sequencer checkpoint from {}", path);

            Some(checkpoint)
        }

        Err(error) => {
            println!("Checkpoint at {} is incompatible: {}", path, error);

            None
        }
    }
}

fn load_checkpoint() -> Option<SequencerCheckpoint> {
    /*
     * Prefer Solizone EVM's own checkpoint.
     */
    if let Some(checkpoint) = try_load_checkpoint(CHECKPOINT_PATH) {
        return Some(checkpoint);
    }

    /*
     * On first run only, fall back to the
     * checkpoint created by Experiment 003.
     */
    if let Some(checkpoint) = try_load_checkpoint(LEGACY_CHECKPOINT_PATH) {
        println!("Using Experiment 003 checkpoint as starting state.");

        return Some(checkpoint);
    }

    println!("No compatible checkpoint found — cold backfill.");

    None
}

fn save_checkpoint(checkpoint: &SequencerCheckpoint) {
    fs::create_dir_all(".state").expect("failed to create .state");

    let data = serde_json::to_vec_pretty(checkpoint).expect("failed to encode checkpoint");

    let temp = format!("{}.tmp", CHECKPOINT_PATH);

    fs::write(&temp, data).expect("failed to write checkpoint");

    fs::rename(&temp, CHECKPOINT_PATH).expect("failed to commit checkpoint");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /*
     * Enable Logos SDK warnings/errors.
     *
     * Run with:
     *
     * RUST_LOG=warn cargo run --bin publish -- --resume
     *
     * or:
     *
     * RUST_LOG=debug cargo run --bin publish -- --resume
     */
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    println!("=== Solizone Block Publisher ===");
    println!("Building execution-backed Solizone block...\n");

    /*
     * ============================================================
     * SOLIZONE EXECUTION
     * ============================================================
     */

    let engine = RevmExecutionEngine::new();

    let deployer = address!("1111111111111111111111111111111111111111");

    let outcome = engine.deploy_increment_and_read_counter(deployer, U256::from(10_000_000u64));

    let receipts = vec![
        outcome.deploy_receipt.clone(),
        outcome.increment_receipt.clone(),
    ];

    let block = build_block(
        1,
        9001,
        0,
        B256::ZERO,
        1_800_000_000,
        outcome.state_root,
        &outcome.transactions,
        &receipts,
        30_000_000,
    );

    block
        .validate()
        .expect("generated Solizone block is invalid");

    let publication = prepare_block_for_publication(&block);

    println!();
    println!("✅ Solizone block built successfully");
    println!("Height:        {}", block.header.height);
    println!("Transactions:  {}", block.transactions.len());
    println!("Gas used:      {}", block.header.gas_used);
    println!("Payload size:  {} bytes", publication.bytes.len());
    println!("Block hash:    {}", publication.block_hash);

    /*
     * ============================================================
     * CLI MODES
     * ============================================================
     *
     * no args:
     *      build block locally only
     *
     * --connect:
     *      connect, recover/backfill, receive one live checkpoint
     *      and exit
     *
     * --resume:
     *      restore an already-pending Logos transaction from the
     *      checkpoint and keep driving the ZoneSequencer until it
     *      reaches the Logos mempool
     *
     * --send:
     *      create ONE new Logos inscription transaction and keep
     *      driving until that exact tx reaches MempoolPending
     */

    let args: Vec<String> = std::env::args().collect();

    let send = args.iter().any(|arg| arg == "--send");

    let resume = args.iter().any(|arg| arg == "--resume");

    let explicit_connect = args.iter().any(|arg| arg == "--connect");

    if send && resume {
        return Err("use either --send or --resume, not both".into());
    }

    let connect = explicit_connect || send || resume;

    if !connect {
        println!();
        println!("DRY RUN ONLY");
        println!("No Logos connection was opened.");

        return Ok(());
    }

    /*
     * ============================================================
     * LOGOS CREDENTIALS
     * ============================================================
     */

    println!();
    println!("=== Connecting to Logos Basecamp node ===");

    let channel_bytes = read_hex_32(CHANNEL_ID_PATH);

    let sequencer_secret = read_hex_32(SEQUENCER_KEY_PATH);

    let channel_id = ChannelId::from(channel_bytes);

    let sequencer_key = Ed25519Key::from_bytes(&sequencer_secret);

    /*
     * Funding wallet.
     */
    let funding_bytes: [u8; 32] = hex::decode(FUNDING_ADDRESS)?
        .try_into()
        .map_err(|_| "funding address must be 32 bytes")?;

    let funding_pk = ZkPublicKey::new(fr_from_bytes(&funding_bytes)?);

    let funding = FundingConfig {
        funding_pk,

        /*
         * Keep the conservative Solizone ceiling
         * instead of the node's unlimited cap.
         */
        max_tx_fee: 1_000_000.into(),

        priority_fee_percent: FundingConfig::DEFAULT_PRIORITY_FEE_PERCENT,
    };

    /*
     * ============================================================
     * LOGOS NODE
     * ============================================================
     */

    let node = NodeHttpClient::new(
        CommonHttpClient::new(None),
        "http://127.0.0.1:8080".parse()?,
    );

    /*
     * Capture pending transaction hashes BEFORE passing the
     * checkpoint into ZoneSequencer.
     *
     * This lets --resume identify the exact transaction(s)
     * restored from disk.
     */
    let checkpoint = load_checkpoint();

    let restored_pending_hashes = checkpoint
        .as_ref()
        .map(|checkpoint| {
            checkpoint
                .pending_txs
                .iter()
                .map(|(tx_hash, _)| tx_hash.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if restored_pending_hashes.is_empty() {
        println!("Checkpoint pending transactions: 0");
    } else {
        println!(
            "Checkpoint pending transactions: {}",
            restored_pending_hashes.len()
        );

        for tx_hash in &restored_pending_hashes {
            println!("  pending: {:?}", tx_hash);
        }
    }

    /*
     * Protect against accidentally publishing a second block
     * while an older publication is still pending.
     */
    if send && !restored_pending_hashes.is_empty() {
        return Err(format!(
            "checkpoint already contains {} pending transaction(s); \
                 refusing to create another publication. \
                 Run with --resume first.",
            restored_pending_hashes.len()
        )
        .into());
    }

    let mut sequencer = ZoneSequencer::init(channel_id, sequencer_key, node, funding, checkpoint);

    println!("Driving ZoneSequencer...");

    /*
     * ============================================================
     * READY / BACKFILL
     * ============================================================
     *
     * next_event() is the Zone SDK drive loop.
     *
     * It drives:
     *
     * - connection
     * - backfill
     * - live blocks
     * - pending tx resubmission
     * - in-flight HTTP post_transaction futures
     */

    while !sequencer.is_ready() {
        match sequencer.next_event().await {
            Event::BlocksProcessed { checkpoint, .. } => {
                let slot: u64 = checkpoint.lib_slot.into();

                println!("Backfill progress: LIB slot {}", slot);

                save_checkpoint(&checkpoint);
            }

            Event::Ready => {
                println!("✅ ZoneSequencer Ready");
            }

            Event::MempoolPending(tx_hash) => {
                println!(
                    "Existing transaction reached mempool during recovery: {:?}",
                    tx_hash
                );
            }

            Event::TurnNotification { notification } => {
                println!("Turn update: our_turn={}", notification.our_turn_to_write);
            }
        }
    }

    /*
     * Print the current Zone channel state.
     */
    let channel_view_rx = sequencer.subscribe_channel_view();

    {
        let view = channel_view_rx.borrow().clone();

        println!("=== Channel View ===");

        println!("current_slot:          {:?}", view.current_slot);

        println!("own_key_index:         {:?}", view.own_key_index);

        println!("authorized_key_index:  {:?}", view.authorized_key_index);

        println!("our_turn_to_write:     {}", view.our_turn_to_write);

        println!("accredited_key_count:  {:?}", view.accredited_key_count);

        println!("pending_publish_txs:   {}", view.pending_publish_txs);
    }

    /*
     * ============================================================
     * CONNECT-ONLY MODE
     * ============================================================
     */

    if explicit_connect && !send && !resume {
        println!("Waiting for one live Logos checkpoint...");

        loop {
            match sequencer.next_event().await {
                Event::BlocksProcessed { checkpoint, .. } => {
                    let slot: u64 = checkpoint.lib_slot.into();

                    save_checkpoint(&checkpoint);

                    println!("✅ Live checkpoint received at LIB slot {}", slot);

                    break;
                }

                Event::MempoolPending(tx_hash) => {
                    println!("Mempool pending: {:?}", tx_hash);
                }

                Event::Ready => {}

                Event::TurnNotification { notification } => {
                    println!("Turn update: our_turn={}", notification.our_turn_to_write);
                }
            }
        }

        println!();
        println!("Solizone successfully connected to the Logos node.");
        println!("NO new block was published.");

        return Ok(());
    }

    /*
     * ============================================================
     * RESUME MODE
     * ============================================================
     *
     * This is the mode you should use RIGHT NOW because the
     * checkpoint already contains:
     *
     * d8c72874...
     *
     * No new inscription is created here.
     */

    if resume {
        if restored_pending_hashes.is_empty() {
            println!("No pending Logos transactions are stored in the checkpoint.");

            return Ok(());
        }

        println!();
        println!("=== Resuming pending Logos publication ===");

        println!("Waiting for restored transaction to reach mempool...");

        loop {
            match sequencer.next_event().await {
                Event::MempoolPending(tx_hash) => {
                    println!("📨 Logos mempool pending: {:?}", tx_hash);

                    if restored_pending_hashes.contains(&tx_hash) {
                        println!();
                        println!("✅ Restored Solizone publication accepted by Logos mempool");

                        println!("Logos tx: {:?}", tx_hash);

                        return Ok(());
                    }
                }

                Event::BlocksProcessed { checkpoint, .. } => {
                    save_checkpoint(&checkpoint);

                    let slot: u64 = checkpoint.lib_slot.into();

                    let view = channel_view_rx.borrow().clone();

                    println!(
                        "Live checkpoint | LIB {} | pending {} | our_turn={}",
                        slot,
                        checkpoint.pending_txs.len(),
                        view.our_turn_to_write
                    );
                }

                Event::Ready => {
                    println!("ZoneSequencer reconnected");
                }

                Event::TurnNotification { notification } => {
                    println!(
                        "Turn update: our_turn={} current={:?} start={:?} end={:?}",
                        notification.our_turn_to_write,
                        notification.current_slot,
                        notification.starting_slot,
                        notification.ends_at_slot,
                    );
                }
            }
        }
    }

    /*
     * ============================================================
     * SEND MODE
     * ============================================================
     *
     * This deliberately follows the same pattern that worked
     * in Experiment 003:
     *
     * handle().publish()
     *        ↓
     * save checkpoint
     *        ↓
     * keep calling next_event()
     *        ↓
     * wait for THIS tx's MempoolPending
     */

    if send {
        println!();
        println!("=== Publishing Solizone block ===");

        let inscription = Inscription::try_from(publication.bytes.clone())
            .map_err(|_| "Solizone block exceeds Logos inscription limit")?;

        let started = Instant::now();

        /*
         * IMPORTANT:
         *
         * handle().publish() creates/funds/signs the tx,
         * records it in sequencer pending state, and queues
         * the actual network POST.
         *
         * It does NOT mean the node has accepted it yet.
         */
        let (publish_result, publish_checkpoint) = sequencer.handle().publish(inscription).await?;

        let published_tx_hash = publish_result.inscription_id();

        /*
         * Persist the local pending state immediately.
         */
        save_checkpoint(&publish_checkpoint);

        println!("✅ Block accepted locally by Logos ZoneSequencer");

        println!("Solizone block hash: {}", publication.block_hash);

        println!("Logos inscription/tx id: {:?}", published_tx_hash);

        println!("Status: accepted locally / queued");

        println!("Waiting for Logos mempool acceptance...");

        /*
         * THIS LOOP IS CRITICAL.
         *
         * next_event() drives the SDK's queued
         * post_transaction future.
         *
         * Do not exit after handle().publish().
         */
        loop {
            match sequencer.next_event().await {
                Event::MempoolPending(tx_hash) => {
                    println!("📨 Logos mempool pending: {:?}", tx_hash);

                    if tx_hash == published_tx_hash {
                        let elapsed = started.elapsed().as_secs_f64();

                        println!();
                        println!("✅ Solizone block accepted by Logos mempool");

                        println!("Solizone block hash: {}", publication.block_hash);

                        println!("Logos tx hash: {:?}", tx_hash);

                        println!("Publish → mempool: {:.3}s", elapsed);

                        return Ok(());
                    }
                }

                Event::BlocksProcessed { checkpoint, .. } => {
                    save_checkpoint(&checkpoint);

                    let slot: u64 = checkpoint.lib_slot.into();

                    let view = channel_view_rx.borrow().clone();

                    println!(
                        "Live checkpoint | LIB {} | pending {} | our_turn={}",
                        slot,
                        checkpoint.pending_txs.len(),
                        view.our_turn_to_write
                    );
                }

                Event::Ready => {
                    println!("ZoneSequencer reconnected");
                }

                Event::TurnNotification { notification } => {
                    println!(
                        "Turn update: our_turn={} current={:?} start={:?} end={:?}",
                        notification.our_turn_to_write,
                        notification.current_slot,
                        notification.starting_slot,
                        notification.ends_at_slot,
                    );
                }
            }
        }
    }

    Ok(())
}
