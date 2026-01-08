# F3: Git Safety Layer

## Promise

Provide safe git operations for ralf workflows: detecting working tree state, capturing baselines before implementation, creating thread branches, resetting to baseline on backward transitions, and generating diffs for review.

## Deliverables

**File:** `crates/ralf-engine/src/git.rs` (new)

### New Types

```rust
/// Error type for git operations.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepo(PathBuf),

    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Working tree is dirty")]
    DirtyWorkingTree,

    #[error("Repository is in detached HEAD state")]
    DetachedHead,

    #[error("Invalid branch/thread name: {0}")]
    InvalidName(String),

    #[error("Branch already exists: {0}")]
    BranchExists(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Git safety operations for a repository.
pub struct GitSafety {
    repo_path: PathBuf,
}
```

### GitSafety Methods

```rust
impl GitSafety {
    /// Create a new GitSafety for the given repository path.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self;

    /// Check if the path is inside a git repository.
    /// Returns false for non-repos (does not error).
    pub fn is_repo(&self) -> bool;

    /// Check if the working tree is clean.
    /// Returns false if there are:
    /// - Staged changes
    /// - Unstaged changes to tracked files
    /// - Untracked files (for safety)
    pub fn is_clean(&self) -> Result<bool, GitError>;

    /// Get the current branch name.
    /// Returns `DetachedHead` error if not on a branch.
    pub fn current_branch(&self) -> Result<String, GitError>;

    /// Get the current HEAD commit SHA (full 40 chars).
    pub fn head_sha(&self) -> Result<String, GitError>;

    /// Capture baseline (current branch + commit SHA).
    /// Returns GitBaseline from thread.rs.
    pub fn capture_baseline(&self) -> Result<GitBaseline, GitError>;

    /// Create a thread branch: ralf/<thread-id>
    /// Validates thread_id contains only safe characters (alphanumeric, dash, underscore).
    /// Fails if branch already exists or thread_id is invalid.
    pub fn create_thread_branch(&self, thread_id: &str) -> Result<(), GitError>;

    /// Check if a thread branch exists.
    /// Returns false for non-repos (does not error).
    pub fn thread_branch_exists(&self, thread_id: &str) -> bool;

    /// Delete a thread branch.
    /// Cannot delete if it's the currently checked out branch.
    pub fn delete_thread_branch(&self, thread_id: &str) -> Result<(), GitError>;

    /// Checkout a branch.
    pub fn checkout(&self, branch: &str) -> Result<(), GitError>;

    /// Reset to a specific commit (hard reset).
    /// WARNING: Destructive - discards all uncommitted changes to tracked files.
    /// NOTE: Does NOT remove untracked files. Use with user confirmation.
    pub fn reset_hard(&self, commit_sha: &str) -> Result<(), GitError>;

    /// Reset to baseline: checkout branch and hard reset to baseline SHA.
    /// WARNING: Destructive - should only be used on ralf/<thread-id> branches
    /// to avoid rewinding the user's base branch if it has advanced.
    /// NOTE: Does NOT remove untracked files created during implementation.
    pub fn reset_to_baseline(&self, baseline: &GitBaseline) -> Result<(), GitError>;

    /// Get diff from baseline to current working tree (includes uncommitted).
    pub fn diff_from_baseline(&self, baseline: &GitBaseline) -> Result<String, GitError>;

    /// Get short diff stats (files changed, insertions, deletions).
    pub fn diff_stat(&self, baseline: &GitBaseline) -> Result<String, GitError>;

    /// Validate that a thread_id is safe for use in branch names.
    /// Only allows: alphanumeric, dash, underscore.
    pub fn validate_thread_id(thread_id: &str) -> Result<(), GitError>;

    /// Validate that a commit SHA is a valid 40-character hex string.
    /// Prevents option injection in commands that take a commit reference.
    pub fn validate_commit_sha(sha: &str) -> Result<(), GitError>;
}
```

## Implementation Notes

- Use `std::process::Command` to run git commands
- Always use `.arg()` method, never shell string interpolation (prevents injection)
- Use `--` separator before branch/ref names to prevent option injection
- Parse git output to extract information
- Thread branch naming: `ralf/<thread-id>` (e.g., `ralf/abc123`)
- All destructive operations (`reset_hard`, `reset_to_baseline`) are clearly documented
- Caller is responsible for user confirmation before destructive operations
- Graceful degradation: `is_repo()` and `thread_branch_exists()` return false for non-git directories
- Result-returning methods return `NotARepo` error for non-git directories

### Git Commands Used

| Method | Git Command |
|--------|-------------|
| `is_repo` | `git rev-parse --is-inside-work-tree` |
| `is_clean` | `git status --porcelain` (empty = clean) |
| `current_branch` | `git branch --show-current` (empty = detached HEAD) |
| `head_sha` | `git rev-parse HEAD` |
| `create_thread_branch` | `git branch -- ralf/<id>` |
| `thread_branch_exists` | `git show-ref --verify --quiet refs/heads/ralf/<id>` |
| `delete_thread_branch` | `git branch -D -- ralf/<id>` |
| `checkout` | `git switch -- <branch>` |
| `reset_hard` | `git reset --hard <sha>` |
| `diff_from_baseline` | `git diff <sha>` (includes uncommitted work) |
| `diff_stat` | `git diff --stat <sha>` |

## Acceptance Criteria

- [ ] `GitSafety::new()` creates instance for any path
- [ ] `is_repo()` returns true for git repos, false otherwise
- [ ] `is_clean()` returns true when no uncommitted changes
- [ ] `is_clean()` returns false when there are staged or unstaged changes
- [ ] `is_clean()` returns false when there are untracked files
- [ ] `current_branch()` returns the current branch name
- [ ] `current_branch()` returns `DetachedHead` error when not on a branch
- [ ] `head_sha()` returns the full commit SHA
- [ ] `capture_baseline()` returns `GitBaseline` with branch, sha, timestamp
- [ ] `create_thread_branch()` creates `ralf/<thread-id>` branch
- [ ] `create_thread_branch()` fails with `BranchExists` if branch exists
- [ ] `create_thread_branch()` fails with `InvalidName` for invalid thread IDs
- [ ] `validate_thread_id()` accepts alphanumeric, dash, underscore characters
- [ ] `validate_thread_id()` rejects invalid characters (spaces, slashes, etc.)
- [ ] `validate_commit_sha()` accepts valid 40-character hex SHAs
- [ ] `validate_commit_sha()` rejects invalid SHAs (wrong length, non-hex, option-like)
- [ ] `reset_hard()` validates commit SHA before executing
- [ ] `diff_from_baseline()` and `diff_stat()` validate commit SHA
- [ ] `thread_branch_exists()` correctly detects branch presence
- [ ] `delete_thread_branch()` removes the branch
- [ ] `checkout()` switches to the specified branch
- [ ] `reset_hard()` resets to specified commit
- [ ] `reset_to_baseline()` combines checkout and reset
- [ ] `diff_from_baseline()` returns diff output
- [ ] `diff_stat()` returns summary stats
- [ ] All methods return `NotARepo` error for non-git directories
- [ ] `cargo build -p ralf-engine` succeeds
- [ ] `cargo clippy -p ralf-engine` has no warnings
- [ ] `cargo test -p ralf-engine` passes
- [ ] At least 15 unit tests covering happy paths and error cases

## Non-Goals (for F3)

- Interactive user confirmation (TUI responsibility)
- Automatic branch switching during transitions (orchestration layer)
- Merge/rebase operations
- Remote operations (push, fetch, pull)

## Testing Strategy

Unit tests will use a temporary git repository created in a temp directory:

```rust
fn setup_test_repo() -> (TempDir, GitSafety) {
    let temp = tempfile::tempdir().unwrap();
    // git init, create initial commit
    let git = GitSafety::new(temp.path());
    (temp, git)
}
```

Tests should cover:
1. Clean repo detection
2. Dirty repo detection (staged, unstaged, and untracked)
3. Branch creation and deletion
4. Baseline capture and reset
5. Diff generation
6. Non-repo error handling
7. Detached HEAD error handling
8. Thread ID validation (valid and invalid characters)
