//! Changelog generation for ralf engine.
//!
//! This module handles writing per-iteration changelog entries.

use crate::runner::{GitInfo, InvocationResult, VerifierResult};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Data for a changelog entry.
pub struct ChangelogEntry<'a> {
    /// Directory to write changelog to.
    pub changelog_dir: &'a Path,
    /// Run identifier.
    pub run_id: &'a str,
    /// Iteration number.
    pub iteration: u64,
    /// Model invocation result.
    pub invocation: &'a InvocationResult,
    /// Verifier results.
    pub verifier_results: &'a [VerifierResult],
    /// Hash of the prompt.
    pub prompt_hash: &'a str,
    /// Git information.
    pub git_info: &'a GitInfo,
    /// Status of the iteration.
    pub status: IterationStatus,
    /// Reason for the status.
    pub reason: &'a str,
    /// Path to the log file.
    pub log_path: PathBuf,
}

/// Write a changelog entry for an iteration.
pub fn write_changelog_entry(entry: &ChangelogEntry<'_>) -> Result<(), ChangelogError> {
    // Ensure changelog directory exists
    std::fs::create_dir_all(entry.changelog_dir).map_err(ChangelogError::Io)?;

    let changelog_path = entry
        .changelog_dir
        .join(format!("{}.md", entry.invocation.model));

    // Append to existing file or create new
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&changelog_path)
        .map_err(ChangelogError::Io)?;

    // Format verifier results
    let verifier_lines: Vec<String> = entry
        .verifier_results
        .iter()
        .map(|v| {
            let status = if v.passed { "pass" } else { "fail" };
            format!("  - {}: {status}", v.name)
        })
        .collect();

    // Format changed files (limit to 10)
    let changed_files = if entry.git_info.changed_files.len() > 10 {
        format!(
            "{} (and {} more)",
            entry.git_info.changed_files[..10].join(", "),
            entry.git_info.changed_files.len() - 10
        )
    } else {
        entry.git_info.changed_files.join(", ")
    };

    // Write entry
    let run_id = entry.run_id;
    let iteration = entry.iteration;
    let status = entry.status;
    let reason = entry.reason;
    let prompt_hash = entry.prompt_hash;

    writeln!(file, "\n## Run {run_id} — Iteration {iteration}\n").map_err(ChangelogError::Io)?;
    writeln!(file, "- **Model**: {}", entry.invocation.model).map_err(ChangelogError::Io)?;
    writeln!(file, "- **Status**: {status}").map_err(ChangelogError::Io)?;
    writeln!(file, "- **Reason**: {reason}").map_err(ChangelogError::Io)?;
    writeln!(file, "- **Prompt hash**: {prompt_hash}").map_err(ChangelogError::Io)?;
    writeln!(file, "- **Git branch**: {}", entry.git_info.branch).map_err(ChangelogError::Io)?;
    writeln!(file, "- **Git dirty**: {}", entry.git_info.dirty).map_err(ChangelogError::Io)?;
    writeln!(file, "- **Changed files**: {changed_files}").map_err(ChangelogError::Io)?;
    writeln!(file, "- **Verifier results**:").map_err(ChangelogError::Io)?;
    for line in &verifier_lines {
        writeln!(file, "{line}").map_err(ChangelogError::Io)?;
    }
    writeln!(file, "- **Logs**: {}", entry.log_path.display()).map_err(ChangelogError::Io)?;

    Ok(())
}

/// Status of an iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationStatus {
    /// Iteration succeeded.
    Success,
    /// Model was rate limited.
    RateLimited,
    /// Model timed out.
    Timeout,
    /// Model returned an error.
    Error,
    /// Verifiers failed.
    VerifierFailed,
}

impl std::fmt::Display for IterationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::RateLimited => write!(f, "rate_limited"),
            Self::Timeout => write!(f, "timeout"),
            Self::Error => write!(f, "error"),
            Self::VerifierFailed => write!(f, "verifier_failed"),
        }
    }
}

/// Errors that can occur when writing changelogs.
#[derive(Debug, thiserror::Error)]
pub enum ChangelogError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_changelog_entry() {
        let temp_dir = TempDir::new().unwrap();
        let changelog_dir = temp_dir.path().join("changelog");

        let invocation = InvocationResult {
            model: "claude".into(),
            exit_code: Some(0),
            stdout: "output".into(),
            stderr: String::new(),
            rate_limited: false,
            duration_ms: 1000,
            has_promise: true,
        };

        let verifier_results = vec![VerifierResult {
            name: "tests".into(),
            passed: true,
            exit_code: Some(0),
            output: String::new(),
            duration_ms: 500,
        }];

        let git_info = GitInfo {
            branch: "main".into(),
            dirty: false,
            changed_files: vec!["src/lib.rs".into()],
        };

        let entry = ChangelogEntry {
            changelog_dir: &changelog_dir,
            run_id: "abc123",
            iteration: 1,
            invocation: &invocation,
            verifier_results: &verifier_results,
            prompt_hash: "hash123",
            git_info: &git_info,
            status: IterationStatus::Success,
            reason: "All verifiers passed",
            log_path: PathBuf::from(".ralf/runs/abc123/claude.log"),
        };

        let result = write_changelog_entry(&entry);

        assert!(result.is_ok());

        // Verify file was created
        let changelog_path = changelog_dir.join("claude.md");
        assert!(changelog_path.exists());

        let content = std::fs::read_to_string(changelog_path).unwrap();
        assert!(content.contains("Run abc123"));
        assert!(content.contains("Iteration 1"));
        assert!(content.contains("claude"));
    }
}
