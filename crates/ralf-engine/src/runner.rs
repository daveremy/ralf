//! Loop runner for ralf engine.
//!
//! This module implements the main iteration loop, model invocation,
//! and verification.

use crate::config::{Config, ModelConfig, ModelSelection, VerifierConfig};
use crate::state::{Cooldowns, RunState};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;
use uuid::Uuid;

/// Events emitted during a run for TUI observation.
#[derive(Debug, Clone)]
pub enum RunEvent {
    /// Run started.
    Started {
        run_id: String,
        max_iterations: usize,
    },
    /// Iteration started.
    IterationStarted { iteration: usize, model: String },
    /// Model invocation completed.
    ModelCompleted {
        iteration: usize,
        model: String,
        duration_ms: u64,
        has_promise: bool,
        rate_limited: bool,
        output_preview: String,
    },
    /// Verifier completed.
    VerifierCompleted {
        iteration: usize,
        name: String,
        passed: bool,
        duration_ms: u64,
    },
    /// Model entered cooldown.
    CooldownStarted { model: String, duration_secs: u64 },
    /// Iteration completed.
    IterationCompleted {
        iteration: usize,
        all_verifiers_passed: bool,
    },
    /// Run completed successfully.
    Completed { iteration: usize, reason: String },
    /// Run failed.
    Failed { iteration: usize, error: String },
    /// Run was cancelled.
    Cancelled { iteration: usize },
    /// Status update (for progress display).
    Status { message: String },
}

/// Configuration for a run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Maximum iterations (0 = unlimited).
    pub max_iterations: usize,
    /// Maximum runtime in seconds (0 = unlimited).
    pub max_runtime_secs: u64,
    /// Path to the prompt file.
    pub prompt_path: PathBuf,
    /// Repository path.
    pub repo_path: PathBuf,
}

/// Handle for controlling a running loop.
#[derive(Debug)]
pub struct RunHandle {
    /// Channel to send cancel signal.
    cancel_tx: mpsc::Sender<()>,
}

impl RunHandle {
    /// Cancel the running loop (async version).
    pub async fn cancel(&self) {
        let _ = self.cancel_tx.send(()).await;
    }

    /// Try to cancel the running loop (non-blocking version).
    /// Returns true if the cancel signal was sent successfully.
    pub fn try_cancel(&self) -> bool {
        self.cancel_tx.try_send(()).is_ok()
    }
}

/// Run the main loop with event emission.
///
/// Returns a handle for cancellation and spawns the loop as a background task.
pub fn start_run(
    config: Config,
    run_config: RunConfig,
    event_tx: mpsc::UnboundedSender<RunEvent>,
) -> RunHandle {
    let (cancel_tx, cancel_rx) = mpsc::channel(1);

    tokio::spawn(async move {
        run_loop(config, run_config, event_tx, cancel_rx).await;
    });

    RunHandle { cancel_tx }
}

/// The main run loop.
///
/// # Event Channel
/// All event sends use `let _ = event_tx.send(...)` to silently ignore
/// failures. This is intentional: if the receiver is dropped (e.g., TUI
/// closed), the run should continue but stop sending events.
#[allow(clippy::too_many_lines)]
async fn run_loop(
    config: Config,
    run_config: RunConfig,
    event_tx: mpsc::UnboundedSender<RunEvent>,
    mut cancel_rx: mpsc::Receiver<()>,
) {
    let run_id = Uuid::new_v4().to_string()[..8].to_string();
    let start_time = Instant::now();

    // Load or create state (using spawn_blocking for serde operations)
    let ralf_dir = run_config.repo_path.join(".ralf");
    let state_path = ralf_dir.join("state.json");
    let cooldowns_path = ralf_dir.join("cooldowns.json");

    let state_path_clone = state_path.clone();
    let mut state = tokio::task::spawn_blocking(move || {
        RunState::load(&state_path_clone).unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let cooldowns_path_clone = cooldowns_path.clone();
    let mut cooldowns = tokio::task::spawn_blocking(move || {
        Cooldowns::load(&cooldowns_path_clone).unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    // Create run directory (async)
    let run_dir = ralf_dir.join("runs").join(&run_id);
    if let Err(e) = tokio::fs::create_dir_all(&run_dir).await {
        let _ = event_tx.send(RunEvent::Failed {
            iteration: 0,
            error: format!("Failed to create run directory: {e}"),
        });
        return;
    }

    // Load prompt (async)
    let prompt = match tokio::fs::read_to_string(&run_config.prompt_path).await {
        Ok(p) => p,
        Err(e) => {
            let _ = event_tx.send(RunEvent::Failed {
                iteration: 0,
                error: format!("Failed to read prompt: {e}"),
            });
            return;
        }
    };

    let _ = event_tx.send(RunEvent::Started {
        run_id: run_id.clone(),
        max_iterations: run_config.max_iterations,
    });

    let mut iteration = 0;

    loop {
        iteration += 1;

        // Check cancellation
        if cancel_rx.try_recv().is_ok() {
            let _ = event_tx.send(RunEvent::Cancelled { iteration });
            break;
        }

        // Check max iterations
        if run_config.max_iterations > 0 && iteration > run_config.max_iterations {
            let _ = event_tx.send(RunEvent::Completed {
                iteration: iteration - 1,
                reason: "Max iterations reached".into(),
            });
            break;
        }

        // Check max runtime
        if run_config.max_runtime_secs > 0
            && start_time.elapsed().as_secs() > run_config.max_runtime_secs
        {
            let _ = event_tx.send(RunEvent::Completed {
                iteration: iteration - 1,
                reason: "Max runtime reached".into(),
            });
            break;
        }

        // Clear expired cooldowns
        cooldowns.clear_expired();

        // Select model
        let model = match select_model(&config, &cooldowns, &mut state) {
            Some(m) => m.clone(),
            None => {
                // Use actual remaining cooldown time instead of fixed 5 seconds
                let wait_secs = cooldowns.earliest_expiry().map_or(5, |exp| {
                    let now = crate::state::current_timestamp();
                    exp.saturating_sub(now).max(1) // At least 1 second
                });

                let _ = event_tx.send(RunEvent::Status {
                    message: format!("All models in cooldown, waiting {wait_secs}s..."),
                });
                // Wait for cooldown with cancel check
                tokio::select! {
                    _ = cancel_rx.recv() => {
                        let _ = event_tx.send(RunEvent::Cancelled { iteration });
                        return;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(wait_secs)) => {}
                }
                continue;
            }
        };

        let _ = event_tx.send(RunEvent::IterationStarted {
            iteration,
            model: model.name.clone(),
        });

        // Invoke model with cancel check
        let invoke_result = tokio::select! {
            _ = cancel_rx.recv() => {
                let _ = event_tx.send(RunEvent::Cancelled { iteration });
                return;
            }
            result = invoke_model(&model, &prompt, &run_dir) => result
        };

        let result = match invoke_result {
            Ok(mut r) => {
                r.has_promise = check_promise(&r.stdout, &config.completion_promise);
                r
            }
            Err(e) => {
                let _ = event_tx.send(RunEvent::Failed {
                    iteration,
                    error: format!("Model invocation failed: {e}"),
                });

                // Apply cooldown on error
                cooldowns.set_cooldown(
                    &model.name,
                    model.default_cooldown_seconds,
                    "invocation error",
                );
                // Save cooldowns asynchronously
                let cooldowns_clone = cooldowns.clone();
                let path = cooldowns_path.clone();
                let _ = tokio::task::spawn_blocking(move || cooldowns_clone.save(&path)).await;

                let _ = event_tx.send(RunEvent::CooldownStarted {
                    model: model.name.clone(),
                    duration_secs: model.default_cooldown_seconds,
                });

                continue;
            }
        };

        // Send full output to TUI (no truncation - TUI handles display)
        let output_preview = result.stdout.clone();

        let _ = event_tx.send(RunEvent::ModelCompleted {
            iteration,
            model: model.name.clone(),
            duration_ms: result.duration_ms,
            has_promise: result.has_promise,
            rate_limited: result.rate_limited,
            output_preview,
        });

        // Handle rate limiting
        if result.rate_limited {
            cooldowns.set_cooldown(&model.name, model.default_cooldown_seconds, "rate limited");
            // Save cooldowns asynchronously
            let cooldowns_clone = cooldowns.clone();
            let path = cooldowns_path.clone();
            let _ = tokio::task::spawn_blocking(move || cooldowns_clone.save(&path)).await;

            let _ = event_tx.send(RunEvent::CooldownStarted {
                model: model.name.clone(),
                duration_secs: model.default_cooldown_seconds,
            });

            continue;
        }

        // Complete if promise found
        // Note: AI-powered criteria verification will be added in a future milestone
        if result.has_promise {
            let _ = event_tx.send(RunEvent::IterationCompleted {
                iteration,
                all_verifiers_passed: true, // Assuming success for now
            });

            let _ = event_tx.send(RunEvent::Completed {
                iteration,
                reason: "Promise fulfilled".into(),
            });
            break;
        } else {
            let _ = event_tx.send(RunEvent::IterationCompleted {
                iteration,
                all_verifiers_passed: false,
            });
        }

        // Save state (iteration is u64 now, safe conversion)
        state.iteration = iteration as u64;
        let state_clone = state.clone();
        let path = state_path.clone();
        let _ = tokio::task::spawn_blocking(move || state_clone.save(&path)).await;
    }

    // Final state save (awaited to ensure completion before function returns)
    let state_clone = state.clone();
    let path = state_path.clone();
    let _ = tokio::task::spawn_blocking(move || state_clone.save(&path)).await;

    let cooldowns_clone = cooldowns.clone();
    let path = cooldowns_path.clone();
    let _ = tokio::task::spawn_blocking(move || cooldowns_clone.save(&path)).await;
}

/// Result of a model invocation.
#[derive(Debug, Clone)]
pub struct InvocationResult {
    /// Model name.
    pub model: String,

    /// Exit code.
    pub exit_code: Option<i32>,

    /// Stdout output.
    pub stdout: String,

    /// Stderr output.
    pub stderr: String,

    /// Whether rate limit was detected.
    pub rate_limited: bool,

    /// Duration in milliseconds.
    pub duration_ms: u64,

    /// Whether the output contains the completion promise.
    pub has_promise: bool,
}

/// Result of running a verifier.
#[derive(Debug, Clone)]
pub struct VerifierResult {
    /// Verifier name.
    pub name: String,

    /// Whether the verifier passed.
    pub passed: bool,

    /// Exit code.
    pub exit_code: Option<i32>,

    /// Combined output.
    pub output: String,

    /// Duration in milliseconds.
    pub duration_ms: u64,
}

/// Invoke a model with the given prompt.
pub async fn invoke_model(
    model: &ModelConfig,
    prompt: &str,
    run_dir: &Path,
) -> Result<InvocationResult, RunnerError> {
    let start = std::time::Instant::now();

    // Build command
    let mut cmd = Command::new(&model.command_argv[0]);
    for arg in &model.command_argv[1..] {
        cmd.arg(arg);
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(RunnerError::Spawn)?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(RunnerError::Io)?;
        // Drop stdin to close it and signal EOF
        drop(stdin);
    }

    // Wait with timeout
    let timeout_duration = Duration::from_secs(model.timeout_seconds);
    let result = timeout(timeout_duration, child.wait_with_output()).await;

    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Check for rate limiting
            let combined = format!("{stdout}\n{stderr}");
            let rate_limited = check_rate_limit(&combined, &model.rate_limit_patterns);

            // Write log file (async)
            let log_path = run_dir.join(format!("{}.log", model.name));
            write_log(&log_path, &stdout, &stderr).await?;

            Ok(InvocationResult {
                model: model.name.clone(),
                exit_code: output.status.code(),
                stdout,
                stderr,
                rate_limited,
                duration_ms,
                has_promise: false, // Set by caller after checking
            })
        }
        Ok(Err(e)) => Err(RunnerError::Io(e)),
        Err(_) => {
            // Timeout - process was killed by kill_on_drop
            Err(RunnerError::Timeout(model.name.clone()))
        }
    }
}

/// Check if output contains rate limit patterns.
fn check_rate_limit(output: &str, patterns: &[String]) -> bool {
    let lower = output.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
}

/// Write log file with stdout and stderr.
async fn write_log(path: &Path, stdout: &str, stderr: &str) -> Result<(), RunnerError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(RunnerError::Io)?;
    }

    let file = tokio::fs::File::create(path).await.map_err(RunnerError::Io)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b"=== STDOUT ===\n").await.map_err(RunnerError::Io)?;
    writer.write_all(stdout.as_bytes()).await.map_err(RunnerError::Io)?;
    writer.write_all(b"\n\n=== STDERR ===\n").await.map_err(RunnerError::Io)?;
    writer.write_all(stderr.as_bytes()).await.map_err(RunnerError::Io)?;
    writer.write_all(b"\n").await.map_err(RunnerError::Io)?;
    writer.flush().await.map_err(RunnerError::Io)?;
    Ok(())
}

/// Run a verifier.
pub async fn run_verifier(
    verifier: &VerifierConfig,
    run_dir: &Path,
) -> Result<VerifierResult, RunnerError> {
    let start = std::time::Instant::now();

    let mut cmd = Command::new(&verifier.command_argv[0]);
    for arg in &verifier.command_argv[1..] {
        cmd.arg(arg);
    }

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let timeout_duration = Duration::from_secs(verifier.timeout_seconds);
    let result = timeout(timeout_duration, cmd.output()).await;

    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}\n{stderr}");

            // Write verifier log (async)
            let log_path = run_dir.join(format!("{}.log", verifier.name));
            write_log(&log_path, &stdout, &stderr).await?;

            Ok(VerifierResult {
                name: verifier.name.clone(),
                passed: output.status.success(),
                exit_code: output.status.code(),
                output: combined,
                duration_ms,
            })
        }
        Ok(Err(e)) => Err(RunnerError::Io(e)),
        Err(_) => Err(RunnerError::Timeout(verifier.name.clone())),
    }
}

/// Select the next model to use based on the selection strategy.
///
/// For round-robin selection, this advances the index for the next call.
pub fn select_model<'a>(
    config: &'a Config,
    cooldowns: &Cooldowns,
    state: &mut RunState,
) -> Option<&'a ModelConfig> {
    let available: Vec<&ModelConfig> = config
        .models
        .iter()
        .filter(|m| !cooldowns.is_cooling(&m.name))
        .collect();

    if available.is_empty() {
        return None;
    }

    match config.model_selection {
        ModelSelection::RoundRobin => {
            // Get next model in rotation
            let index = state.last_model_index % available.len();
            // Advance index for next selection
            state.last_model_index = state.last_model_index.wrapping_add(1);
            Some(available[index])
        }
        ModelSelection::Priority => {
            // Find first available model in priority order
            for name in &config.model_priority {
                if let Some(model) = available.iter().find(|m| &m.name == name) {
                    return Some(model);
                }
            }
            // Fall back to first available
            available.first().copied()
        }
    }
}

/// Check if output contains the completion promise.
pub fn check_promise(output: &str, promise: &str) -> bool {
    let pattern = format!("<promise>{promise}</promise>");
    output.contains(&pattern)
}

/// Extract promise from output if present.
pub fn extract_promise(output: &str) -> Option<String> {
    let re = Regex::new(r"<promise>([^<]+)</promise>").ok()?;
    re.captures(output)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Compute SHA256 hash of prompt.
pub fn hash_prompt(prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    let result = hasher.finalize();
    format!("{result:x}")
}

/// Get git information for changelog.
pub fn get_git_info() -> GitInfo {
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".into());

    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .is_some_and(|o| !o.stdout.is_empty());

    let changed_files = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
                    parts.get(1).map(|s| s.trim().to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    GitInfo {
        branch,
        dirty,
        changed_files,
    }
}

/// Git information.
#[derive(Debug, Clone)]
pub struct GitInfo {
    /// Current branch.
    pub branch: String,
    /// Whether the working tree is dirty.
    pub dirty: bool,
    /// List of changed files.
    pub changed_files: Vec<String>,
}

/// Errors that can occur during running.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to spawn process.
    #[error("Failed to spawn process: {0}")]
    Spawn(#[source] std::io::Error),

    /// Process timed out.
    #[error("Process timed out: {0}")]
    Timeout(String),

    /// No models available.
    #[error("No models available (all in cooldown)")]
    NoModelsAvailable,

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Prompt file not found.
    #[error("Prompt file not found: {0}")]
    PromptNotFound(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_rate_limit() {
        let patterns = vec!["429".into(), "rate limit".into()];

        assert!(check_rate_limit("Error: 429 Too Many Requests", &patterns));
        assert!(check_rate_limit("Rate limit exceeded", &patterns));
        assert!(!check_rate_limit("Success", &patterns));
    }

    #[test]
    fn test_check_promise() {
        assert!(check_promise(
            "Output with <promise>COMPLETE</promise> tag",
            "COMPLETE"
        ));
        assert!(!check_promise("Output without tag", "COMPLETE"));
        assert!(!check_promise(
            "Output with <promise>WRONG</promise> tag",
            "COMPLETE"
        ));
    }

    #[test]
    fn test_extract_promise() {
        assert_eq!(
            extract_promise("Output with <promise>COMPLETE</promise> tag"),
            Some("COMPLETE".into())
        );
        assert_eq!(extract_promise("No tag here"), None);
    }

    #[test]
    fn test_hash_prompt() {
        let hash1 = hash_prompt("Hello, world!");
        let hash2 = hash_prompt("Hello, world!");
        let hash3 = hash_prompt("Different prompt");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 hex is 64 chars
    }

    #[test]
    fn test_select_model_round_robin() {
        let config = Config::with_detected_models(&["claude".into(), "codex".into()]);
        let cooldowns = Cooldowns::default();

        let mut state = RunState::default();
        state.last_model_index = 0;

        // First selection should get first model and advance index
        let model1 = select_model(&config, &cooldowns, &mut state);
        assert!(model1.is_some());
        assert_eq!(state.last_model_index, 1);

        // Second selection should get second model and advance index
        let model2 = select_model(&config, &cooldowns, &mut state);
        assert!(model2.is_some());
        assert_eq!(state.last_model_index, 2);

        // Models should be different (round-robin working)
        assert_ne!(model1.unwrap().name, model2.unwrap().name);
    }
}
