//! Loop runner for ralf engine.
//!
//! This module implements the main iteration loop, model invocation,
//! and verification.

use crate::config::{Config, ModelConfig, ModelSelection, VerifierConfig};
use crate::state::{Cooldowns, RunState};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

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

            // Write log file
            let log_path = run_dir.join(format!("{}.log", model.name));
            write_log(&log_path, &stdout, &stderr)?;

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
    patterns
        .iter()
        .any(|p| lower.contains(&p.to_lowercase()))
}

/// Write log file with stdout and stderr.
fn write_log(path: &Path, stdout: &str, stderr: &str) -> Result<(), RunnerError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(RunnerError::Io)?;
    }

    let mut file = std::fs::File::create(path).map_err(RunnerError::Io)?;
    writeln!(file, "=== STDOUT ===").map_err(RunnerError::Io)?;
    writeln!(file, "{stdout}").map_err(RunnerError::Io)?;
    writeln!(file, "\n=== STDERR ===").map_err(RunnerError::Io)?;
    writeln!(file, "{stderr}").map_err(RunnerError::Io)?;
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

            // Write verifier log
            let log_path = run_dir.join(format!("{}.log", verifier.name));
            write_log(&log_path, &stdout, &stderr)?;

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
pub fn select_model<'a>(
    config: &'a Config,
    cooldowns: &Cooldowns,
    state: &RunState,
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

        let model = select_model(&config, &cooldowns, &state);
        assert!(model.is_some());
    }
}
