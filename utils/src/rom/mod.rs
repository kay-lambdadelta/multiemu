use std::{fmt::Display, path::PathBuf};

use clap::{Subcommand, ValueEnum};

pub mod export;
pub mod import;
pub mod verify;

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum ExportStyle {
    #[default]
    NoIntro,
    Native,
    EmulationStation,
}

impl Display for ExportStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ExportStyle::NoIntro => "no-intro",
                ExportStyle::Native => "native",
                ExportStyle::EmulationStation => "emulationstation",
            }
        )
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum RomAction {
    /// Recursively searches through select paths to find ROMs that match database entries
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
        #[clap(short = 'l', long)]
        symlink: bool,
    },
    /// Exports ROMs from the local store to a directory
    Export {
        path: PathBuf,
        #[clap(short = 'l', long)]
        symlink: bool,
        #[clap(short, long, default_value_t=ExportStyle::default())]
        style: ExportStyle,
    },
    Verify,
}
