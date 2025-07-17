use clap::{Parser, Subcommand};
use multiemu_rom::{RomId, System};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub action: Option<CliAction>,
}

#[derive(Clone, Subcommand)]
pub enum CliAction {
    /// Run a ROM(s) according to their ID (sha1 hash)
    Run {
        #[clap(required=true, num_args=1..)]
        roms: Vec<RomId>,
        #[clap(short, long)]
        forced_system: Option<System>,
    },
    /// Run a ROM(s) according to their path on your filesystem
    RunExternal {
        #[clap(required=true, num_args=1..)]
        roms: Vec<PathBuf>,
        #[clap(short, long)]
        forced_system: Option<System>,
    },
}
