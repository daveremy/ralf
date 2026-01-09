//! Preflight checks for thread implementation.
//!
//! Validates all prerequisites before a thread can transition from Finalized
//! to Preflight phase. Ensures git safety, spec validity, model availability,
//! verifier availability, and single-run enforcement.

use std::collections::HashSet;
use std::path::Path;

use crate::chat::draft_has_promise;
use crate::config::Config;
use crate::git::GitSafety;
use crate::parse_criteria;
use crate::persistence::ThreadStore;
use crate::thread::Thread;

/// Result of running preflight checks.
#[derive(Debug, Clone)]
pub struct PreflightResult {
    /// Whether all checks passed.
    pub passed: bool,
    /// Individual check results.
    pub checks: Vec<PreflightCheck>,
}

impl PreflightResult {
    /// Get the first failing check, if any.
    pub fn first_failure(&self) -> Option<&PreflightCheck> {
        self.checks.iter().find(|c| !c.passed)
    }

    /// Get a summary message suitable for display.
    pub fn summary(&self) -> String {
        let passed_count = self.checks.iter().filter(|c| c.passed).count();
        let total = self.checks.len();

        if self.passed {
            format!("All {total} preflight checks passed")
        } else {
            let failed_count = total - passed_count;
            let failures: Vec<_> = self
                .checks
                .iter()
                .filter(|c| !c.passed)
                .map(|c| c.label.as_str())
                .collect();
            format!(
                "{failed_count} of {total} check(s) failed: {}",
                failures.join(", ")
            )
        }
    }
}

/// A single preflight check result.
#[derive(Debug, Clone)]
pub struct PreflightCheck {
    /// Check identifier (e.g., `git_clean`, `spec_valid`).
    pub name: String,
    /// Human-readable label (e.g., "Git Working Tree").
    pub label: String,
    /// Whether this check passed.
    pub passed: bool,
    /// Descriptive message (success or failure reason).
    pub message: String,
}

/// Run all preflight checks for a thread.
///
/// Checks are run in order, but all checks run regardless of earlier failures
/// (so user sees all issues at once).
///
/// # Arguments
/// * `thread` - The thread to validate (must be in Finalized phase)
/// * `repo_path` - Path to the repository root
/// * `store` - `ThreadStore` for checking other threads
/// * `config` - Application configuration
///
/// # Returns
/// `PreflightResult` with all check outcomes
pub fn run_preflight(
    thread: &Thread,
    repo_path: &Path,
    store: &ThreadStore,
    config: &Config,
) -> PreflightResult {
    // Run all checks in order
    let checks = vec![
        check_git_state(thread, repo_path),
        check_baseline_capturable(repo_path),
        check_spec_has_promise(thread, store),
        check_criteria_parseable(thread, store),
        check_models_available(thread, config),
        check_verifiers_available(config),
        check_no_concurrent_run(thread, store),
    ];

    let passed = checks.iter().all(|c| c.passed);

    PreflightResult { passed, checks }
}

/// Check 1: Git working tree is clean or on a ralf-managed branch.
///
/// Passes if:
/// - Not a git repository (git safety disabled), OR
/// - Working tree is clean (no uncommitted changes), OR
/// - Currently on a ralf/<thread-id> branch for this thread
fn check_git_state(thread: &Thread, repo_path: &Path) -> PreflightCheck {
    let git = GitSafety::new(repo_path);

    // Not a git repo? Pass with warning (user's choice to run without git safety)
    if !git.is_repo() {
        return PreflightCheck {
            name: "git_state".to_string(),
            label: "Git Working Tree".to_string(),
            passed: true,
            message: "Not a git repository (git safety disabled)".to_string(),
        };
    }

    // Clean working tree? Pass.
    if git.is_clean().unwrap_or(false) {
        return PreflightCheck {
            name: "git_state".to_string(),
            label: "Git Working Tree".to_string(),
            passed: true,
            message: "Working tree is clean".to_string(),
        };
    }

    // On this thread's branch? Pass (resuming previous work).
    if let Ok(branch) = git.current_branch() {
        let thread_branch = format!("ralf/{}", thread.id);
        if branch == thread_branch {
            return PreflightCheck {
                name: "git_state".to_string(),
                label: "Git Working Tree".to_string(),
                passed: true,
                message: format!("On thread branch {thread_branch}"),
            };
        }
    }

    // Dirty on non-thread branch? Fail.
    PreflightCheck {
        name: "git_state".to_string(),
        label: "Git Working Tree".to_string(),
        passed: false,
        message: "Working tree has uncommitted changes. Commit or stash before running."
            .to_string(),
    }
}

/// Check 2: Git baseline can be captured.
///
/// Passes if:
/// - Not a git repository (git safety disabled), OR
/// - We can get current branch name (not detached HEAD)
/// - We can get current commit SHA
fn check_baseline_capturable(repo_path: &Path) -> PreflightCheck {
    let git = GitSafety::new(repo_path);

    // Not a git repo? Pass (no baseline needed)
    if !git.is_repo() {
        return PreflightCheck {
            name: "baseline_capturable".to_string(),
            label: "Git Baseline".to_string(),
            passed: true,
            message: "Not a git repository (no baseline needed)".to_string(),
        };
    }

    // Check if we can get branch and commit
    match (git.current_branch(), git.head_sha()) {
        (Ok(branch), Ok(commit)) => {
            let short_sha = if commit.len() >= 8 {
                &commit[..8]
            } else {
                &commit
            };
            PreflightCheck {
                name: "baseline_capturable".to_string(),
                label: "Git Baseline".to_string(),
                passed: true,
                message: format!("Branch: {branch}, Commit: {short_sha}"),
            }
        }
        (Err(_), _) => PreflightCheck {
            name: "baseline_capturable".to_string(),
            label: "Git Baseline".to_string(),
            passed: false,
            message: "Cannot determine current branch (detached HEAD?)".to_string(),
        },
        (_, Err(_)) => PreflightCheck {
            name: "baseline_capturable".to_string(),
            label: "Git Baseline".to_string(),
            passed: false,
            message: "Cannot get current commit SHA".to_string(),
        },
    }
}

/// Check 3: Spec has a promise tag.
///
/// Passes if:
/// - Thread has at least one spec revision saved
/// - Latest spec contains a promise tag (<promise>...</promise>)
fn check_spec_has_promise(thread: &Thread, store: &ThreadStore) -> PreflightCheck {
    // Try to load the latest spec
    let spec_content = match store.load_latest_spec(&thread.id) {
        Ok(Some(content)) => content,
        Ok(None) => {
            return PreflightCheck {
                name: "spec_has_promise".to_string(),
                label: "Spec Promise".to_string(),
                passed: false,
                message: "No spec saved for this thread".to_string(),
            };
        }
        Err(e) => {
            return PreflightCheck {
                name: "spec_has_promise".to_string(),
                label: "Spec Promise".to_string(),
                passed: false,
                message: format!("Failed to load spec: {e}"),
            };
        }
    };

    // Check for promise tag
    if draft_has_promise(&spec_content) {
        PreflightCheck {
            name: "spec_has_promise".to_string(),
            label: "Spec Promise".to_string(),
            passed: true,
            message: "Spec contains promise tag".to_string(),
        }
    } else {
        PreflightCheck {
            name: "spec_has_promise".to_string(),
            label: "Spec Promise".to_string(),
            passed: false,
            message: "Spec is missing <promise>...</promise> tag".to_string(),
        }
    }
}

/// Check 4: Completion criteria are parseable.
///
/// Passes if:
/// - Spec has a criteria section (## Requirements, ## Criteria, etc.)
/// - At least one criterion can be extracted
fn check_criteria_parseable(thread: &Thread, store: &ThreadStore) -> PreflightCheck {
    // Try to load the latest spec
    let spec_content = match store.load_latest_spec(&thread.id) {
        Ok(Some(content)) => content,
        Ok(None) => {
            return PreflightCheck {
                name: "criteria_parseable".to_string(),
                label: "Completion Criteria".to_string(),
                passed: false,
                message: "No spec saved for this thread".to_string(),
            };
        }
        Err(e) => {
            return PreflightCheck {
                name: "criteria_parseable".to_string(),
                label: "Completion Criteria".to_string(),
                passed: false,
                message: format!("Failed to load spec: {e}"),
            };
        }
    };

    // Parse criteria
    let criteria = parse_criteria(&spec_content);

    if criteria.is_empty() {
        PreflightCheck {
            name: "criteria_parseable".to_string(),
            label: "Completion Criteria".to_string(),
            passed: false,
            message: "No completion criteria found in spec".to_string(),
        }
    } else {
        PreflightCheck {
            name: "criteria_parseable".to_string(),
            label: "Completion Criteria".to_string(),
            passed: true,
            message: format!("Found {} criterion/criteria", criteria.len()),
        }
    }
}

/// Check 5: At least one model is configured.
///
/// Passes if:
/// - Config has at least one model in models vec, OR
/// - Thread has `run_config` with at least one model
fn check_models_available(thread: &Thread, config: &Config) -> PreflightCheck {
    // Check thread's run_config first
    if let Some(run_config) = &thread.run_config {
        if !run_config.models.is_empty() {
            return PreflightCheck {
                name: "models_available".to_string(),
                label: "Model Availability".to_string(),
                passed: true,
                message: format!(
                    "{} model(s) configured in thread",
                    run_config.models.len()
                ),
            };
        }
    }

    // Fall back to config models
    if config.models.is_empty() {
        PreflightCheck {
            name: "models_available".to_string(),
            label: "Model Availability".to_string(),
            passed: false,
            message: "No models configured. Add models to ralf.toml or thread config.".to_string(),
        }
    } else {
        PreflightCheck {
            name: "models_available".to_string(),
            label: "Model Availability".to_string(),
            passed: true,
            message: format!("{} model(s) configured globally", config.models.len()),
        }
    }
}

/// Check 6: Required verifiers are configured.
///
/// Passes if:
/// - All verifiers listed in `config.required_verifiers` exist in `config.verifiers`
fn check_verifiers_available(config: &Config) -> PreflightCheck {
    // If no verifiers required, pass
    if config.required_verifiers.is_empty() {
        return PreflightCheck {
            name: "verifiers_available".to_string(),
            label: "Required Verifiers".to_string(),
            passed: true,
            message: "No verifiers required".to_string(),
        };
    }

    // Check each required verifier exists
    let configured_names: HashSet<_> = config.verifiers.iter().map(|v| &v.name).collect();
    let missing: Vec<_> = config
        .required_verifiers
        .iter()
        .filter(|name| !configured_names.contains(name))
        .collect();

    if missing.is_empty() {
        PreflightCheck {
            name: "verifiers_available".to_string(),
            label: "Required Verifiers".to_string(),
            passed: true,
            message: format!(
                "{} required verifier(s) configured",
                config.required_verifiers.len()
            ),
        }
    } else {
        let missing_list: Vec<_> = missing.iter().map(|s| s.as_str()).collect();
        PreflightCheck {
            name: "verifiers_available".to_string(),
            label: "Required Verifiers".to_string(),
            passed: false,
            message: format!("Missing verifiers: {}", missing_list.join(", ")),
        }
    }
}

/// Check 7: No other thread is currently Running.
///
/// Passes if:
/// - No other thread in the store is in Running, Verifying, or Paused phase
/// - (The current thread being in Finalized/Preflight is expected)
fn check_no_concurrent_run(thread: &Thread, store: &ThreadStore) -> PreflightCheck {
    let threads = store.list().unwrap_or_default();

    for summary in threads {
        if summary.id == thread.id {
            continue; // Skip self
        }

        // Check if another thread is in an active run phase
        // ThreadSummary.phase contains the display name from phase_display_name()
        let active_phases = ["Running", "Verifying", "Paused"];
        if active_phases.contains(&summary.phase.as_str()) {
            return PreflightCheck {
                name: "no_concurrent_run".to_string(),
                label: "Concurrent Runs".to_string(),
                passed: false,
                message: format!(
                    "Thread '{}' is currently {}. Only one thread can run at a time.",
                    summary.title,
                    summary.phase.to_lowercase()
                ),
            };
        }
    }

    PreflightCheck {
        name: "no_concurrent_run".to_string(),
        label: "Concurrent Runs".to_string(),
        passed: true,
        message: "No other threads running".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ModelConfig, VerifierConfig};
    use crate::thread::ThreadPhase;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    /// Setup a test repository with git initialized.
    fn setup_git_repo() -> TempDir {
        let temp = TempDir::new().unwrap();

        Command::new("git")
            .arg("init")
            .current_dir(temp.path())
            .output()
            .expect("git init failed");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp.path())
            .output()
            .expect("git config email failed");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp.path())
            .output()
            .expect("git config name failed");

        let readme_path = temp.path().join("README.md");
        fs::write(&readme_path, "# Test Repo\n").unwrap();

        // Add .gitignore to ignore threads directory (test data)
        let gitignore_path = temp.path().join(".gitignore");
        fs::write(&gitignore_path, "threads/\n").unwrap();

        Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(temp.path())
            .output()
            .expect("git add failed");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp.path())
            .output()
            .expect("git commit failed");

        temp
    }

    /// Setup a test environment with ThreadStore.
    fn setup_test_env() -> (TempDir, ThreadStore) {
        let temp = setup_git_repo();
        let store = ThreadStore::new(temp.path()).unwrap();
        (temp, store)
    }

    fn default_config_with_models() -> Config {
        Config {
            setup_completed: true,
            models: vec![ModelConfig {
                name: "test-model".to_string(),
                command_argv: vec!["echo".to_string()],
                timeout_seconds: 300,
                rate_limit_patterns: vec![],
                default_cooldown_seconds: 900,
            }],
            verifiers: vec![VerifierConfig {
                name: "tests".to_string(),
                command_argv: vec!["cargo".to_string(), "test".to_string()],
                timeout_seconds: 300,
                run_when: crate::config::VerifierRunWhen::OnChange,
            }],
            required_verifiers: vec!["tests".to_string()],
            ..Default::default()
        }
    }

    fn create_thread_with_spec(store: &ThreadStore, has_promise: bool, has_criteria: bool) -> Thread {
        let mut thread = Thread::new("Test Thread");
        thread.phase = ThreadPhase::Finalized;
        store.save(&thread).unwrap();

        let mut spec = String::new();
        if has_promise {
            spec.push_str("<promise>\nImplement the feature\n</promise>\n\n");
        }
        if has_criteria {
            spec.push_str("## Requirements\n\n- [ ] First requirement\n- [ ] Second requirement\n");
        }

        store.save_spec(&thread.id, &spec).unwrap();
        thread
    }

    // Test: PreflightResult methods
    #[test]
    fn test_preflight_result_first_failure() {
        let result = PreflightResult {
            passed: false,
            checks: vec![
                PreflightCheck {
                    name: "check1".to_string(),
                    label: "Check 1".to_string(),
                    passed: true,
                    message: "OK".to_string(),
                },
                PreflightCheck {
                    name: "check2".to_string(),
                    label: "Check 2".to_string(),
                    passed: false,
                    message: "Failed".to_string(),
                },
            ],
        };

        let failure = result.first_failure().unwrap();
        assert_eq!(failure.name, "check2");
    }

    #[test]
    fn test_preflight_result_first_failure_none() {
        let result = PreflightResult {
            passed: true,
            checks: vec![PreflightCheck {
                name: "check1".to_string(),
                label: "Check 1".to_string(),
                passed: true,
                message: "OK".to_string(),
            }],
        };

        assert!(result.first_failure().is_none());
    }

    #[test]
    fn test_preflight_result_summary_passed() {
        let result = PreflightResult {
            passed: true,
            checks: vec![
                PreflightCheck {
                    name: "a".to_string(),
                    label: "A".to_string(),
                    passed: true,
                    message: "OK".to_string(),
                },
                PreflightCheck {
                    name: "b".to_string(),
                    label: "B".to_string(),
                    passed: true,
                    message: "OK".to_string(),
                },
            ],
        };

        assert_eq!(result.summary(), "All 2 preflight checks passed");
    }

    #[test]
    fn test_preflight_result_summary_failed() {
        let result = PreflightResult {
            passed: false,
            checks: vec![
                PreflightCheck {
                    name: "a".to_string(),
                    label: "Check A".to_string(),
                    passed: true,
                    message: "OK".to_string(),
                },
                PreflightCheck {
                    name: "b".to_string(),
                    label: "Check B".to_string(),
                    passed: false,
                    message: "Failed".to_string(),
                },
                PreflightCheck {
                    name: "c".to_string(),
                    label: "Check C".to_string(),
                    passed: false,
                    message: "Also failed".to_string(),
                },
            ],
        };

        let summary = result.summary();
        assert!(summary.contains("2 of 3"));
        assert!(summary.contains("Check B"));
        assert!(summary.contains("Check C"));
    }

    // Test: check_git_state
    #[test]
    fn test_check_git_state_clean() {
        let temp = setup_git_repo();
        let thread = Thread::new("test");

        let check = check_git_state(&thread, temp.path());
        assert!(check.passed);
        assert!(check.message.contains("clean"));
    }

    #[test]
    fn test_check_git_state_dirty() {
        let temp = setup_git_repo();
        let thread = Thread::new("test");

        // Make dirty
        fs::write(temp.path().join("dirty.txt"), "dirty").unwrap();

        let check = check_git_state(&thread, temp.path());
        assert!(!check.passed);
        assert!(check.message.contains("uncommitted"));
    }

    #[test]
    fn test_check_git_state_on_thread_branch() {
        let temp = setup_git_repo();
        let mut thread = Thread::new("test");
        // Use a clean thread ID for branch name
        thread.id = "test-branch-id".to_string();

        // Create and checkout thread branch
        let git = GitSafety::new(temp.path());
        git.create_thread_branch("test-branch-id").unwrap();
        git.checkout("ralf/test-branch-id").unwrap();

        // Make dirty (should still pass because on thread branch)
        fs::write(temp.path().join("work.txt"), "work").unwrap();

        let check = check_git_state(&thread, temp.path());
        assert!(check.passed);
        assert!(check.message.contains("thread branch"));
    }

    #[test]
    fn test_check_git_state_non_git_repo() {
        let temp = TempDir::new().unwrap();
        let thread = Thread::new("test");

        let check = check_git_state(&thread, temp.path());
        assert!(check.passed);
        assert!(check.message.contains("Not a git repository"));
    }

    // Test: check_baseline_capturable
    #[test]
    fn test_check_baseline_capturable_success() {
        let temp = setup_git_repo();

        let check = check_baseline_capturable(temp.path());
        assert!(check.passed);
        assert!(check.message.contains("Branch:"));
        assert!(check.message.contains("Commit:"));
    }

    #[test]
    fn test_check_baseline_capturable_detached_head() {
        let temp = setup_git_repo();
        let git = GitSafety::new(temp.path());

        // Get current SHA and checkout detached
        let sha = git.head_sha().unwrap();
        Command::new("git")
            .args(["checkout", "--detach", &sha])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let check = check_baseline_capturable(temp.path());
        assert!(!check.passed);
        assert!(check.message.contains("detached HEAD"));
    }

    #[test]
    fn test_check_baseline_capturable_non_git_repo() {
        let temp = TempDir::new().unwrap();

        let check = check_baseline_capturable(temp.path());
        assert!(check.passed);
        assert!(check.message.contains("Not a git repository"));
    }

    // Test: check_spec_has_promise
    #[test]
    fn test_check_spec_has_promise_success() {
        let (_temp, store) = setup_test_env();
        let thread = create_thread_with_spec(&store, true, true);

        let check = check_spec_has_promise(&thread, &store);
        assert!(check.passed);
        assert!(check.message.contains("promise tag"));
    }

    #[test]
    fn test_check_spec_has_promise_missing() {
        let (_temp, store) = setup_test_env();
        let thread = create_thread_with_spec(&store, false, true);

        let check = check_spec_has_promise(&thread, &store);
        assert!(!check.passed);
        assert!(check.message.contains("missing"));
    }

    #[test]
    fn test_check_spec_has_promise_no_spec() {
        let (_temp, store) = setup_test_env();
        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Finalized;
        store.save(&thread).unwrap();
        // Don't save spec

        let check = check_spec_has_promise(&thread, &store);
        assert!(!check.passed);
        assert!(check.message.contains("No spec saved"));
    }

    // Test: check_criteria_parseable
    #[test]
    fn test_check_criteria_parseable_success() {
        let (_temp, store) = setup_test_env();
        let thread = create_thread_with_spec(&store, true, true);

        let check = check_criteria_parseable(&thread, &store);
        assert!(check.passed);
        assert!(check.message.contains("2 criterion"));
    }

    #[test]
    fn test_check_criteria_parseable_missing() {
        let (_temp, store) = setup_test_env();
        let thread = create_thread_with_spec(&store, true, false);

        let check = check_criteria_parseable(&thread, &store);
        assert!(!check.passed);
        assert!(check.message.contains("No completion criteria"));
    }

    // Test: check_models_available
    #[test]
    fn test_check_models_available_from_config() {
        let thread = Thread::new("test");
        let config = default_config_with_models();

        let check = check_models_available(&thread, &config);
        assert!(check.passed);
        assert!(check.message.contains("globally"));
    }

    #[test]
    fn test_check_models_available_from_thread() {
        let mut thread = Thread::new("test");
        thread.run_config = Some(crate::thread::RunConfig {
            models: vec!["model1".to_string()],
            max_iterations: 5,
        });

        let config = Config::default();

        let check = check_models_available(&thread, &config);
        assert!(check.passed);
        assert!(check.message.contains("in thread"));
    }

    #[test]
    fn test_check_models_available_none() {
        let thread = Thread::new("test");
        let config = Config::default();

        let check = check_models_available(&thread, &config);
        assert!(!check.passed);
        assert!(check.message.contains("No models configured"));
    }

    // Test: check_verifiers_available
    #[test]
    fn test_check_verifiers_available_success() {
        let config = default_config_with_models();

        let check = check_verifiers_available(&config);
        assert!(check.passed);
        assert!(check.message.contains("1 required verifier"));
    }

    #[test]
    fn test_check_verifiers_available_none_required() {
        let mut config = Config::default();
        config.required_verifiers = vec![];

        let check = check_verifiers_available(&config);
        assert!(check.passed);
        assert!(check.message.contains("No verifiers required"));
    }

    #[test]
    fn test_check_verifiers_available_missing() {
        let mut config = Config::default();
        config.required_verifiers = vec!["tests".to_string(), "lint".to_string()];
        config.verifiers = vec![VerifierConfig {
            name: "tests".to_string(),
            command_argv: vec!["cargo".to_string(), "test".to_string()],
            timeout_seconds: 300,
            run_when: crate::config::VerifierRunWhen::OnChange,
        }];

        let check = check_verifiers_available(&config);
        assert!(!check.passed);
        assert!(check.message.contains("lint"));
    }

    // Test: check_no_concurrent_run
    #[test]
    fn test_check_no_concurrent_run_success() {
        let (_temp, store) = setup_test_env();
        let thread = Thread::new("test");
        store.save(&thread).unwrap();

        let check = check_no_concurrent_run(&thread, &store);
        assert!(check.passed);
        assert!(check.message.contains("No other threads running"));
    }

    #[test]
    fn test_check_no_concurrent_run_blocked() {
        let (_temp, store) = setup_test_env();

        // Create a running thread
        let mut running = Thread::new("Running Thread");
        running.phase = ThreadPhase::Running { iteration: 1 };
        store.save(&running).unwrap();

        // Create our thread
        let thread = Thread::new("test");
        store.save(&thread).unwrap();

        let check = check_no_concurrent_run(&thread, &store);
        assert!(!check.passed);
        assert!(check.message.contains("Running Thread"));
        assert!(check.message.contains("running"));
    }

    #[test]
    fn test_check_no_concurrent_run_verifying() {
        let (_temp, store) = setup_test_env();

        // Create a verifying thread
        let mut verifying = Thread::new("Verifying Thread");
        verifying.phase = ThreadPhase::Verifying { iteration: 1 };
        store.save(&verifying).unwrap();

        let thread = Thread::new("test");
        store.save(&thread).unwrap();

        let check = check_no_concurrent_run(&thread, &store);
        assert!(!check.passed);
        assert!(check.message.contains("Verifying Thread"));
    }

    #[test]
    fn test_check_no_concurrent_run_paused() {
        let (_temp, store) = setup_test_env();

        // Create a paused thread
        let mut paused = Thread::new("Paused Thread");
        paused.phase = ThreadPhase::Paused { iteration: 1 };
        store.save(&paused).unwrap();

        let thread = Thread::new("test");
        store.save(&thread).unwrap();

        let check = check_no_concurrent_run(&thread, &store);
        assert!(!check.passed);
        assert!(check.message.contains("Paused Thread"));
    }

    // Test: run_preflight (integration)
    #[test]
    fn test_run_preflight_all_pass() {
        let (temp, store) = setup_test_env();
        let thread = create_thread_with_spec(&store, true, true);
        let config = default_config_with_models();

        let result = run_preflight(&thread, temp.path(), &store, &config);

        assert!(result.passed);
        assert_eq!(result.checks.len(), 7);
        assert!(result.checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_run_preflight_multiple_failures() {
        let temp = TempDir::new().unwrap();
        let store = ThreadStore::new(temp.path()).unwrap();

        let mut thread = Thread::new("Test");
        thread.phase = ThreadPhase::Finalized;
        store.save(&thread).unwrap();
        // No spec saved

        let config = Config::default(); // No models

        let result = run_preflight(&thread, temp.path(), &store, &config);

        assert!(!result.passed);
        // Should have multiple failures
        let failure_count = result.checks.iter().filter(|c| !c.passed).count();
        assert!(failure_count > 1);
        // All 7 checks should still run
        assert_eq!(result.checks.len(), 7);
    }
}
