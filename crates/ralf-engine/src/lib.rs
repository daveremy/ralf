//! ralf-engine: Headless engine for multi-model autonomous loops
//!
//! This crate provides the core orchestration logic for ralf, including:
//! - Configuration and state management
//! - Model adapters for CLI process execution
//! - Rate-limit detection and cooldown management
//! - Verification runners
//! - Changelog generation

pub mod changelog;
pub mod config;
pub mod discovery;
pub mod runner;
pub mod state;

// Re-export commonly used types
pub use changelog::{write_changelog_entry, ChangelogEntry, ChangelogError, IterationStatus};
pub use config::{Config, ConfigError, ModelConfig, ModelSelection, VerifierConfig};
pub use discovery::{discover_model, discover_models, probe_model, DiscoveryResult, ModelInfo, ProbeResult};
pub use runner::{
    check_promise, extract_promise, get_git_info, hash_prompt, invoke_model, run_verifier,
    select_model, GitInfo, InvocationResult, RunnerError, VerifierResult,
};
pub use state::{Cooldowns, RunState, RunStatus, StateError};

/// Returns the engine version.
pub fn engine_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_version() {
        let version = engine_version();
        assert!(!version.is_empty());
        assert!(version.starts_with("0."));
    }
}
