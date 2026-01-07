//! ralf CLI: Command-line interface for multi-model autonomous loops

use clap::{Parser, Subcommand};
use ralf_engine::{
    discover_models, probe_model, Config, Cooldowns, RunState, RunStatus,
};
use std::path::Path;
use std::time::Duration;

/// Multi-model autonomous loop engine with TUI
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

    /// Detect models and print diagnostics
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Initialize .ralf/ directory and config
    Init,

    /// Probe models with timeout to detect auth prompts/hangs
    Probe {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Probe specific model only
        #[arg(long)]
        model: Option<String>,

        /// Timeout in seconds (default: 10)
        #[arg(long, default_value = "10")]
        timeout: u64,
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

        /// Models to use (comma-separated, e.g. claude,codex,gemini)
        #[arg(long, value_delimiter = ',')]
        models: Option<Vec<String>>,
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

const RALF_DIR: &str = ".ralf";

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
        Some(Commands::Doctor { json }) => {
            cmd_doctor(json);
        }
        Some(Commands::Init) => {
            cmd_init();
        }
        Some(Commands::Probe {
            json,
            model,
            timeout,
        }) => {
            cmd_probe(json, model, timeout);
        }
        Some(Commands::Run {
            max_iterations,
            max_seconds,
            branch,
            models,
        }) => {
            cmd_run(max_iterations, max_seconds, branch, models);
        }
        Some(Commands::Status { json }) => {
            cmd_status(json);
        }
        Some(Commands::Cancel) => {
            cmd_cancel();
        }
    }
}

fn cmd_doctor(json: bool) {
    let result = discover_models();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("failed to serialize")
        );
        return;
    }

    println!("Model Discovery Results\n");

    for model in &result.models {
        let status = if model.callable {
            "ready"
        } else if model.found {
            "found but not callable"
        } else {
            "not found"
        };

        println!("  {} - {}", model.name, status);

        if let Some(path) = &model.path {
            println!("    Path: {path}");
        }
        if let Some(version) = &model.version {
            println!("    Version: {version}");
        }
        for issue in &model.issues {
            println!("    Issue: {issue}");
        }
        println!();
    }

    let ready_count = result.models.iter().filter(|m| m.callable).count();
    println!("{ready_count} model(s) ready");
}

fn cmd_init() {
    let ralf_dir = Path::new(RALF_DIR);

    // Create directory structure
    let dirs = ["runs", "changelog"];
    for dir in dirs {
        let path = ralf_dir.join(dir);
        if let Err(e) = std::fs::create_dir_all(&path) {
            eprintln!("Failed to create {}: {e}", path.display());
            std::process::exit(1);
        }
    }

    // Check for existing config
    let config_path = ralf_dir.join("config.json");
    if config_path.exists() {
        println!("Config already exists at {}", config_path.display());
    } else {
        // Discover models and create config
        let result = discover_models();
        let available: Vec<String> = result
            .models
            .iter()
            .filter(|m| m.callable)
            .map(|m| m.name.clone())
            .collect();

        let config = if available.is_empty() {
            println!("Warning: No models found on PATH");
            Config::default()
        } else {
            println!("Found models: {}", available.join(", "));
            Config::with_detected_models(&available)
        };

        match config.save(&config_path) {
            Ok(()) => println!("Created {}", config_path.display()),
            Err(e) => {
                eprintln!("Failed to write config: {e}");
                std::process::exit(1);
            }
        }
    }

    // Check for prompt file
    let prompt_path = ralf_dir.join("PROMPT.md");
    if !prompt_path.exists() {
        let default_prompt = r"# Task Description

Describe the task for the autonomous loop here.

## Completion Criteria

- [ ] Criterion 1
- [ ] Criterion 2

## Instructions

When the task is complete, output:

<promise>COMPLETE</promise>
";
        if let Err(e) = std::fs::write(&prompt_path, default_prompt) {
            eprintln!("Failed to write prompt file: {e}");
            std::process::exit(1);
        }
        println!("Created {}", prompt_path.display());
    }

    println!("\nInitialization complete!");
    println!("Edit {} to configure your task", prompt_path.display());
}

fn cmd_probe(json: bool, model_filter: Option<String>, timeout_secs: u64) {
    let timeout = Duration::from_secs(timeout_secs);

    let models_to_probe = if let Some(name) = model_filter {
        vec![name]
    } else {
        ralf_engine::discovery::KNOWN_MODELS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    };

    let results: Vec<_> = models_to_probe
        .iter()
        .map(|name| probe_model(name, timeout))
        .collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).expect("failed to serialize")
        );
        return;
    }

    println!("Model Probe Results (timeout: {timeout_secs}s)\n");

    for result in &results {
        let status = if result.success {
            "OK"
        } else if result.needs_auth {
            "needs auth"
        } else {
            "failed"
        };

        println!("  {} - {}", result.name, status);

        if let Some(ms) = result.response_time_ms {
            println!("    Response time: {ms}ms");
        }
        for issue in &result.issues {
            println!("    Issue: {issue}");
        }
        for suggestion in &result.suggestions {
            println!("    Suggestion: {suggestion}");
        }
        println!();
    }

    let ready_count = results.iter().filter(|r| r.success).count();
    println!("{ready_count} model(s) responding");
}

fn cmd_run(
    _max_iterations: Option<u32>,
    _max_seconds: Option<u32>,
    _branch: Option<String>,
    _models: Option<Vec<String>>,
) {
    let ralf_dir = Path::new(RALF_DIR);

    // Check for initialization
    if !ralf_dir.exists() {
        eprintln!("Error: .ralf directory not found. Run `ralf init` first.");
        std::process::exit(1);
    }

    let config_path = ralf_dir.join("config.json");
    if !config_path.exists() {
        eprintln!("Error: config.json not found. Run `ralf init` first.");
        std::process::exit(1);
    }

    let prompt_path = ralf_dir.join("PROMPT.md");
    if !prompt_path.exists() {
        eprintln!("Error: PROMPT.md not found. Run `ralf init` first.");
        std::process::exit(1);
    }

    // Load config
    let config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {e}");
            std::process::exit(1);
        }
    };

    println!("Loaded config with {} model(s)", config.models.len());
    println!("Promise: {}", config.completion_promise);
    println!("\nLoop runner not yet implemented - use TUI for now");
}

fn cmd_status(json: bool) {
    let ralf_dir = Path::new(RALF_DIR);
    let state_path = ralf_dir.join("state.json");
    let cooldowns_path = ralf_dir.join("cooldowns.json");

    let state = RunState::load(&state_path).ok();
    let cooldowns = Cooldowns::load(&cooldowns_path).ok();

    if json {
        let output = serde_json::json!({
            "state": state,
            "cooldowns": cooldowns,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("failed to serialize")
        );
        return;
    }

    println!("Ralf Status\n");

    match state {
        Some(s) => {
            if let Some(run_id) = &s.run_id {
                println!("Run: {run_id}");
            }
            println!("Status: {}", s.status);
            println!("Iteration: {}", s.iteration);
            if let Some(started) = s.started_at {
                println!("Started: {started} (Unix timestamp)");
            }
        }
        None => {
            println!("No active run");
        }
    }

    println!();

    match cooldowns {
        Some(c) => {
            let cooling = c.cooling_models();
            if cooling.is_empty() {
                println!("No models in cooldown");
            } else {
                println!("Models in cooldown:");
                for name in cooling {
                    println!("  - {name}");
                }
            }
        }
        None => {
            println!("No cooldown data");
        }
    }
}

fn cmd_cancel() {
    let ralf_dir = Path::new(RALF_DIR);
    let state_path = ralf_dir.join("state.json");

    let Ok(mut state) = RunState::load(&state_path) else {
        eprintln!("No active run to cancel");
        std::process::exit(1);
    };

    if state.status != RunStatus::Running {
        eprintln!("Run is not active (status: {})", state.status);
        std::process::exit(1);
    }

    state.cancel();

    if let Err(e) = state.save(&state_path) {
        eprintln!("Failed to save state: {e}");
        std::process::exit(1);
    }

    let run_id = state.run_id.as_deref().unwrap_or("unknown");
    println!("Cancelled run {run_id}");
}
