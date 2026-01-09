# F5: Preflight Checks

## Promise

Validate all prerequisites before a thread can transition from Finalized to Preflight. Preflight ensures git safety, spec validity, model availability, verifier availability, and single-run enforcement. Returns structured results for TUI display.

## Context

Preflight runs at the **Finalized → Preflight** transition (per state machine):
1. User finalizes spec (human checkpoint)
2. User clicks "Start Implementation"
3. Thread transitions to **Preflight** phase
4. **Preflight checks run**
5. If all pass → Configuring → Running
6. If any fail → PreflightFailed with clear error

**Note:** This module uses `crate::thread::Thread` (the workflow thread), not `crate::chat::Thread` (chat conversation).

## Deliverables

**File:** `crates/ralf-engine/src/preflight.rs` (new)

### New Types

```rust
/// Result of running preflight checks.
#[derive(Debug, Clone)]
pub struct PreflightResult {
    /// Whether all checks passed.
    pub passed: bool,
    /// Individual check results.
    pub checks: Vec<PreflightCheck>,
}

/// A single preflight check result.
#[derive(Debug, Clone)]
pub struct PreflightCheck {
    /// Check identifier (e.g., "git_clean", "spec_valid").
    pub name: String,
    /// Human-readable label (e.g., "Git Working Tree").
    pub label: String,
    /// Whether this check passed.
    pub passed: bool,
    /// Descriptive message (success or failure reason).
    pub message: String,
}

impl PreflightResult {
    /// Get the first failing check, if any.
    pub fn first_failure(&self) -> Option<&PreflightCheck>;

    /// Get a summary message suitable for display.
    pub fn summary(&self) -> String;
}
```

### Main Function

```rust
/// Run all preflight checks for a thread.
///
/// Checks are run in order, but all checks run regardless of earlier failures
/// (so user sees all issues at once).
///
/// # Arguments
/// * `thread` - The thread to validate (crate::thread::Thread, must be in Finalized phase)
/// * `repo_path` - Path to the repository root
/// * `store` - ThreadStore for checking other threads
/// * `config` - Application configuration
///
/// # Returns
/// PreflightResult with all check outcomes
pub fn run_preflight(
    thread: &crate::thread::Thread,
    repo_path: &Path,
    store: &ThreadStore,
    config: &Config,
) -> PreflightResult;
```

### Individual Check Functions

```rust
/// Check 1: Git working tree is clean or on a ralf-managed branch.
///
/// Passes if:
/// - Not a git repository (git safety disabled), OR
/// - Working tree is clean (no uncommitted changes), OR
/// - Currently on a ralf/<thread-id> branch for this thread
fn check_git_state(thread: &crate::thread::Thread, repo_path: &Path) -> PreflightCheck;

/// Check 2: Git baseline can be captured.
///
/// Passes if:
/// - Not a git repository (git safety disabled), OR
/// - We can get current branch name (not detached HEAD)
/// - We can get current commit SHA
fn check_baseline_capturable(repo_path: &Path) -> PreflightCheck;

/// Check 3: Spec has a promise tag.
///
/// Passes if:
/// - Thread has at least one spec revision saved
/// - Latest spec contains a promise tag (<promise>...</promise>)
fn check_spec_has_promise(thread: &crate::thread::Thread, store: &ThreadStore) -> PreflightCheck;

/// Check 4: Completion criteria are parseable.
///
/// Passes if:
/// - Spec has a criteria section (## Requirements, ## Criteria, etc.)
/// - At least one criterion can be extracted
fn check_criteria_parseable(thread: &crate::thread::Thread, store: &ThreadStore) -> PreflightCheck;

/// Check 5: At least one model is configured.
///
/// Passes if:
/// - Config has at least one model in models vec, OR
/// - Thread has run_config with at least one model
fn check_models_available(thread: &crate::thread::Thread, config: &Config) -> PreflightCheck;

/// Check 6: Required verifiers are configured.
///
/// Passes if:
/// - All verifiers listed in config.required_verifiers exist in config.verifiers
fn check_verifiers_available(config: &Config) -> PreflightCheck;

/// Check 7: No other thread is currently Running.
///
/// Passes if:
/// - No other thread in the store is in Running, Verifying, or Paused phase
/// - (The current thread being in Finalized/Preflight is expected)
fn check_no_concurrent_run(thread: &crate::thread::Thread, store: &ThreadStore) -> PreflightCheck;
```

## Implementation Notes

### Check Order

Checks run in this order (all run, even if earlier ones fail):
1. `git_state` - Most likely to fail, user needs to commit/stash
2. `baseline_capturable` - Related to git state
3. `spec_has_promise` - Spec content issue
4. `criteria_parseable` - Spec content issue
5. `models_available` - Configuration issue
6. `verifiers_available` - Configuration issue
7. `no_concurrent_run` - Runtime constraint

### Git State Logic

```rust
fn check_git_state(thread: &crate::thread::Thread, repo_path: &Path) -> PreflightCheck {
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
            passed: true,
            message: "Working tree is clean".to_string(),
            ..
        };
    }

    // On this thread's branch? Pass (resuming previous work).
    if let Ok(branch) = git.current_branch() {
        let thread_branch = format!("ralf/{}", thread.id);
        if branch == thread_branch {
            return PreflightCheck {
                passed: true,
                message: format!("On thread branch {}", thread_branch),
                ..
            };
        }
    }

    // Dirty on non-thread branch? Fail.
    PreflightCheck {
        passed: false,
        message: "Working tree has uncommitted changes. Commit or stash before running.".to_string(),
        ..
    }
}
```

### Baseline Capturable Logic

```rust
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
    match (git.current_branch(), git.current_commit()) {
        (Ok(branch), Ok(commit)) => PreflightCheck {
            passed: true,
            message: format!("Branch: {}, Commit: {}", branch, &commit[..8]),
            ..
        },
        (Err(_), _) => PreflightCheck {
            passed: false,
            message: "Cannot determine current branch (detached HEAD?)".to_string(),
            ..
        },
        (_, Err(_)) => PreflightCheck {
            passed: false,
            message: "Cannot get current commit SHA".to_string(),
            ..
        },
    }
}
```

### Promise Tag Detection

The promise tag format (from existing `draft_has_promise` in `chat.rs`):
```
<promise>
Your promise content here
</promise>
```

Use the existing `draft_has_promise` function from `chat.rs`:
```rust
pub fn draft_has_promise(draft: &str) -> bool {
    draft.contains("<promise>") && draft.contains("</promise>")
}
```

### Criteria Parsing

Use the existing `parse_criteria` function from `lib.rs` which looks for:
- `## Requirements`
- `## Criteria`
- `## Acceptance Criteria`
- `## Completion Criteria`
- `## Verification`

And extracts bullet points from those sections.

### Verifiers Available Check

```rust
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
    let missing: Vec<_> = config.required_verifiers
        .iter()
        .filter(|name| !configured_names.contains(name))
        .collect();

    if missing.is_empty() {
        PreflightCheck {
            passed: true,
            message: format!("{} required verifier(s) configured", config.required_verifiers.len()),
            ..
        }
    } else {
        PreflightCheck {
            passed: false,
            message: format!("Missing verifiers: {}", missing.join(", ")),
            ..
        }
    }
}
```

### Concurrent Run Check

```rust
fn check_no_concurrent_run(thread: &crate::thread::Thread, store: &ThreadStore) -> PreflightCheck {
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
                    summary.title, summary.phase.to_lowercase()
                ),
            };
        }
    }

    PreflightCheck {
        passed: true,
        message: "No other threads running".to_string(),
        ..
    }
}
```

## Acceptance Criteria

- [ ] `PreflightResult` and `PreflightCheck` types defined
- [ ] `run_preflight()` runs all 7 checks
- [ ] All checks run even if earlier ones fail (user sees all issues)
- [ ] `check_git_state()` passes for clean tree or thread branch
- [ ] `check_git_state()` passes with warning for non-git repos
- [ ] `check_baseline_capturable()` passes for non-git repos
- [ ] `check_baseline_capturable()` detects detached HEAD
- [ ] `check_spec_has_promise()` validates `<promise>` tag exists
- [ ] `check_criteria_parseable()` validates at least one criterion
- [ ] `check_models_available()` checks config.models or thread run_config
- [ ] `check_verifiers_available()` validates required verifiers exist
- [ ] `check_no_concurrent_run()` blocks if another thread is Running/Verifying/Paused
- [ ] `first_failure()` returns first failing check
- [ ] `summary()` returns human-readable result
- [ ] `cargo build -p ralf-engine` succeeds
- [ ] `cargo clippy -p ralf-engine` has no warnings
- [ ] `cargo test -p ralf-engine` passes
- [ ] At least 14 unit tests covering check logic

## Non-Goals (for F5)

- Running preflight automatically on phase transition (TUI responsibility)
- UI display of preflight results (F6 responsibility)
- Fixing failed checks automatically
- Async/parallel check execution (checks are fast)

## Testing Strategy

Unit tests will use temp directories and mock data:

```rust
fn setup_test_env() -> (TempDir, ThreadStore, GitSafety) {
    let temp = TempDir::new().unwrap();
    // Initialize git repo
    // Create ThreadStore
    (temp, store, git)
}
```

Tests should cover:
1. All checks pass → `passed: true`
2. Each individual check failure (7 checks = 7+ tests)
3. Multiple failures (all reported)
4. Non-git repo handling (git_state and baseline_capturable)
5. Thread branch detection
6. Concurrent run detection (Running, Verifying, Paused)
7. Missing spec / empty spec
8. Spec without promise tag
9. Spec without criteria section
10. Missing required verifiers

## Dependencies

- `crate::thread::{Thread, ThreadPhase}` (workflow thread, NOT chat::Thread)
- `crate::persistence::ThreadStore`
- `crate::git::GitSafety`
- `crate::config::Config`
- `crate::parse_criteria` (from lib.rs)
- `crate::chat::draft_has_promise`
