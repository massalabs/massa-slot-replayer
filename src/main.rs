// std
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, path::Path, path::PathBuf, thread};

// third party crates
use cfg_if::cfg_if;
use clap::Parser;
use copy_dir::copy_dir;
use glob::glob;
use parking_lot::RwLock;
use prost::Message;
#[cfg(feature = "db_storage_backend")]
use rocksdb::{IteratorMode, DB};
use tokio::sync::broadcast;

// Massa crates
use massa_db_exports::MassaDBController;
use massa_execution_exports::{ExecutionBlockMetadata, ExecutionChannels};
use massa_execution_worker::start_execution_worker;
#[cfg(feature = "file_storage_backend")]
use massa_execution_worker::storage_backend::{FileStorageBackend, StorageBackend};
#[cfg(feature = "db_storage_backend")]
use massa_execution_worker::storage_backend::{RocksDBStorageBackend, StorageBackend};
use massa_final_state::{FinalState, FinalStateController};
use massa_ledger_worker::FinalLedger;
use massa_metrics::MassaMetrics;
#[cfg(feature = "db_storage_backend")]
use massa_models::slot::SLOT_KEY_SIZE;
use massa_models::{
    block::SecureShareBlock,
    config::{CHAINID, THREAD_COUNT},
    prehash::PreHashMap,
    slot::Slot,
};
use massa_pos_worker::start_selector_worker;
use massa_proto_rs::massa::model::v1::{self as grpc_model};
use massa_storage::Storage;
use massa_time::MassaTime;
use massa_versioning::versioning::MipStore;
use massa_wallet::Wallet;
use tracing::metadata::LevelFilter;
use tracing::{info, trace, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// Custom code
use crate::args::{Cli, Commands, ReplayArgs};
use crate::config::{
    get_db_config, get_execution_config, get_final_state_config, get_ledger_config,
    get_mip_stats_config, get_selector_config,
};
use crate::grpc_conv::{
    address_from_str, secure_share_block_from_filled_block,
    secure_shared_operations_from_filled_operation_entries,
};
use crate::wrapped_massa_db::WrappedMassaDB;

mod args;
mod block_storer;
mod config;
mod grpc_conv;
mod wrapped_massa_db;

const EXECUTION_TIME_PER_BLOCK_IN_S: u64 = 2;

fn main() {
    // init env
    let tracing_layer = LevelFilter::ERROR;
    tracing_subscriber::registry().with(tracing_layer).init();

    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::ListSnapshot => list_snapshot(&cli.db_path),
        Commands::Replay(replay_args) => replay(&cli.initial_rolls_path, &replay_args),
    }
}

fn replay(initial_rolls_path: &Path, replay_arg: &ReplayArgs) {
    // Setup
    // 1- Copy db backup path

    let temp_folder = tempfile::tempdir().unwrap();
    println!("Using temp folder: {:?}", temp_folder);

    let temp_folder_path = temp_folder.into_path();
    let db_temp_folder_path = temp_folder_path.join("db");
    let gas_costs_temp_folder_path = temp_folder_path.join("gas_costs");

    copy_dir(
        replay_arg.db_backup_path.clone(),
        db_temp_folder_path.clone(),
    )
    .expect("Unable to copy db backup path to temp dir");

    // 2- Copy additional files
    let gas_costs_folder = initial_rolls_path.parent().unwrap().join("gas_costs");
    copy_dir(gas_costs_folder, gas_costs_temp_folder_path.clone())
        .expect("Unable to copy gas costs folder to temp dir");

    // DB
    println!("Init db from: {:?}", db_temp_folder_path);
    let db_config = get_db_config(db_temp_folder_path);
    let wrapped_db = WrappedMassaDB::new(db_config, false);
    let db = Arc::new(RwLock::new(
        Box::new(wrapped_db.0) as Box<(dyn MassaDBController + 'static)>
    ));

    // Ledger
    let ledger_config = get_ledger_config();
    let ledger = FinalLedger::new(ledger_config, db.clone());

    // Versioning - MIP Store
    let mip_stats_config = get_mip_stats_config();
    let mip_store: MipStore =
        MipStore::try_from_db(db.clone(), mip_stats_config).expect("MIP store creation failed");

    // POS - Selector
    let selector_config = get_selector_config();
    let (_selector_manager, selector_controller) =
        start_selector_worker(selector_config).expect("could not start selector worker");

    let db_snapshot_last_slot = db.read().get_change_id().unwrap();
    println!("Last slot: {}", db_snapshot_last_slot);

    let final_state_config = get_final_state_config(initial_rolls_path.to_path_buf());

    let final_state: Arc<RwLock<dyn FinalStateController>> = Arc::new(RwLock::new(
        FinalState::new_derived_from_snapshot(
            db.clone(),
            final_state_config,
            Box::new(ledger),
            selector_controller.clone(),
            mip_store.clone(),
            db_snapshot_last_slot.period,
        )
        .expect("could not init final state"),
    ));

    println!(
        "final_state: {}",
        final_state.read().get_database().read().get_xof_db_hash()
    );

    // launch execution module

    let execution_config =
        get_execution_config(db_snapshot_last_slot.period, &gas_costs_temp_folder_path);

    let execution_channels = ExecutionChannels {
        slot_execution_output_sender: broadcast::channel(
            execution_config.broadcast_slot_execution_output_channel_capacity,
        )
        .0,
        #[cfg(feature = "execution-trace")]
        slot_execution_traces_sender: broadcast::channel(
            execution_config.broadcast_slot_execution_traces_channel_capacity,
        )
        .0,
    };

    let node_wallet = Arc::new(RwLock::new(
        Wallet::new(
            PathBuf::from("config/staking_wallets"), // SETTINGS.factory.staking_wallet_path
            "1234".to_string(),
            *CHAINID,
        )
        .unwrap(),
    ));

    let (massa_metrics, _metrics_stopper) = MassaMetrics::new(
        false,                                       // SETTINGS.metrics.enabled,
        SocketAddr::from_str("[::]:31248").unwrap(), // SETTINGS.metrics.bind,
        THREAD_COUNT,
        MassaTime::from_millis(5000).to_duration(), // SETTINGS.metrics.tick_delay.to_duration(),
    );

    let (_execution_manager, execution_controller) = start_execution_worker(
        execution_config,
        final_state.clone(),
        selector_controller.clone(),
        mip_store.clone(),
        execution_channels.clone(),
        node_wallet.clone(),
        massa_metrics.clone(),
    );

    println!("Execution manager & Execution controller done!");

    let dumped_slots = list_dumped_blocks(&replay_arg.dump_block_path);
    let first_slot = dumped_slots.iter().min().unwrap();
    let last_slot = dumped_slots.iter().max().unwrap();
    println!("first block in dumped block pool {:?}", first_slot);
    println!("last block in dumped block pool {:?}", last_slot);

    let mut slot = db_snapshot_last_slot;

    cfg_if! {
        if #[cfg(feature = "db_storage_backend")] {
            let block_db =
                RocksDBStorageBackend::new(
                replay_arg.dump_block_path.clone())
            ;
        } else if #[cfg(feature = "file_storage_backend")] {
            let block_db =
                FileStorageBackend::new(
                replay_arg.dump_block_path.clone())
            ;
        } else  {
            compile_error!("Slot replayer binary require either feature `db_storage_backend` or feature `file_storage_backend`");
        }
    }

    // let blocks = block_storer::fetch_block_from_node_storer();

    while let Ok(next_slot) = slot.get_next_slot(THREAD_COUNT) {
        info!("next_slot: {}", next_slot);

        if let Some(until_slot) = replay_arg.until_slot {
            if next_slot > Slot::new(until_slot.0, until_slot.1) {
                println!("Until slot reached, exiting now...");
                break;
            }
        }

        if next_slot > *last_slot {
            println!("Last slot reached, exiting now...");
            break;
        }
        trace!("Read dumped block - next_slot: {:?}", next_slot);

        let slot_same_thread_parent = Slot::new(next_slot.period - 1, next_slot.thread);

        let dumped_block_content_ = block_db.read(&next_slot);
        let dumped_block_parent_ = block_db.read(&slot_same_thread_parent).unwrap(); // FIXME

        match dumped_block_content_ {
            Some(dump_block_content) => {
                let filled_block =
                    grpc_model::FilledBlock::decode(&dump_block_content[..]).unwrap();
                let mut storage = Storage::create_root();

                let operations = secure_shared_operations_from_filled_operation_entries(
                    &filled_block.operations,
                );
                info!("Find {} operations", operations.len());
                storage.store_operations(operations);

                let block: SecureShareBlock = secure_share_block_from_filled_block(filled_block);
                trace!("add block id: {} in storage...", block.id);

                let block_id = block.id.clone();
                storage.store_block(block);

                let finalized_blocks = HashMap::from([(next_slot, block_id)]);

                let filled_block_parent =
                    grpc_model::FilledBlock::decode(&dumped_block_parent_[..]).unwrap();

                let execution_block_metadata = ExecutionBlockMetadata {
                    same_thread_parent_creator: Some(address_from_str(
                        &filled_block_parent.header.unwrap().content_creator_address,
                    )),
                    storage: Some(storage),
                };

                let mut block_metadata = PreHashMap::default();
                block_metadata.insert(block_id, execution_block_metadata);

                execution_controller.update_blockclique_status(
                    finalized_blocks,
                    None,
                    block_metadata,
                );
            }
            None => {
                warn!("Unable to read dumped block for slot: {}", next_slot);
                break;
            }
        }

        slot = next_slot;
    }

    trace!("End of while loop...");

    let d = Duration::from_secs(
        (last_slot.period - db_snapshot_last_slot.period + 1) * EXECUTION_TIME_PER_BLOCK_IN_S,
    );
    println!(
        "Waiting {} seconds for the slot execution to complete
    (depending on your computeur you may need to adjust this value)",
        d.as_secs()
    );
    thread::sleep(d);
}

#[cfg(feature = "db_storage_backend")]
fn list_dumped_blocks_db(path: &PathBuf) -> Vec<Slot> {
    let opts = rocksdb::Options::default();

    let blocks_db = DB::open_for_read_only(&opts, path, true)
        .unwrap_or_else(|_| panic!("failed to open db at {:?}", path));
    blocks_db
        .iterator(IteratorMode::Start)
        .map(|x| match x {
            Ok((k, _v)) => {
                let buffer: &[u8; SLOT_KEY_SIZE] = &k.to_vec().try_into().unwrap();
                Slot::from_bytes_key(buffer)
            }
            Err(e) => panic!("{}", e),
        })
        .collect()
}

#[cfg(feature = "file_storage_backend")]
fn list_dumped_blocks_file(path: &PathBuf) -> Vec<Slot> {
    use std::fs::read_dir;
    let dir = read_dir(path).unwrap();
    let slots: Vec<_> = dir
        .map(|e| {
            let file_name = &e.unwrap().file_name();
            let file_name = file_name.to_str().unwrap();
            let slot: Vec<_> = file_name.split('_').collect();
            let thread = slot.get(2).unwrap().parse::<u8>().unwrap();
            let period = slot
                .get(3)
                .unwrap()
                .split('.')
                .nth(0)
                .unwrap()
                .parse::<u64>()
                .unwrap();
            Slot::new(period, thread)
        })
        .collect();

    slots
}

fn list_dumped_blocks(path: &PathBuf) -> Vec<Slot> {
    cfg_if! {
        if #[cfg(feature = "db_storage_backend")] {
            list_dumped_blocks_db(path)
        }
        else if #[cfg(feature = "file_storage_backend")] {
            list_dumped_blocks_file(path)
        } else  {
            compile_error!("Requise either feature db_storage_backend or feature  file_storage_backend");
        }
    }
}

fn list_snapshot(db_path: &Path) {
    let pattern_ = db_path.join("backup_*_*");
    let pattern = pattern_.to_str().unwrap();

    let glob_res = glob(pattern).expect("Failed to read glob pattern");
    let mut found = 0;

    for entry in glob_res {
        match entry {
            Ok(path) => {
                let db_config_from_backup = get_db_config(path.clone());
                let wrapped_db = WrappedMassaDB::new(db_config_from_backup, false);
                let db_slot = wrapped_db.0.get_change_id().unwrap();

                println!(
                    "Backup (path: {:?}): hash: {}, last slot: {}",
                    path.display(),
                    wrapped_db.0.get_xof_db_hash(),
                    db_slot
                );

                found += 1;
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    if found == 0 {
        println!("Cannot find any backup with pattern: {:?}", pattern);
    }
}
