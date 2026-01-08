//! Configuration types for ralf engine.
//!
//! This module defines the configuration schema for ralf, including
//! model definitions, verifiers, and runtime settings.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration for ralf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Whether initial setup has been completed.
    #[serde(default)]
    pub setup_completed: bool,

    /// Priority order for model selection.
    #[serde(default = "default_model_priority")]
    pub model_priority: Vec<String>,

    /// Model selection strategy.
    #[serde(default = "default_model_selection")]
    pub model_selection: ModelSelection,

    /// Required verifiers that must pass for completion.
    #[serde(default = "default_required_verifiers")]
    pub required_verifiers: Vec<String>,

    /// The promise text that signals completion.
    #[serde(default = "default_completion_promise")]
    pub completion_promise: String,

    /// Whether to create checkpoint commits after each iteration.
    #[serde(default)]
    pub checkpoint_commits: bool,

    /// Model configurations.
    #[serde(default)]
    pub models: Vec<ModelConfig>,

    /// Verifier configurations.
    #[serde(default)]
    pub verifiers: Vec<VerifierConfig>,
}

fn default_model_priority() -> Vec<String> {
    vec!["claude".into(), "codex".into(), "gemini".into()]
}

fn default_model_selection() -> ModelSelection {
    ModelSelection::RoundRobin
}

fn default_required_verifiers() -> Vec<String> {
    vec!["tests".into()]
}

fn default_completion_promise() -> String {
    "COMPLETE".into()
}

/// Model selection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelSelection {
    /// Rotate through available models, skipping those in cooldown.
    #[default]
    RoundRobin,
    /// Use first non-cooldown model from priority list.
    Priority,
}

/// Configuration for a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name (e.g., "claude", "codex", "gemini").
    pub name: String,

    /// Command and arguments to invoke the model.
    pub command_argv: Vec<String>,

    /// Timeout in seconds for model invocation.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Patterns that indicate rate limiting.
    #[serde(default = "default_rate_limit_patterns")]
    pub rate_limit_patterns: Vec<String>,

    /// Default cooldown duration in seconds when rate limited.
    #[serde(default = "default_cooldown_seconds")]
    pub default_cooldown_seconds: u64,
}

fn default_timeout() -> u64 {
    300
}

fn default_rate_limit_patterns() -> Vec<String> {
    vec![
        "429".into(),
        "rate limit".into(),
        "quota".into(),
        "too many requests".into(),
    ]
}

fn default_cooldown_seconds() -> u64 {
    900
}

/// Configuration for a verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierConfig {
    /// Verifier name.
    pub name: String,

    /// Command and arguments to run the verifier.
    pub command_argv: Vec<String>,

    /// Timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// When to run this verifier.
    #[serde(default)]
    pub run_when: VerifierRunWhen,
}

/// When to run a verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerifierRunWhen {
    /// Run on every change.
    #[default]
    OnChange,
    /// Always run.
    Always,
}

impl Config {
    /// Load configuration from a file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        serde_json::from_str(&content).map_err(ConfigError::Parse)
    }

    /// Save configuration to a file.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(self).map_err(ConfigError::Serialize)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        std::fs::write(path, content).map_err(ConfigError::Io)
    }

    /// Create a default configuration with the given detected models.
    pub fn with_detected_models(model_names: &[String]) -> Self {
        let models = model_names
            .iter()
            .map(|name| ModelConfig::default_for(name))
            .collect();

        Self {
            model_priority: model_names.to_vec(),
            models,
            verifiers: vec![VerifierConfig::default_tests()],
            ..Default::default()
        }
    }

    /// Get the model config by name.
    pub fn get_model(&self, name: &str) -> Option<&ModelConfig> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Get the verifier config by name.
    pub fn get_verifier(&self, name: &str) -> Option<&VerifierConfig> {
        self.verifiers.iter().find(|v| v.name == name)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            setup_completed: false,
            model_priority: default_model_priority(),
            model_selection: default_model_selection(),
            required_verifiers: default_required_verifiers(),
            completion_promise: default_completion_promise(),
            checkpoint_commits: false,
            models: Vec::new(),
            verifiers: vec![VerifierConfig::default_tests()],
        }
    }
}

impl ModelConfig {
    /// Create a default configuration for a known model.
    pub fn default_for(name: &str) -> Self {
        match name {
            "claude" => Self {
                name: "claude".into(),
                command_argv: vec![
                    "claude".into(),
                    "-p".into(),
                    "--output-format".into(),
                    "text".into(),
                    "--dangerously-skip-permissions".into(),
                ],
                timeout_seconds: 300,
                rate_limit_patterns: default_rate_limit_patterns(),
                default_cooldown_seconds: 900,
            },
            "codex" => Self {
                name: "codex".into(),
                command_argv: vec![
                    "codex".into(),
                    "exec".into(),
                    "--dangerously-bypass-approvals-and-sandbox".into(),
                    "-".into(),
                ],
                timeout_seconds: 300,
                rate_limit_patterns: default_rate_limit_patterns(),
                default_cooldown_seconds: 900,
            },
            "gemini" => Self {
                name: "gemini".into(),
                command_argv: vec!["gemini".into(), "-p".into()],
                timeout_seconds: 300,
                rate_limit_patterns: default_rate_limit_patterns(),
                default_cooldown_seconds: 900,
            },
            _ => Self {
                name: name.into(),
                command_argv: vec![name.into()],
                timeout_seconds: 300,
                rate_limit_patterns: default_rate_limit_patterns(),
                default_cooldown_seconds: 900,
            },
        }
    }
}

impl VerifierConfig {
    /// Create a default "tests" verifier using cargo test.
    pub fn default_tests() -> Self {
        Self {
            name: "tests".into(),
            command_argv: vec!["cargo".into(), "test".into()],
            timeout_seconds: 300,
            run_when: VerifierRunWhen::OnChange,
        }
    }
}

/// Errors that can occur when working with configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// I/O error reading or writing config.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error parsing config JSON.
    #[error("Parse error: {0}")]
    Parse(#[source] serde_json::Error),

    /// Error serializing config to JSON.
    #[error("Serialize error: {0}")]
    Serialize(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model_selection, ModelSelection::RoundRobin);
        assert_eq!(config.completion_promise, "COMPLETE");
        assert_eq!(config.required_verifiers, vec!["tests"]);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::with_detected_models(&["claude".into(), "codex".into()]);
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model_priority, config.model_priority);
    }

    #[test]
    fn test_model_config_defaults() {
        let claude = ModelConfig::default_for("claude");
        assert_eq!(claude.name, "claude");
        assert!(claude.command_argv.contains(&"claude".to_string()));

        let codex = ModelConfig::default_for("codex");
        assert_eq!(codex.name, "codex");

        let gemini = ModelConfig::default_for("gemini");
        assert_eq!(gemini.name, "gemini");
    }
}
