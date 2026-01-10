//! Model discovery for ralf engine.
//!
//! This module handles detecting and probing model CLIs on the system.

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

/// Known model CLI names.
pub const KNOWN_MODELS: &[&str] = &["claude", "codex", "gemini"];

/// Result of model discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// Discovered models.
    pub models: Vec<ModelInfo>,
}

/// Information about a discovered model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name.
    pub name: String,

    /// Whether the model binary was found on PATH.
    pub found: bool,

    /// Whether the model is callable (responds to --help).
    pub callable: bool,

    /// Path to the binary, if found.
    pub path: Option<String>,

    /// Version string, if available.
    pub version: Option<String>,

    /// Any issues detected.
    pub issues: Vec<String>,
}

/// Discover all known models on the system.
pub fn discover_models() -> DiscoveryResult {
    let models = KNOWN_MODELS
        .iter()
        .map(|name| discover_model(name))
        .collect();

    DiscoveryResult { models }
}

/// Discover a single model by name.
pub fn discover_model(name: &str) -> ModelInfo {
    let mut info = ModelInfo {
        name: name.to_string(),
        found: false,
        callable: false,
        path: None,
        version: None,
        issues: Vec::new(),
    };

    // Try to find the binary on PATH
    match which::which(name) {
        Ok(path) => {
            info.found = true;
            info.path = Some(path.display().to_string());

            // Try to call with --help to verify it's callable
            match Command::new(name).arg("--help").output() {
                Ok(output) => {
                    if output.status.success() {
                        info.callable = true;
                        // Try to extract version from output
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if let Some(version) = extract_version(&stdout) {
                            info.version = Some(version);
                        }
                    } else {
                        info.issues.push(format!(
                            "--help exited with code {}",
                            output.status.code().unwrap_or(-1)
                        ));
                    }
                }
                Err(e) => {
                    info.issues.push(format!("Failed to run --help: {e}"));
                }
            }
        }
        Err(_) => {
            info.issues.push(format!("{name} not found on PATH"));
        }
    }

    info
}

/// Result of probing a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Model name.
    pub name: String,

    /// Whether the probe succeeded.
    pub success: bool,

    /// Response time in milliseconds.
    pub response_time_ms: Option<u64>,

    /// Whether the model appears to require auth.
    pub needs_auth: bool,

    /// Whether the model is rate limited.
    pub rate_limited: bool,

    /// When the rate limit resets (human-readable string if available).
    pub rate_limit_reset: Option<String>,

    /// Any issues detected.
    pub issues: Vec<String>,

    /// Suggestions for fixing issues.
    pub suggestions: Vec<String>,
}

/// Probe a model with a simple test prompt.
///
/// This is a convenience function that discovers the model first.
/// If you already have a `ModelInfo`, use [`probe_model_with_info`] instead
/// to avoid redundant discovery.
pub fn probe_model(name: &str, timeout: Duration) -> ProbeResult {
    let info = discover_model(name);
    probe_model_with_info(&info, timeout)
}

/// Probe a model with a simple test prompt, using pre-discovered model info.
///
/// This avoids redundant discovery when you already have the `ModelInfo`.
pub fn probe_model_with_info(info: &ModelInfo, timeout: Duration) -> ProbeResult {
    let mut result = ProbeResult {
        name: info.name.clone(),
        success: false,
        response_time_ms: None,
        needs_auth: false,
        rate_limited: false,
        rate_limit_reset: None,
        issues: Vec::new(),
        suggestions: Vec::new(),
    };

    if !info.found {
        result
            .issues
            .push(format!("{} not found on PATH", info.name));
        result
            .suggestions
            .push(format!("Install {} CLI and add to PATH", info.name));
        return result;
    }

    if !info.callable {
        result.issues.extend(info.issues.clone());
        return result;
    }

    // Try a simple probe with timeout
    let start = std::time::Instant::now();

    // Use a simple echo-like prompt that should return quickly
    let probe_result = run_probe_command(&info.name, timeout);

    match probe_result {
        Ok(output) => {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed = start.elapsed().as_millis() as u64;
            result.response_time_ms = Some(elapsed);

            if output.success {
                result.success = true;
            } else {
                // Check for specific error conditions
                let combined = format!("{}\n{}", output.stdout, output.stderr);
                let combined_lower = combined.to_lowercase();

                // Extract actual error line for cleaner messages
                let error_line = combined
                    .lines()
                    .find(|l| l.starts_with("ERROR:") || l.starts_with("error:"))
                    .map_or("", |l| {
                        l.trim_start_matches("ERROR:")
                            .trim_start_matches("error:")
                            .trim()
                    });

                if combined_lower.contains("limit")
                    || combined_lower.contains("quota")
                    || combined.contains("429")
                {
                    // Rate limit / usage limit reached
                    result.rate_limited = true;
                    result.issues.push("Rate limited".into());

                    // Try to extract reset time from error (e.g., "try again at Jan 12th, 2026 9:08 PM")
                    if let Some(reset_time) = extract_reset_time(&combined) {
                        result.rate_limit_reset = Some(reset_time);
                    } else {
                        result
                            .suggestions
                            .push("Wait for limit to reset or upgrade plan".into());
                    }
                } else if is_auth_error(&combined_lower) {
                    // Auth required
                    result.needs_auth = true;
                    result.issues.push("Model requires authentication".into());
                    result.suggestions.push(format!(
                        "Run `{} auth login` or configure credentials",
                        info.name
                    ));
                } else {
                    // Generic failure - use error line if available
                    let message = if error_line.is_empty() {
                        // Truncate long stderr to first meaningful line
                        output
                            .stderr
                            .lines()
                            .find(|l| {
                                !l.trim().is_empty()
                                    && !l.contains("v0.")
                                    && !l.contains("preview")
                            })
                            .unwrap_or(&output.stderr)
                            .to_string()
                    } else {
                        error_line.to_string()
                    };
                    result.issues.push(format!("Probe failed: {message}"));
                }
            }
        }
        Err(e) => {
            if e.to_string().contains("timed out") {
                result.issues.push("Probe timed out".into());
                // Don't assume timeout = needs auth; model may just be slow
                result.suggestions.push(
                    "Model may be slow or waiting for interactive prompt. \
                     Try running the model manually first."
                        .into(),
                );
            } else {
                result.issues.push(format!("Probe error: {e}"));
            }
        }
    }

    result
}

/// Output from running a probe command.
struct ProbeOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

/// Run a probe command for a model.
fn run_probe_command(name: &str, timeout: Duration) -> Result<ProbeOutput, std::io::Error> {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};

    // Explicit prompt to prevent agentic models from reading codebase
    let probe_prompt = "Ping. Just say 'ok' - do not read files or use tools.";

    // Build command based on model
    // Some CLIs take prompt via stdin, others via -p argument
    let (mut cmd, uses_stdin) = match name {
        "claude" => {
            let mut c = Command::new("claude");
            c.args(["-p", "--output-format", "text"]);
            (c, true)
        }
        "codex" => {
            let mut c = Command::new("codex");
            c.args(["exec", "-"]);
            (c, true)
        }
        "gemini" => {
            // Gemini CLI takes prompt as argument to -p, not via stdin
            let mut c = Command::new("gemini");
            c.args(["-p", probe_prompt]);
            (c, false)
        }
        _ => (Command::new(name), true),
    };

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    // Send prompt via stdin if needed
    if uses_stdin {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(probe_prompt.as_bytes());
            let _ = stdin.write_all(b"\n");
        }
    }

    // Wait with timeout
    let start = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let mut stdout = String::new();
            let mut stderr = String::new();

            if let Some(mut out) = child.stdout.take() {
                let _ = out.read_to_string(&mut stdout);
            }
            if let Some(mut err) = child.stderr.take() {
                let _ = err.read_to_string(&mut stderr);
            }

            return Ok(ProbeOutput {
                success: status.success(),
                stdout,
                stderr,
            });
        }

        if start.elapsed() > timeout {
            // Kill the process
            let _ = child.kill();
            let _ = child.wait(); // Reap the zombie

            // Try to capture any output that was written before timeout
            // This helps detect auth prompts that were printed before hanging
            let mut stdout = String::new();
            let mut stderr = String::new();

            if let Some(mut out) = child.stdout.take() {
                let _ = out.read_to_string(&mut stdout);
            }
            if let Some(mut err) = child.stderr.take() {
                let _ = err.read_to_string(&mut stderr);
            }

            // Check if it looks like an auth issue
            // Be careful not to false-positive on success messages like "Loaded cached credentials"
            let combined = format!("{stdout}\n{stderr}").to_lowercase();
            let needs_auth = (combined.contains("auth") && !combined.contains("loaded"))
                || combined.contains("please login")
                || combined.contains("sign in")
                || combined.contains("not authenticated")
                || combined.contains("authentication required")
                || combined.contains("unauthorized");

            if needs_auth {
                // Return as a failed probe with auth info, not a timeout error
                return Ok(ProbeOutput {
                    success: false,
                    stdout,
                    stderr,
                });
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Probe timed out",
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Check if output indicates an auth error (not success messages like "Loaded cached credentials").
fn is_auth_error(output: &str) -> bool {
    // Positive patterns indicating auth is needed
    let needs_auth_patterns = [
        "not authenticated",
        "authentication required",
        "unauthorized",
        "please login",
        "please sign in",
        "api key required",
        "api_key required",
        "missing api key",
        "invalid api key",
        "no credentials",
    ];

    // Check for explicit auth error patterns first
    for pattern in needs_auth_patterns {
        if output.contains(pattern) {
            return true;
        }
    }

    // Check for "auth" keyword but exclude success messages
    if output.contains("auth") && !output.contains("loaded") && !output.contains("success") {
        return true;
    }

    // Check for "login" but exclude "please run ... login" success context
    if output.contains("login required") || output.contains("must login") {
        return true;
    }

    false
}

/// Extract rate limit reset time from error message.
///
/// Looks for patterns like:
/// - "try again at Jan 12th, 2026 9:08 PM"
/// - "resets at 2026-01-12T21:08:00"
fn extract_reset_time(output: &str) -> Option<String> {
    // Look for "try again at <datetime>"
    if let Some(idx) = output.find("try again at ") {
        let rest = &output[idx + 13..];
        // Take until end of line or period
        let end = rest.find(['.', '\n']).unwrap_or(rest.len());
        let time_str = rest[..end].trim();
        if !time_str.is_empty() {
            return Some(time_str.to_string());
        }
    }

    // Look for "resets at <datetime>"
    if let Some(idx) = output.find("resets at ") {
        let rest = &output[idx + 10..];
        let end = rest.find(['.', '\n', '"']).unwrap_or(rest.len());
        let time_str = rest[..end].trim();
        if !time_str.is_empty() {
            return Some(time_str.to_string());
        }
    }

    None
}

/// Extract version from command output.
fn extract_version(output: &str) -> Option<String> {
    // Look for common version patterns
    for line in output.lines().take(5) {
        let line = line.trim();
        // Match patterns like "v1.2.3", "1.2.3", "version 1.2.3"
        if line.contains("version")
            || line.starts_with('v')
            || line.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            // Extract just the version number
            let version: String = line
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if !version.is_empty() {
                return Some(version);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_result_serialization() {
        let result = DiscoveryResult {
            models: vec![ModelInfo {
                name: "claude".into(),
                found: true,
                callable: true,
                path: Some("/usr/local/bin/claude".into()),
                version: Some("1.0.0".into()),
                issues: vec![],
            }],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("claude"));
    }

    #[test]
    fn test_probe_result_serialization() {
        let result = ProbeResult {
            name: "claude".into(),
            success: true,
            response_time_ms: Some(100),
            needs_auth: false,
            rate_limited: false,
            rate_limit_reset: None,
            issues: vec![],
            suggestions: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("claude"));
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version("v1.2.3"), Some("1.2.3".into()));
        assert_eq!(extract_version("version 1.2.3"), Some("1.2.3".into()));
        assert_eq!(extract_version("1.2.3"), Some("1.2.3".into()));
        assert_eq!(extract_version("no version here"), None);
    }
}
