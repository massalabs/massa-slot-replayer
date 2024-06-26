use std::error::Error;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(name = "slot_replayer_try_1")]
#[command(about = "Slot replay", long_about = None)]
pub struct Cli {
    #[arg(
        short = 'p',
        long = "path",
        help = "Path of an existing db (e.g: /tmp/massa_8_g4j_3n/massa-node/storage/ledger/rocks_db"
    )]
    pub(crate) db_path: PathBuf,
    #[arg(
        short = 'r',
        long = "initial_roll_path",
        help = "Filepath to initial_rolls.json"
    )]
    pub(crate) initial_rolls_path: PathBuf,
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Clone, PartialEq, Subcommand)]
pub(crate) enum Commands {
    #[command(about = "List snapshot (& display info)")]
    ListSnapshot,
    #[command(about = "Replay blocks (from a db backup and dumped blocks)")]
    Replay(ReplayArgs),
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct ReplayArgs {
    #[arg(
        short = 'b',
        long = "blocks",
        help = "Folder where to find block dumped as .bin file"
    )]
    pub(crate) dump_block_path: PathBuf,
    #[arg(long = "backup", help = "Folder where to find db backup")]
    pub(crate) db_backup_path: PathBuf,
    #[arg(
        long = "until_slot",
        help = "Replay from last slot defined into backup to given slot period, if not specified will replay until blocks are available. ex: `--slot 40,2`",
        value_parser = parse_slot,
    )]
    pub(crate) until_slot: Option<(u64, u8)>,
}

fn parse_slot(s: &str) -> Result<(u64, u8), Box<dyn Error + Send + Sync + 'static>> {
    
    let (period_, thread_) = s.split_once(',')
        .ok_or("Slot must be specified as PERIOD,THREAD, ex: `--slt 40,2`")?;
    let period = period_.parse::<u64>()?;
    let thread = thread_.parse::<u8>()?;
    Ok((period, thread))
}
