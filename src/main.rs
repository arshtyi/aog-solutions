use std::path::PathBuf;

use anyhow::Result;
use aog_solutions::{DEFAULT_DATA_FILE, DEFAULT_GAME_DIR, ExtractOptions, extract};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Extract every discoverable puzzle into normalized JSON.
    Extract {
        /// The root directory of the installed game.
        #[arg(long, default_value = DEFAULT_GAME_DIR)]
        game_dir: PathBuf,

        /// Destination for generated puzzle data.
        #[arg(short, long, default_value = DEFAULT_DATA_FILE)]
        output: PathBuf,

        /// Suppress extraction progress messages.
        #[arg(long)]
        quiet: bool,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Extract {
            game_dir,
            output,
            quiet,
        } => {
            let options = ExtractOptions::new(game_dir, &output).with_progress(!quiet);
            let report = extract::run(&options)?;
            println!(
                "Extracted {} puzzles from {} candidate sources in {} PAK archives to {}",
                report.puzzles,
                report.scanned_sources,
                report.archives,
                output.display()
            );
            Ok(())
        }
    }
}
