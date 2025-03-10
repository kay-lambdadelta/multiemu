use clap::Subcommand;
use logiqx::LogiqxAction;
use native::NativeAction;

pub mod logiqx;
pub mod native;
pub mod redump;
pub mod screenscraper;

#[derive(Clone, Debug, Subcommand)]
pub enum DatabaseAction {
    /// Extracts metadata from Logiqx style dat files
    Logiqx {
        #[clap(subcommand)]
        action: LogiqxAction,
    },
    /// Imports the contents of a native database
    Native {
        #[clap(subcommand)]
        action: NativeAction,
    },
    Redump {
        #[clap(subcommand)]
        action: redump::RedumpAction,
    },
    ScreenScraper {},
}
