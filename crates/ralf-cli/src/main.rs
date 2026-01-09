//! ralf CLI: Command-line interface for multi-model autonomous loops

use clap::{Parser, Subcommand};
use ralf_engine::{
    check_promise, discover_models, get_git_info, hash_prompt, invoke_model, probe_model,
    run_verifier, select_model, write_changelog_entry, ChangelogEntry, Config, Cooldowns,
    IterationStatus, RunState, RunStatus,
};
use std::path::Path;
use std::time::{Duration, Instant};

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

    /// Open the M5-A shell (new TUI architecture preview)
    Shell,

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
        max_iterations: Option<u64>,

        /// Maximum runtime in seconds
        #[arg(long)]
        max_seconds: Option<u64>,

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
            let repo_path = std::env::current_dir().expect("Failed to get current directory");
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            if let Err(e) = rt.block_on(ralf_tui::run_tui(&repo_path)) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(Commands::Shell) => {
            // M5-A shell preview
            if let Err(e) = ralf_tui::run_shell_tui() {
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

    // Check for prompt file (at repo root, not in .ralf/)
    let prompt_path = Path::new("PROMPT.md");
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
        if let Err(e) = std::fs::write(prompt_path, default_prompt) {
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
    max_iterations: Option<u64>,
    max_seconds: Option<u64>,
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

    // Prompt is at repo root
    let prompt_path = Path::new("PROMPT.md");
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

    // Run the loop
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(run_loop(
        config,
        ralf_dir,
        prompt_path,
        max_iterations,
        max_seconds,
    ));
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

/// Run the main autonomous loop.
#[allow(clippy::too_many_lines, clippy::similar_names)]
async fn run_loop(
    config: Config,
    ralf_dir: &Path,
    prompt_path: &Path,
    max_iterations: Option<u64>,
    max_seconds: Option<u64>,
) {
    let state_path = ralf_dir.join("state.json");
    let cooldowns_path = ralf_dir.join("cooldowns.json");
    let runs_dir = ralf_dir.join("runs");
    let changelog_dir = ralf_dir.join("changelog");

    // Load or create state
    let mut state = RunState::load(&state_path).unwrap_or_default();
    let mut cooldowns = Cooldowns::load(&cooldowns_path).unwrap_or_default();

    // Start a new run
    let run_id = state.start_run();
    println!("Starting run {run_id}");

    // Create run directory
    let run_dir = runs_dir.join(&run_id);
    if let Err(e) = std::fs::create_dir_all(&run_dir) {
        eprintln!("Failed to create run directory: {e}");
        state.fail();
        let _ = state.save(&state_path);
        std::process::exit(1);
    }

    // Read the prompt
    let prompt = match std::fs::read_to_string(prompt_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to read prompt: {e}");
            state.fail();
            let _ = state.save(&state_path);
            std::process::exit(1);
        }
    };
    let prompt_hash = hash_prompt(&prompt);

    // Save initial state
    let _ = state.save(&state_path);

    let start_time = Instant::now();
    let max_iterations = max_iterations.unwrap_or(100);
    let max_duration = max_seconds.map(Duration::from_secs);

    println!("Prompt hash: {}", &prompt_hash[..8]);
    println!("Max iterations: {max_iterations}");
    if let Some(d) = max_duration {
        println!("Max duration: {}s", d.as_secs());
    }
    println!();

    // Main loop
    loop {
        // Check iteration limit
        if state.iteration >= max_iterations {
            println!("\nMax iterations ({max_iterations}) reached");
            state.fail();
            break;
        }

        // Check time limit
        if let Some(max_dur) = max_duration {
            if start_time.elapsed() > max_dur {
                println!("\nMax duration reached");
                state.fail();
                break;
            }
        }

        // Clear expired cooldowns
        cooldowns.clear_expired();

        // Select a model
        let Some(model) = select_model(&config, &cooldowns, &mut state) else {
            // All models in cooldown - wait for earliest expiry
            if let Some(expiry) = cooldowns.earliest_expiry() {
                let now = ralf_engine::state::current_timestamp();
                let wait_secs = expiry.saturating_sub(now);
                println!("All models in cooldown, waiting {wait_secs}s...");
                tokio::time::sleep(Duration::from_secs(wait_secs + 1)).await;
                continue;
            }
            eprintln!("No models available");
            state.fail();
            break;
        };

        state.next_iteration();
        println!(
            "=== Iteration {} - Model: {} ===",
            state.iteration, model.name
        );

        // Save state
        let _ = state.save(&state_path);

        // Invoke the model
        let invocation = match invoke_model(model, &prompt, &run_dir).await {
            Ok(mut inv) => {
                inv.has_promise = check_promise(&inv.stdout, &config.completion_promise);
                inv
            }
            Err(ralf_engine::RunnerError::Timeout(name)) => {
                println!("  Model {name} timed out");
                let entry = ChangelogEntry {
                    changelog_dir: &changelog_dir,
                    run_id: &run_id,
                    iteration: state.iteration,
                    invocation: &ralf_engine::InvocationResult {
                        model: model.name.clone(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        rate_limited: false,
                        duration_ms: model.timeout_seconds * 1000,
                        has_promise: false,
                    },
                    verifier_results: &[],
                    prompt_hash: &prompt_hash,
                    git_info: &get_git_info(),
                    status: IterationStatus::Timeout,
                    reason: "Model timed out",
                    log_path: run_dir.join(format!("{}.log", model.name)),
                };
                let _ = write_changelog_entry(&entry);
                cooldowns.set_cooldown(&model.name, model.default_cooldown_seconds, "timeout");
                let _ = cooldowns.save(&cooldowns_path);
                continue;
            }
            Err(e) => {
                eprintln!("  Model error: {e}");
                let entry = ChangelogEntry {
                    changelog_dir: &changelog_dir,
                    run_id: &run_id,
                    iteration: state.iteration,
                    invocation: &ralf_engine::InvocationResult {
                        model: model.name.clone(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: e.to_string(),
                        rate_limited: false,
                        duration_ms: 0,
                        has_promise: false,
                    },
                    verifier_results: &[],
                    prompt_hash: &prompt_hash,
                    git_info: &get_git_info(),
                    status: IterationStatus::Error,
                    reason: "Model invocation failed",
                    log_path: run_dir.join(format!("{}.log", model.name)),
                };
                let _ = write_changelog_entry(&entry);
                continue;
            }
        };

        // Check for rate limiting
        if invocation.rate_limited {
            println!(
                "  Rate limited ({}ms), cooling down for {}s",
                invocation.duration_ms, model.default_cooldown_seconds
            );
            let entry = ChangelogEntry {
                changelog_dir: &changelog_dir,
                run_id: &run_id,
                iteration: state.iteration,
                invocation: &invocation,
                verifier_results: &[],
                prompt_hash: &prompt_hash,
                git_info: &get_git_info(),
                status: IterationStatus::RateLimited,
                reason: "Rate limited",
                log_path: run_dir.join(format!("{}.log", model.name)),
            };
            let _ = write_changelog_entry(&entry);
            cooldowns.set_cooldown(&model.name, model.default_cooldown_seconds, "rate_limit");
            let _ = cooldowns.save(&cooldowns_path);
            continue;
        }

        println!("  Model completed in {}ms", invocation.duration_ms);
        println!("  Has promise: {}", invocation.has_promise);

        // Run verifiers
        let mut verifier_results = Vec::new();
        let mut all_passed = true;

        for verifier in &config.verifiers {
            print!("  Running verifier '{}'... ", verifier.name);
            match run_verifier(verifier, &run_dir).await {
                Ok(result) => {
                    if result.passed {
                        println!("PASS ({}ms)", result.duration_ms);
                    } else {
                        println!("FAIL ({}ms)", result.duration_ms);
                        all_passed = false;
                    }
                    verifier_results.push(result);
                }
                Err(e) => {
                    println!("ERROR: {e}");
                    all_passed = false;
                    verifier_results.push(ralf_engine::VerifierResult {
                        name: verifier.name.clone(),
                        passed: false,
                        exit_code: None,
                        output: e.to_string(),
                        duration_ms: 0,
                    });
                }
            }
        }

        // Determine status and reason
        let (status, reason) = if invocation.has_promise && all_passed {
            (
                IterationStatus::Success,
                "All verifiers passed with promise",
            )
        } else if invocation.has_promise && !all_passed {
            (
                IterationStatus::VerifierFailed,
                "Promise found but verifiers failed",
            )
        } else if !invocation.has_promise && all_passed {
            (
                IterationStatus::VerifierFailed,
                "Verifiers passed but no promise",
            )
        } else {
            (
                IterationStatus::VerifierFailed,
                "Verifiers failed, no promise",
            )
        };

        // Write changelog entry
        let entry = ChangelogEntry {
            changelog_dir: &changelog_dir,
            run_id: &run_id,
            iteration: state.iteration,
            invocation: &invocation,
            verifier_results: &verifier_results,
            prompt_hash: &prompt_hash,
            git_info: &get_git_info(),
            status,
            reason,
            log_path: run_dir.join(format!("{}.log", model.name)),
        };
        let _ = write_changelog_entry(&entry);

        // Check for completion
        if invocation.has_promise && all_passed {
            println!("\n=== RUN COMPLETE ===");
            println!("Promise found and all verifiers passed!");
            state.complete();
            break;
        }

        println!("  Status: {status} - {reason}");
    }

    // Save final state
    let _ = state.save(&state_path);
    let _ = cooldowns.save(&cooldowns_path);

    println!("\nRun {} finished with status: {}", run_id, state.status);
}
