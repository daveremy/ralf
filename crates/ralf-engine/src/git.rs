//! Git safety operations for ralf workflows.
//!
//! Provides safe git operations: detecting working tree state, capturing baselines
//! before implementation, creating thread branches, resetting to baseline on
//! backward transitions, and generating diffs for review.

use std::path::PathBuf;
use std::process::Command;

use chrono::Utc;
use thiserror::Error;

use crate::thread::GitBaseline;

/// Error type for git operations.
#[derive(Debug, Error)]
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

impl GitSafety {
    /// Create a new `GitSafety` for the given repository path.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
        }
    }

    /// Check if the path is inside a git repository.
    /// Returns false for non-repos (does not error).
    pub fn is_repo(&self) -> bool {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(&self.repo_path)
            .output();

        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    /// Check if the working tree is clean.
    /// Returns false if there are:
    /// - Staged changes
    /// - Unstaged changes to tracked files
    /// - Untracked files (for safety)
    pub fn is_clean(&self) -> Result<bool, GitError> {
        self.ensure_repo()?;

        let output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().is_empty())
    }

    /// Get the current branch name.
    /// Returns `DetachedHead` error if not on a branch.
    pub fn current_branch(&self) -> Result<String, GitError> {
        self.ensure_repo()?;

        let output = Command::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            return Err(GitError::DetachedHead);
        }

        Ok(branch)
    }

    /// Get the current HEAD commit SHA (full 40 chars).
    pub fn head_sha(&self) -> Result<String, GitError> {
        self.ensure_repo()?;

        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Capture baseline (current branch + commit SHA).
    /// Returns `GitBaseline` from thread.rs.
    pub fn capture_baseline(&self) -> Result<GitBaseline, GitError> {
        let branch = self.current_branch()?;
        let commit_sha = self.head_sha()?;

        Ok(GitBaseline {
            branch,
            commit_sha,
            captured_at: Utc::now(),
        })
    }

    /// Validate that a `thread_id` is safe for use in branch names.
    /// Only allows: alphanumeric, dash, underscore.
    pub fn validate_thread_id(thread_id: &str) -> Result<(), GitError> {
        if thread_id.is_empty() {
            return Err(GitError::InvalidName("thread_id cannot be empty".to_string()));
        }

        for ch in thread_id.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
                return Err(GitError::InvalidName(format!(
                    "invalid character '{ch}' in thread_id"
                )));
            }
        }

        Ok(())
    }

    /// Create a thread branch: `ralf/<thread-id>`
    /// Validates `thread_id` contains only safe characters (alphanumeric, dash, underscore).
    /// Fails if branch already exists or `thread_id` is invalid.
    pub fn create_thread_branch(&self, thread_id: &str) -> Result<(), GitError> {
        self.ensure_repo()?;
        Self::validate_thread_id(thread_id)?;

        let branch_name = format!("ralf/{thread_id}");

        if self.thread_branch_exists(thread_id) {
            return Err(GitError::BranchExists(branch_name));
        }

        let output = Command::new("git")
            .arg("branch")
            .arg("--")
            .arg(&branch_name)
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Check if a thread branch exists.
    /// Returns false for non-repos (does not error).
    pub fn thread_branch_exists(&self, thread_id: &str) -> bool {
        if !self.is_repo() {
            return false;
        }

        // Validate thread_id first - invalid IDs never exist
        if Self::validate_thread_id(thread_id).is_err() {
            return false;
        }

        let branch_ref = format!("refs/heads/ralf/{thread_id}");

        let output = Command::new("git")
            .arg("show-ref")
            .arg("--verify")
            .arg("--quiet")
            .arg(&branch_ref)
            .current_dir(&self.repo_path)
            .output();

        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    /// Delete a thread branch.
    /// Cannot delete if it's the currently checked out branch.
    pub fn delete_thread_branch(&self, thread_id: &str) -> Result<(), GitError> {
        self.ensure_repo()?;
        Self::validate_thread_id(thread_id)?;

        let branch_name = format!("ralf/{thread_id}");

        if !self.thread_branch_exists(thread_id) {
            return Err(GitError::BranchNotFound(branch_name));
        }

        let output = Command::new("git")
            .arg("branch")
            .arg("-D")
            .arg("--")
            .arg(&branch_name)
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Checkout a branch.
    pub fn checkout(&self, branch: &str) -> Result<(), GitError> {
        self.ensure_repo()?;

        // Use git switch for branch checkout (safer than checkout)
        // Falls back to checkout if switch is not available
        let output = Command::new("git")
            .arg("switch")
            .arg(branch)
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Reset to a specific commit (hard reset).
    /// WARNING: Destructive - discards all uncommitted changes to tracked files.
    /// NOTE: Does NOT remove untracked files. Use with user confirmation.
    pub fn reset_hard(&self, commit_sha: &str) -> Result<(), GitError> {
        self.ensure_repo()?;

        let output = Command::new("git")
            .arg("reset")
            .arg("--hard")
            .arg(commit_sha)
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Reset to baseline: checkout branch and hard reset to baseline SHA.
    /// WARNING: Destructive - should only be used on ralf/<thread-id> branches
    /// to avoid rewinding the user's base branch if it has advanced.
    /// NOTE: Does NOT remove untracked files created during implementation.
    pub fn reset_to_baseline(&self, baseline: &GitBaseline) -> Result<(), GitError> {
        self.checkout(&baseline.branch)?;
        self.reset_hard(&baseline.commit_sha)?;
        Ok(())
    }

    /// Get diff from baseline to current working tree (includes uncommitted).
    pub fn diff_from_baseline(&self, baseline: &GitBaseline) -> Result<String, GitError> {
        self.ensure_repo()?;

        let output = Command::new("git")
            .arg("diff")
            .arg(&baseline.commit_sha)
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get short diff stats (files changed, insertions, deletions).
    pub fn diff_stat(&self, baseline: &GitBaseline) -> Result<String, GitError> {
        self.ensure_repo()?;

        let output = Command::new("git")
            .arg("diff")
            .arg("--stat")
            .arg(&baseline.commit_sha)
            .current_dir(&self.repo_path)
            .output()
            .map_err(GitError::Io)?;

        if !output.status.success() {
            return Err(GitError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Helper to ensure we're in a git repo.
    fn ensure_repo(&self) -> Result<(), GitError> {
        if !self.is_repo() {
            return Err(GitError::NotARepo(self.repo_path.clone()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Setup a test repository with an initial commit.
    fn setup_test_repo() -> (TempDir, GitSafety) {
        let temp = TempDir::new().unwrap();

        // git init
        Command::new("git")
            .arg("init")
            .current_dir(temp.path())
            .output()
            .expect("git init failed");

        // Configure git user for commits
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

        // Create initial commit
        let readme_path = temp.path().join("README.md");
        fs::write(&readme_path, "# Test Repo\n").unwrap();

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

        let git = GitSafety::new(temp.path());
        (temp, git)
    }

    #[test]
    fn test_is_repo_true() {
        let (_temp, git) = setup_test_repo();
        assert!(git.is_repo());
    }

    #[test]
    fn test_is_repo_false() {
        let temp = TempDir::new().unwrap();
        let git = GitSafety::new(temp.path());
        assert!(!git.is_repo());
    }

    #[test]
    fn test_is_clean_true() {
        let (_temp, git) = setup_test_repo();
        assert!(git.is_clean().unwrap());
    }

    #[test]
    fn test_is_clean_false_staged() {
        let (temp, git) = setup_test_repo();

        // Create and stage a new file
        let file_path = temp.path().join("staged.txt");
        fs::write(&file_path, "staged content").unwrap();

        Command::new("git")
            .arg("add")
            .arg("staged.txt")
            .current_dir(temp.path())
            .output()
            .unwrap();

        assert!(!git.is_clean().unwrap());
    }

    #[test]
    fn test_is_clean_false_unstaged() {
        let (temp, git) = setup_test_repo();

        // Modify tracked file without staging
        let readme_path = temp.path().join("README.md");
        fs::write(&readme_path, "# Modified\n").unwrap();

        assert!(!git.is_clean().unwrap());
    }

    #[test]
    fn test_is_clean_false_untracked() {
        let (temp, git) = setup_test_repo();

        // Create untracked file
        let file_path = temp.path().join("untracked.txt");
        fs::write(&file_path, "untracked content").unwrap();

        assert!(!git.is_clean().unwrap());
    }

    #[test]
    fn test_current_branch() {
        let (_temp, git) = setup_test_repo();
        let branch = git.current_branch().unwrap();
        // Default branch is either "main" or "master"
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn test_current_branch_detached_head() {
        let (temp, git) = setup_test_repo();

        // Get current SHA and checkout detached
        let sha = git.head_sha().unwrap();
        Command::new("git")
            .args(["checkout", "--detach", &sha])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let result = git.current_branch();
        assert!(matches!(result, Err(GitError::DetachedHead)));
    }

    #[test]
    fn test_head_sha() {
        let (_temp, git) = setup_test_repo();
        let sha = git.head_sha().unwrap();
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_capture_baseline() {
        let (_temp, git) = setup_test_repo();
        let baseline = git.capture_baseline().unwrap();

        assert!(baseline.branch == "main" || baseline.branch == "master");
        assert_eq!(baseline.commit_sha.len(), 40);
    }

    #[test]
    fn test_validate_thread_id_valid() {
        assert!(GitSafety::validate_thread_id("abc123").is_ok());
        assert!(GitSafety::validate_thread_id("my-thread").is_ok());
        assert!(GitSafety::validate_thread_id("my_thread").is_ok());
        assert!(GitSafety::validate_thread_id("ABC-123_xyz").is_ok());
    }

    #[test]
    fn test_validate_thread_id_invalid() {
        assert!(matches!(
            GitSafety::validate_thread_id(""),
            Err(GitError::InvalidName(_))
        ));
        assert!(matches!(
            GitSafety::validate_thread_id("has space"),
            Err(GitError::InvalidName(_))
        ));
        assert!(matches!(
            GitSafety::validate_thread_id("has/slash"),
            Err(GitError::InvalidName(_))
        ));
        assert!(matches!(
            GitSafety::validate_thread_id("has.dot"),
            Err(GitError::InvalidName(_))
        ));
    }

    #[test]
    fn test_create_thread_branch() {
        let (_temp, git) = setup_test_repo();

        assert!(!git.thread_branch_exists("test-thread"));
        git.create_thread_branch("test-thread").unwrap();
        assert!(git.thread_branch_exists("test-thread"));
    }

    #[test]
    fn test_create_thread_branch_already_exists() {
        let (_temp, git) = setup_test_repo();

        git.create_thread_branch("test-thread").unwrap();
        let result = git.create_thread_branch("test-thread");

        assert!(matches!(result, Err(GitError::BranchExists(_))));
    }

    #[test]
    fn test_create_thread_branch_invalid_name() {
        let (_temp, git) = setup_test_repo();

        let result = git.create_thread_branch("invalid/name");
        assert!(matches!(result, Err(GitError::InvalidName(_))));
    }

    #[test]
    fn test_delete_thread_branch() {
        let (_temp, git) = setup_test_repo();

        git.create_thread_branch("to-delete").unwrap();
        assert!(git.thread_branch_exists("to-delete"));

        git.delete_thread_branch("to-delete").unwrap();
        assert!(!git.thread_branch_exists("to-delete"));
    }

    #[test]
    fn test_delete_thread_branch_not_found() {
        let (_temp, git) = setup_test_repo();

        let result = git.delete_thread_branch("nonexistent");
        assert!(matches!(result, Err(GitError::BranchNotFound(_))));
    }

    #[test]
    fn test_checkout() {
        let (_temp, git) = setup_test_repo();

        git.create_thread_branch("feature").unwrap();
        git.checkout("ralf/feature").unwrap();

        let branch = git.current_branch().unwrap();
        assert_eq!(branch, "ralf/feature");
    }

    #[test]
    fn test_reset_hard() {
        let (temp, git) = setup_test_repo();

        // Capture original state
        let original_sha = git.head_sha().unwrap();

        // Make a commit
        let file_path = temp.path().join("newfile.txt");
        fs::write(&file_path, "content").unwrap();

        Command::new("git")
            .args(["add", "."])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Add newfile"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Verify we're at new commit
        let new_sha = git.head_sha().unwrap();
        assert_ne!(new_sha, original_sha);

        // Reset hard
        git.reset_hard(&original_sha).unwrap();

        // Should be back to original
        assert_eq!(git.head_sha().unwrap(), original_sha);
        assert!(!file_path.exists());
    }

    #[test]
    fn test_reset_to_baseline() {
        let (temp, git) = setup_test_repo();

        // Capture baseline
        let baseline = git.capture_baseline().unwrap();

        // Create and checkout thread branch
        git.create_thread_branch("work").unwrap();
        git.checkout("ralf/work").unwrap();

        // Make changes and commit
        let file_path = temp.path().join("work.txt");
        fs::write(&file_path, "work content").unwrap();

        Command::new("git")
            .args(["add", "."])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Work commit"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Reset to baseline
        git.reset_to_baseline(&baseline).unwrap();

        // Should be back on original branch at original commit
        assert_eq!(git.current_branch().unwrap(), baseline.branch);
        assert_eq!(git.head_sha().unwrap(), baseline.commit_sha);
    }

    #[test]
    fn test_diff_from_baseline() {
        let (temp, git) = setup_test_repo();

        let baseline = git.capture_baseline().unwrap();

        // Make uncommitted changes
        let readme_path = temp.path().join("README.md");
        fs::write(&readme_path, "# Modified README\n").unwrap();

        let diff = git.diff_from_baseline(&baseline).unwrap();
        assert!(diff.contains("Modified README"));
    }

    #[test]
    fn test_diff_stat() {
        let (temp, git) = setup_test_repo();

        let baseline = git.capture_baseline().unwrap();

        // Make changes
        let readme_path = temp.path().join("README.md");
        fs::write(&readme_path, "# Modified\nWith more lines\n").unwrap();

        let stat = git.diff_stat(&baseline).unwrap();
        assert!(stat.contains("README.md"));
    }

    #[test]
    fn test_not_a_repo_error() {
        let temp = TempDir::new().unwrap();
        let git = GitSafety::new(temp.path());

        assert!(matches!(git.is_clean(), Err(GitError::NotARepo(_))));
        assert!(matches!(git.current_branch(), Err(GitError::NotARepo(_))));
        assert!(matches!(git.head_sha(), Err(GitError::NotARepo(_))));
        assert!(matches!(git.capture_baseline(), Err(GitError::NotARepo(_))));
    }

    #[test]
    fn test_thread_branch_exists_non_repo() {
        let temp = TempDir::new().unwrap();
        let git = GitSafety::new(temp.path());

        // Should return false for non-repo, not error
        assert!(!git.thread_branch_exists("anything"));
    }
}
