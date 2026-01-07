//! ralf CLI: Command-line interface for multi-modal autonomous loops

use clap::{Parser, Subcommand};

/// Multi-modal autonomous loop engine with TUI
#[derive(Parser)]
#[command(name = "ralf")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Open the TUI (default when no command specified)
    Tui,

    /// Detect modals and print diagnostics
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Initialize .ralf/ directory and config
    Init,

    /// Probe modals with timeout to detect auth prompts/hangs
    Probe {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Probe specific modal only
        #[arg(long)]
        modal: Option<String>,
    },

    /// Run the autonomous loop
    Run {
        /// Maximum number of iterations
        #[arg(long)]
        max_iterations: Option<u32>,

        /// Maximum runtime in seconds
        #[arg(long)]
        max_seconds: Option<u32>,

        /// Run on a specific branch
        #[arg(long)]
        branch: Option<String>,

        /// Modals to use (comma-separated, e.g. claude,codex,gemini)
        #[arg(long, value_delimiter = ',')]
        modals: Option<Vec<String>>,
    },

    /// Print current state and cooldowns
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Cancel the current run
    Cancel,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Tui) => {
            // Default: open TUI
            if let Err(e) = ralf_tui::run_tui() {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(Commands::Doctor { json: _ }) => {
            println!("doctor not implemented");
        }
        Some(Commands::Init) => {
            println!("init not implemented");
        }
        Some(Commands::Probe { json: _, modal: _ }) => {
            println!("probe not implemented");
        }
        Some(Commands::Run { .. }) => {
            println!("run not implemented");
        }
        Some(Commands::Status { json: _ }) => {
            println!("status not implemented");
        }
        Some(Commands::Cancel) => {
            println!("cancel not implemented");
        }
    }
}
