// std
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

// third party crates
use clap::Parser;
use copy_dir::copy_dir;
use glob::glob;
use parking_lot::RwLock;
use tokio::sync::broadcast;
// Massa crates
use massa_db_exports::MassaDBController;
use massa_execution_exports::ExecutionChannels;
use massa_execution_worker::start_execution_worker;
use massa_final_state::{FinalState, FinalStateController};
use massa_versioning::versioning::MipStore;
use massa_ledger_worker::FinalLedger;
use massa_metrics::MassaMetrics;
use massa_models::config::{CHAINID, THREAD_COUNT};
use massa_pos_worker::start_selector_worker;
use massa_time::MassaTime;
use massa_wallet::Wallet;

// Custom code
use crate::args::{Cli, Commands, ReplayArgs};
use crate::config::{
    get_db_config, get_execution_config,
    get_final_state_config, get_ledger_config,
    get_mip_stats_config, get_selector_config
};
use crate::wrapped_massa_db::WrappedMassaDB;

mod args;
mod config;
mod wrapped_massa_db;

fn main() {

    // Parse command line arguments
    let cli = Cli::parse();
    
    match cli.command {
        Commands::ListSnapshot => {
            list_snapshot(&cli.db_path)
        }
        Commands::Replay(replay_args) => {
            replay(&cli.db_path, &cli.initial_rolls_path, &replay_args)
        }
    }

}

fn replay(db_path: &PathBuf, initial_rolls_path: &PathBuf, replay_arg: &ReplayArgs) {

    // Setup
    // 1- Copy db backup path

    let temp_folder = tempfile::tempdir().unwrap();
    println!("Using temp folder: {:?}", temp_folder);
    
    let temp_folder_path = temp_folder.into_path();
    let db_temp_folder_path = temp_folder_path.join("db");
    let gas_costs_temp_folder_path = temp_folder_path.join("gas_costs");
    // std::fs::remove_dir(temp_folder_path.clone()).expect("Unable to remove temp dir");
    
    copy_dir(replay_arg.db_backup_path.clone(), db_temp_folder_path.clone()).expect("Unable to copy db backup path to temp dir");

    // 2- Copy additional files
    let gas_costs_folder = initial_rolls_path.parent().unwrap().join("gas_costs");
    copy_dir(gas_costs_folder, gas_costs_temp_folder_path.clone()).expect("Unable to copy gas costs folder to temp dir");
    
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
    let mip_store: MipStore = MipStore::try_from_db(db.clone(), mip_stats_config)
        .expect("MIP store creation failed");
    // println!("After read from db, Mip store: {:?}", mip_store);

    // POS - Selector
    let selector_config = get_selector_config();
    let (selector_manager, selector_controller) = start_selector_worker(selector_config)
        .expect("could not start selector worker");

    // TODO - FIXME
    let last_slot = db.read().get_change_id().unwrap();
    println!("Last slot: {}", last_slot);
    let final_state_config = get_final_state_config(initial_rolls_path.clone());

    let final_state: Arc<RwLock<dyn FinalStateController>> = Arc::new(RwLock::new(
        FinalState::new_derived_from_snapshot(
            db.clone(),
            final_state_config,
            Box::new(ledger),
            selector_controller.clone(),
            mip_store.clone(),
            last_slot.period,
        ).expect("could not init final state")
    ));

    println!("final_state: {:?}", final_state.read().get_database().read().get_xof_db_hash());

    // launch execution module

    let execution_config = get_execution_config(last_slot.period, &gas_costs_temp_folder_path);

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

    let node_wallet = Arc::new(
        RwLock::new(
            Wallet::new(
                PathBuf::from(PathBuf::from("config/staking_wallets")), // SETTINGS.factory.staking_wallet_path
                "1234".to_string(),
                *CHAINID,
            ).unwrap()
        )
    );

    let (massa_metrics, metrics_stopper) = MassaMetrics::new(
        false, // SETTINGS.metrics.enabled,
        SocketAddr::from_str( "[::]:31248").unwrap(), // SETTINGS.metrics.bind,
        THREAD_COUNT,
        MassaTime::from_millis(5000).to_duration(), // SETTINGS.metrics.tick_delay.to_duration(),
    );

    let (execution_manager, execution_controller) = start_execution_worker(
        execution_config,
        final_state.clone(),
        selector_controller.clone(),
        mip_store.clone(),
        execution_channels.clone(),
        node_wallet.clone(),
        massa_metrics.clone(),
    );
    
    println!("Execution manager & Execution controller done!");
}

fn list_snapshot(db_path: &PathBuf) {

    let pattern_ = db_path.join("backup_*_*");
    let pattern = pattern_
        .to_str()
        .unwrap();

    let glob_res = glob(pattern).expect("Failed to read glob pattern");
    let mut found = 0;

    for entry in glob_res {
        match entry {
            Ok(path) => {
                let db_config_from_backup = get_db_config(path.clone());
                let wrapped_db = WrappedMassaDB::new(db_config_from_backup, false);
                let db_slot = wrapped_db.0.get_change_id().unwrap();

                println!("Backup (path: {:?}): hash: {}, last slot: {}",
                         path.display(),
                         wrapped_db.0.get_xof_db_hash(),
                         db_slot
                );

                found += 1;
            },
            Err(e) => {
                println!("Error: {:?}", e);
            },
        }
    }

    if found == 0 {
        println!("Cannot find any backup with pattern: {:?}", pattern);
    }
}
