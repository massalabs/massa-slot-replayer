// std lib
use std::path::{Path, PathBuf};

use massa_async_pool::AsyncPoolConfig;
use massa_db_exports::MassaDBConfig;
use massa_executed_ops::{ExecutedDenunciationsConfig, ExecutedOpsConfig};
use massa_execution_exports::{ExecutionConfig, GasCosts, StorageCostsConstants};
use massa_final_state::FinalStateConfig;
use massa_ledger_exports::LedgerConfig;
use massa_models::address::Address;
use massa_models::config::{DENUNCIATION_EXPIRE_PERIODS, ENDORSEMENT_COUNT, GENESIS_TIMESTAMP, INITIAL_DRAW_SEED, MAX_ASYNC_POOL_LENGTH, MAX_BOOTSTRAP_FINAL_STATE_PARTS_SIZE, MAX_BOOTSTRAP_VERSIONING_ELEMENTS_SIZE, MAX_DATASTORE_KEY_LENGTH, MAX_DATASTORE_VALUE_LENGTH, MAX_DEFERRED_CREDITS_LENGTH, MAX_DENUNCIATIONS_PER_BLOCK_HEADER, MAX_DENUNCIATION_CHANGES_LENGTH, MAX_FUNCTION_NAME_LENGTH, MAX_PARAMETERS_SIZE, MAX_PRODUCTION_STATS_LENGTH, MAX_ROLLS_COUNT_LENGTH, MIP_STORE_STATS_BLOCK_CONSIDERED, PERIODS_PER_CYCLE, POS_SAVED_CYCLES, T0, THREAD_COUNT, SELECTOR_DRAW_CACHE_SIZE, CHANNEL_SIZE, GENESIS_KEY, MAX_ASYNC_GAS, ASYNC_MSG_CST_GAS_COST, MAX_GAS_PER_BLOCK, ROLL_PRICE, BLOCK_REWARD, OPERATION_VALIDITY_PERIODS, POS_MISS_RATE_DEACTIVATION_THRESHOLD, MAX_BYTECODE_LENGTH, BASE_OPERATION_GAS_COST, ROLL_COUNT_TO_SLASH_ON_DENUNCIATION, MAX_EVENT_DATA_SIZE, CHAINID, LEDGER_COST_PER_BYTE, LEDGER_ENTRY_BASE_COST, LEDGER_ENTRY_DATASTORE_BASE_SIZE};
use massa_pos_exports::{PoSConfig, SelectorConfig};
use massa_time::MassaTime;
use massa_versioning::versioning::MipStatsConfig;
use num::rational::Ratio;

pub fn get_db_config(path: PathBuf) -> MassaDBConfig {
    MassaDBConfig {
        path,
        max_history_length: 100,
        max_versioning_elements_size: MAX_BOOTSTRAP_VERSIONING_ELEMENTS_SIZE as usize,
        max_final_state_elements_size: MAX_BOOTSTRAP_FINAL_STATE_PARTS_SIZE as usize,
        thread_count: THREAD_COUNT,
    }
}

pub fn get_ledger_config() -> LedgerConfig {
    LedgerConfig {
        thread_count: THREAD_COUNT,
        initial_ledger_path: PathBuf::new(),
        max_key_length: MAX_DATASTORE_KEY_LENGTH,
        max_datastore_value_length: MAX_DATASTORE_VALUE_LENGTH,
    }
}

fn get_async_pool_config() -> AsyncPoolConfig {
    AsyncPoolConfig {
        max_length: MAX_ASYNC_POOL_LENGTH,
        max_function_length: MAX_FUNCTION_NAME_LENGTH,
        max_function_params_length: MAX_PARAMETERS_SIZE as u64,
        thread_count: THREAD_COUNT,
        max_key_length: MAX_DATASTORE_KEY_LENGTH as u32,
    }
}

fn get_pos_config() -> PoSConfig {
    PoSConfig {
        initial_deferred_credits_path: Some(PathBuf::new()),
        periods_per_cycle: PERIODS_PER_CYCLE,
        thread_count: THREAD_COUNT,
        cycle_history_length: POS_SAVED_CYCLES,
        max_rolls_length: MAX_ROLLS_COUNT_LENGTH,
        max_production_stats_length: MAX_PRODUCTION_STATS_LENGTH,
        max_credit_length: MAX_DEFERRED_CREDITS_LENGTH,
    }
}

fn get_executed_ops_config() -> ExecutedOpsConfig {
    ExecutedOpsConfig {
        keep_executed_history_extra_periods: 1,
        thread_count: THREAD_COUNT,
    }
}

fn get_executed_denunciations_config() -> ExecutedDenunciationsConfig {
    ExecutedDenunciationsConfig {
        keep_executed_history_extra_periods: 1,
        denunciation_expire_periods: DENUNCIATION_EXPIRE_PERIODS,
        thread_count: THREAD_COUNT,
        endorsement_count: ENDORSEMENT_COUNT,
    }
}

pub fn get_mip_stats_config() -> MipStatsConfig {
    MipStatsConfig {
        block_count_considered: MIP_STORE_STATS_BLOCK_CONSIDERED,
        warn_announced_version_ratio: Ratio::new(3, 10),
    }
}

pub fn get_final_state_config(
    // path: PathBuf,
    initial_rolls_path: PathBuf,
) -> FinalStateConfig {
    let ledger_config = get_ledger_config();
    let async_pool_config = get_async_pool_config();
    let pos_config = get_pos_config();
    let executed_ops_config = get_executed_ops_config();
    let executed_denunciations_config = get_executed_denunciations_config();

    // let initial_rolls_path = match initial_rolls_path {
    //     Some(p) => p,
    //     None => path
    //         .parent()
    //         .unwrap()
    //         .parent()
    //         .unwrap()
    //         .parent()
    //         .unwrap()
    //         .join("base_config")
    //         .join("initial_rolls.json"),
    // };
    
    println!("initial_rolls_path: {:?}", initial_rolls_path);
    FinalStateConfig {
        ledger_config,
        async_pool_config,
        pos_config,
        executed_ops_config,
        executed_denunciations_config,
        final_history_length: 100,
        thread_count: THREAD_COUNT,
        periods_per_cycle: PERIODS_PER_CYCLE,
        initial_seed_string: INITIAL_DRAW_SEED.into(),
        initial_rolls_path,
        endorsement_count: ENDORSEMENT_COUNT,
        max_executed_denunciations_length: MAX_DENUNCIATION_CHANGES_LENGTH,
        max_denunciations_per_block_header: MAX_DENUNCIATIONS_PER_BLOCK_HEADER,
        t0: T0,
        genesis_timestamp: *GENESIS_TIMESTAMP,
    }
}

pub fn get_selector_config() -> SelectorConfig {
    SelectorConfig {
        max_draw_cache: SELECTOR_DRAW_CACHE_SIZE,
        channel_size: CHANNEL_SIZE,
        thread_count: THREAD_COUNT,
        endorsement_count: ENDORSEMENT_COUNT,
        periods_per_cycle: PERIODS_PER_CYCLE,
        genesis_address: Address::from_public_key(&GENESIS_KEY.get_public_key()),
    }
}

pub fn get_execution_config(last_start_period: u64, gas_costs_folder: &PathBuf) -> ExecutionConfig {

    // Storage costs constants
    let storage_costs_constants = StorageCostsConstants {
        ledger_cost_per_byte: LEDGER_COST_PER_BYTE,
        ledger_entry_base_cost: LEDGER_ENTRY_BASE_COST,
        ledger_entry_datastore_base_cost: LEDGER_COST_PER_BYTE
            .checked_mul_u64(LEDGER_ENTRY_DATASTORE_BASE_SIZE as u64)
            .expect("Overflow when creating constant ledger_entry_datastore_base_size"),
    };

    // gas costs
    let gas_costs = GasCosts::new(
        gas_costs_folder.join("abi_gas_costs.json"), // SETTINGS.execution.abi_gas_costs_file.clone(),
        gas_costs_folder.join("wasm_gas_costs.json") // SETTINGS.execution.wasm_gas_costs_file.clone(),
    ).expect("Failed to load gas costs");

    ExecutionConfig {
        max_final_events: 10000, // SETTINGS.execution.max_final_events,
        readonly_queue_length: 10, // SETTINGS.execution.readonly_queue_length,
        cursor_delay: MassaTime::from_millis(2000), // SETTINGS.execution.cursor_delay,
        max_async_gas: MAX_ASYNC_GAS,
        async_msg_cst_gas_cost: ASYNC_MSG_CST_GAS_COST,
        max_gas_per_block: MAX_GAS_PER_BLOCK,
        roll_price: ROLL_PRICE,
        thread_count: THREAD_COUNT,
        t0: T0,
        genesis_timestamp: *GENESIS_TIMESTAMP,
        block_reward: BLOCK_REWARD,
        endorsement_count: ENDORSEMENT_COUNT as u64,
        operation_validity_period: OPERATION_VALIDITY_PERIODS,
        periods_per_cycle: PERIODS_PER_CYCLE,
        stats_time_window_duration: MassaTime::from_millis(60000), // SETTINGS.execution.stats_time_window_duration,
        max_miss_ratio: *POS_MISS_RATE_DEACTIVATION_THRESHOLD,
        max_datastore_key_length: MAX_DATASTORE_KEY_LENGTH,
        max_bytecode_size: MAX_BYTECODE_LENGTH,
        max_datastore_value_size: MAX_DATASTORE_VALUE_LENGTH,
        storage_costs_constants,
        max_read_only_gas: 4_294_967_295, // SETTINGS.execution.max_read_only_gas,
        gas_costs: gas_costs.clone(),
        base_operation_gas_cost: BASE_OPERATION_GAS_COST,
        last_start_period, // final_state.read().get_last_start_period(),
        hd_cache_path: PathBuf::from("storage/cache/rocks_db"), // SETTINGS.execution.hd_cache_path.clone(),
        lru_cache_size: 200,// SETTINGS.execution.lru_cache_size,
        hd_cache_size: 2000,// SETTINGS.execution.hd_cache_size,
        snip_amount: 10,// SETTINGS.execution.snip_amount,
        roll_count_to_slash_on_denunciation: ROLL_COUNT_TO_SLASH_ON_DENUNCIATION,
        denunciation_expire_periods: DENUNCIATION_EXPIRE_PERIODS,
        broadcast_enabled: false, // SETTINGS.api.enable_broadcast,
        broadcast_slot_execution_output_channel_capacity: 5000, // SETTINGS
            // .execution
            // .broadcast_slot_execution_output_channel_capacity,
        max_event_size: MAX_EVENT_DATA_SIZE,
        max_function_length: MAX_FUNCTION_NAME_LENGTH,
        max_parameter_length: MAX_PARAMETERS_SIZE,
        chain_id: *CHAINID,
        #[cfg(feature = "execution-trace")]
        broadcast_traces_enabled: true,
        #[cfg(not(feature = "execution-trace"))]
        broadcast_traces_enabled: false,
        broadcast_slot_execution_traces_channel_capacity: 5000, // SETTINGS
            // .execution
            // .broadcast_slot_execution_traces_channel_capacity,
        max_execution_traces_slot_limit: 320, // SETTINGS.execution.execution_traces_limit,
        block_dump_folder_path: PathBuf::from(""),
    }
}