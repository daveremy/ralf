//! Model status types for the TUI.
//!
//! These types wrap engine discovery/probe results for display purposes.

use ralf_engine::discovery::{ModelInfo, ProbeResult};

/// Install URLs for each model CLI.
const INSTALL_URLS: &[(&str, &str)] = &[
    ("claude", "https://docs.anthropic.com/claude/docs/claude-code"),
    ("codex", "https://github.com/openai/codex"),
    ("gemini", "https://github.com/google/generative-ai-cli"),
];

/// Model state for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelState {
    /// Currently checking model status.
    Probing,
    /// Model probed successfully and is ready.
    Ready,
    /// Model rate-limited during probe (with optional reset time).
    RateLimited(Option<String>),
    /// Model rate-limited, seconds remaining (forward-looking, not used this phase).
    Cooldown(u64),
    /// Model not found, auth error, or probe failed.
    Unavailable,
}

impl ModelState {
    /// Get the status indicator character.
    ///
    /// - `●` (ready) - Model probed successfully
    /// - `◐` (cooldown) - Model rate-limited (temporary)
    /// - `○` (unavailable) - Not found, auth error, or probe failed
    /// - `◌` (probing) - Currently checking
    /// - `◉` (rate limited) - Hit usage/quota limit
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::Ready => "●",
            Self::RateLimited(_) => "◉",
            Self::Cooldown(_) => "◐",
            Self::Unavailable => "○",
            Self::Probing => "◌",
        }
    }

    /// Get the ASCII indicator for `NO_COLOR` mode.
    ///
    /// - `[x]` (ready)
    /// - `[!]` (rate limited)
    /// - `[~]` (cooldown)
    /// - `[ ]` (unavailable)
    /// - `[?]` (probing)
    pub fn indicator_ascii(&self) -> &'static str {
        match self {
            Self::Ready => "[x]",
            Self::RateLimited(_) => "[!]",
            Self::Cooldown(_) => "[~]",
            Self::Unavailable => "[ ]",
            Self::Probing => "[?]",
        }
    }
}

/// Model status combining discovery and probe results for display.
#[derive(Debug, Clone)]
pub struct ModelStatus {
    /// Model name (e.g., "claude", "codex", "gemini").
    pub name: String,
    /// Current state.
    pub state: ModelState,
    /// Version string if available.
    pub version: Option<String>,
    /// User-friendly status or error message.
    pub message: Option<String>,
}

impl ModelStatus {
    /// Create a probing placeholder status.
    pub fn probing(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ModelState::Probing,
            version: None,
            message: Some("Checking...".to_string()),
        }
    }

    /// Create from engine discovery and probe results.
    pub fn from_engine(info: &ModelInfo, probe: Option<&ProbeResult>) -> Self {
        let (state, message) = Self::determine_state_and_message(info, probe);

        Self {
            name: info.name.clone(),
            state,
            version: info.version.clone(),
            message,
        }
    }

    /// Determine state and message from engine results.
    fn determine_state_and_message(
        info: &ModelInfo,
        probe: Option<&ProbeResult>,
    ) -> (ModelState, Option<String>) {
        // Not found on PATH
        if !info.found {
            let url = Self::install_url(&info.name);
            let message = format!("Not found. Install: {url}");
            return (ModelState::Unavailable, Some(message));
        }

        // Not callable (--help failed)
        if !info.callable {
            let message = info.issues.first().map_or_else(
                || "Not responding".to_string(),
                |issue| format!("Error: {issue}"),
            );
            return (ModelState::Unavailable, Some(message));
        }

        // Check probe result
        match probe {
            Some(p) if p.success => (ModelState::Ready, Some("Ready".to_string())),
            Some(p) if p.rate_limited => {
                // Rate limited - show reset time if available
                let message = match &p.rate_limit_reset {
                    Some(reset) => format!("Rate limited (resets: {reset})"),
                    None => "Rate limited".to_string(),
                };
                (
                    ModelState::RateLimited(p.rate_limit_reset.clone()),
                    Some(message),
                )
            }
            Some(p) if p.needs_auth => {
                let message = format!("Needs auth. Run: `{} auth login`", info.name);
                (ModelState::Unavailable, Some(message))
            }
            Some(p) => {
                // Probe failed for other reason
                let message = p.issues.first().map_or_else(
                    || "Probe failed".to_string(),
                    |issue| {
                        if issue.contains("timed out") {
                            "Not responding (10s timeout)".to_string()
                        } else {
                            format!("Error: {issue}")
                        }
                    },
                );
                (ModelState::Unavailable, Some(message))
            }
            None => {
                // No probe yet (shouldn't happen after startup)
                (ModelState::Probing, Some("Checking...".to_string()))
            }
        }
    }

    /// Get the install URL for a model.
    fn install_url(name: &str) -> &'static str {
        INSTALL_URLS
            .iter()
            .find(|(n, _)| *n == name)
            .map_or("https://github.com", |(_, url)| url)
    }

    /// Get the status indicator (Unicode or ASCII based on mode).
    pub fn indicator(&self, ascii_mode: bool) -> &'static str {
        if ascii_mode {
            self.state.indicator_ascii()
        } else {
            self.state.indicator()
        }
    }

    /// Check if this model is ready.
    pub fn is_ready(&self) -> bool {
        matches!(self.state, ModelState::Ready)
    }
}

/// Summary of model statuses for status bar display.
#[derive(Debug, Clone)]
pub struct ModelsSummary {
    /// Count of ready models.
    pub ready: usize,
    /// Total model count.
    pub total: usize,
    /// Whether probing is still in progress.
    pub probing: bool,
}

impl ModelsSummary {
    /// Create from a list of model statuses.
    pub fn from_models(models: &[ModelStatus]) -> Self {
        let ready = models.iter().filter(|m| m.is_ready()).count();
        let probing = models
            .iter()
            .any(|m| matches!(m.state, ModelState::Probing));

        Self {
            ready,
            total: models.len(),
            probing,
        }
    }

    /// Format for narrow terminals (e.g., "2/3 models").
    pub fn narrow_format(&self) -> String {
        if self.probing {
            "Checking...".to_string()
        } else {
            format!("{}/{} models", self.ready, self.total)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_model_info(name: &str, found: bool, callable: bool) -> ModelInfo {
        ModelInfo {
            name: name.to_string(),
            found,
            callable,
            path: if found {
                Some(format!("/usr/bin/{name}"))
            } else {
                None
            },
            version: if callable {
                Some("1.0.0".to_string())
            } else {
                None
            },
            issues: if !found {
                vec![format!("{name} not found on PATH")]
            } else {
                vec![]
            },
        }
    }

    fn mock_probe_result(name: &str, success: bool, needs_auth: bool) -> ProbeResult {
        ProbeResult {
            name: name.to_string(),
            success,
            response_time_ms: Some(100),
            needs_auth,
            rate_limited: false,
            rate_limit_reset: None,
            issues: if !success && !needs_auth {
                vec!["Probe failed".to_string()]
            } else {
                vec![]
            },
            suggestions: vec![],
        }
    }

    #[test]
    fn test_model_state_indicators() {
        assert_eq!(ModelState::Ready.indicator(), "●");
        assert_eq!(ModelState::Unavailable.indicator(), "○");
        assert_eq!(ModelState::Cooldown(60).indicator(), "◐");
        assert_eq!(ModelState::Probing.indicator(), "◌");
        assert_eq!(ModelState::RateLimited(None).indicator(), "◉");

        assert_eq!(ModelState::Ready.indicator_ascii(), "[x]");
        assert_eq!(ModelState::Unavailable.indicator_ascii(), "[ ]");
        assert_eq!(ModelState::RateLimited(None).indicator_ascii(), "[!]");
    }

    #[test]
    fn test_model_status_ready() {
        let info = mock_model_info("claude", true, true);
        let probe = mock_probe_result("claude", true, false);
        let status = ModelStatus::from_engine(&info, Some(&probe));

        assert_eq!(status.name, "claude");
        assert_eq!(status.state, ModelState::Ready);
        assert_eq!(status.message, Some("Ready".to_string()));
        assert!(status.is_ready());
    }

    #[test]
    fn test_model_status_not_found() {
        let info = mock_model_info("codex", false, false);
        let status = ModelStatus::from_engine(&info, None);

        assert_eq!(status.state, ModelState::Unavailable);
        assert!(status.message.as_ref().unwrap().contains("Not found"));
        assert!(status.message.as_ref().unwrap().contains("Install:"));
    }

    #[test]
    fn test_model_status_needs_auth() {
        let info = mock_model_info("gemini", true, true);
        let probe = mock_probe_result("gemini", false, true);
        let status = ModelStatus::from_engine(&info, Some(&probe));

        assert_eq!(status.state, ModelState::Unavailable);
        assert!(status.message.as_ref().unwrap().contains("Needs auth"));
        assert!(status.message.as_ref().unwrap().contains("auth login"));
    }

    #[test]
    fn test_model_status_probing() {
        let status = ModelStatus::probing("claude");

        assert_eq!(status.state, ModelState::Probing);
        assert_eq!(status.message, Some("Checking...".to_string()));
    }

    #[test]
    fn test_models_summary() {
        let models = vec![
            ModelStatus::from_engine(
                &mock_model_info("claude", true, true),
                Some(&mock_probe_result("claude", true, false)),
            ),
            ModelStatus::from_engine(
                &mock_model_info("codex", false, false),
                None,
            ),
            ModelStatus::from_engine(
                &mock_model_info("gemini", true, true),
                Some(&mock_probe_result("gemini", true, false)),
            ),
        ];

        let summary = ModelsSummary::from_models(&models);
        assert_eq!(summary.ready, 2);
        assert_eq!(summary.total, 3);
        assert!(!summary.probing);
        assert_eq!(summary.narrow_format(), "2/3 models");
    }

    #[test]
    fn test_models_summary_probing() {
        let models = vec![ModelStatus::probing("claude")];
        let summary = ModelsSummary::from_models(&models);

        assert!(summary.probing);
        assert_eq!(summary.narrow_format(), "Checking...");
    }
}
