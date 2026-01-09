//! Thread persistence for ralf workflows.
//!
//! Provides reliable persistence for Thread state with atomic writes,
//! schema versioning, and active thread tracking.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use tracing::warn;

use crate::thread::Thread;

/// Current schema version for thread persistence.
const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Error type for persistence operations.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("Thread not found: {0}")]
    ThreadNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid thread data: {0}")]
    InvalidData(String),

    #[error("Unsupported schema version: {0} (max supported: {1})")]
    UnsupportedSchema(u32, u32),

    #[error("Invalid thread ID: {0}")]
    InvalidId(String),
}

/// Summary info for listing threads without loading full state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub id: String,
    pub title: String,
    pub phase: String,
    pub phase_category: u8,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// On-disk format with schema versioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThreadFile {
    schema_version: u32,
    #[serde(flatten)]
    thread: Thread,
}

/// Manages thread persistence.
pub struct ThreadStore {
    base_path: PathBuf,
}

impl ThreadStore {
    /// Create a new `ThreadStore`.
    /// Creates the threads directory if it doesn't exist.
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, PersistenceError> {
        let base_path = base_path.into();
        let threads_dir = base_path.join("threads");
        fs::create_dir_all(&threads_dir)?;
        Ok(Self { base_path })
    }

    /// Save a thread with atomic write pattern.
    /// Creates thread directory if needed.
    pub fn save(&self, thread: &Thread) -> Result<(), PersistenceError> {
        Self::validate_id(&thread.id)?;

        let thread_dir = self.thread_dir(&thread.id);
        fs::create_dir_all(&thread_dir)?;

        let thread_file = ThreadFile {
            schema_version: CURRENT_SCHEMA_VERSION,
            thread: thread.clone(),
        };

        let json = serde_json::to_string_pretty(&thread_file)?;
        let path = thread_dir.join("thread.json");
        atomic_write(&path, json.as_bytes())?;

        Ok(())
    }

    /// Load a thread by ID.
    pub fn load(&self, id: &str) -> Result<Thread, PersistenceError> {
        Self::validate_id(id)?;

        let path = self.thread_dir(id).join("thread.json");
        if !path.exists() {
            return Err(PersistenceError::ThreadNotFound(id.to_string()));
        }

        let content = fs::read_to_string(&path)?;

        // First, extract schema_version to check compatibility
        let raw: serde_json::Value = serde_json::from_str(&content)?;
        let version_u64 = raw
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| PersistenceError::InvalidData("missing schema_version".to_string()))?;
        let version = u32::try_from(version_u64).map_err(|_| {
            PersistenceError::InvalidData("schema_version too large".to_string())
        })?;

        if version > CURRENT_SCHEMA_VERSION {
            return Err(PersistenceError::UnsupportedSchema(
                version,
                CURRENT_SCHEMA_VERSION,
            ));
        }

        // For v1, no migrations needed - just deserialize
        let thread_file: ThreadFile = serde_json::from_str(&content)?;
        Ok(thread_file.thread)
    }

    /// Check if a thread exists (has valid thread.json).
    /// Returns false for corrupted threads (cannot be loaded).
    pub fn exists(&self, id: &str) -> bool {
        if Self::validate_id(id).is_err() {
            return false;
        }
        // Actually try to load the thread to verify it's valid
        self.load(id).is_ok()
    }

    /// Delete a thread and all its data.
    pub fn delete(&self, id: &str) -> Result<(), PersistenceError> {
        Self::validate_id(id)?;

        let thread_dir = self.thread_dir(id);
        if !thread_dir.exists() {
            return Err(PersistenceError::ThreadNotFound(id.to_string()));
        }

        // Clear active if this was the active thread
        if let Ok(Some(active_id)) = self.get_active() {
            if active_id == id {
                self.clear_active()?;
            }
        }

        fs::remove_dir_all(&thread_dir)?;
        Ok(())
    }

    /// List all threads with summary info.
    /// Sorted by `updated_at` descending (most recent first).
    pub fn list(&self) -> Result<Vec<ThreadSummary>, PersistenceError> {
        let threads_dir = self.base_path.join("threads");
        if !threads_dir.exists() {
            return Ok(Vec::new());
        }

        let active_id = self.get_active()?.unwrap_or_default();

        let mut summaries = Vec::new();
        for entry in fs::read_dir(&threads_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let id = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            // Try to load the thread, skip if corrupted
            match self.load(&id) {
                Ok(thread) => {
                    summaries.push(ThreadSummary {
                        id: thread.id.clone(),
                        title: thread.title.clone(),
                        phase: thread.phase_display_name().to_string(),
                        phase_category: thread.phase_category(),
                        updated_at: thread.updated_at,
                        is_active: thread.id == active_id,
                    });
                }
                Err(e) => {
                    warn!(thread_id = %id, error = %e, "Skipping corrupted thread");
                }
            }
        }

        // Sort by updated_at descending
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(summaries)
    }

    /// Get the active thread ID, if any.
    pub fn get_active(&self) -> Result<Option<String>, PersistenceError> {
        let path = self.base_path.join("active_thread");
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let id = content.trim();

        if id.is_empty() {
            return Ok(None);
        }

        // Return None if active thread doesn't exist
        if !self.exists(id) {
            return Ok(None);
        }

        Ok(Some(id.to_string()))
    }

    /// Set the active thread ID.
    pub fn set_active(&self, id: &str) -> Result<(), PersistenceError> {
        Self::validate_id(id)?;

        if !self.exists(id) {
            return Err(PersistenceError::ThreadNotFound(id.to_string()));
        }

        let path = self.base_path.join("active_thread");
        atomic_write(&path, id.as_bytes())?;

        Ok(())
    }

    /// Clear the active thread.
    pub fn clear_active(&self) -> Result<(), PersistenceError> {
        let path = self.base_path.join("active_thread");
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Save a spec revision for a thread.
    /// Returns the revision number assigned.
    pub fn save_spec(&self, thread_id: &str, content: &str) -> Result<u32, PersistenceError> {
        Self::validate_id(thread_id)?;

        if !self.exists(thread_id) {
            return Err(PersistenceError::ThreadNotFound(thread_id.to_string()));
        }

        let spec_dir = self.thread_dir(thread_id).join("spec");
        fs::create_dir_all(&spec_dir)?;

        // Find next revision number
        let existing = self.list_specs(thread_id)?;
        let next_rev = existing.last().copied().unwrap_or(0) + 1;

        let path = spec_dir.join(format!("v{next_rev}.md"));
        atomic_write(&path, content.as_bytes())?;

        Ok(next_rev)
    }

    /// Load a specific spec revision.
    pub fn load_spec(&self, thread_id: &str, revision: u32) -> Result<String, PersistenceError> {
        Self::validate_id(thread_id)?;

        let path = self
            .thread_dir(thread_id)
            .join("spec")
            .join(format!("v{revision}.md"));

        if !path.exists() {
            return Err(PersistenceError::InvalidData(format!(
                "spec revision {revision} not found"
            )));
        }

        Ok(fs::read_to_string(&path)?)
    }

    /// Load the latest spec revision for a thread.
    /// Returns `Ok(None)` if no specs exist.
    pub fn load_latest_spec(&self, thread_id: &str) -> Result<Option<String>, PersistenceError> {
        let revisions = self.list_specs(thread_id)?;
        match revisions.last() {
            Some(&rev) => Ok(Some(self.load_spec(thread_id, rev)?)),
            None => Ok(None),
        }
    }

    /// List available spec revisions for a thread.
    pub fn list_specs(&self, thread_id: &str) -> Result<Vec<u32>, PersistenceError> {
        Self::validate_id(thread_id)?;

        let spec_dir = self.thread_dir(thread_id).join("spec");
        if !spec_dir.exists() {
            return Ok(Vec::new());
        }

        let mut revisions = Vec::new();
        for entry in fs::read_dir(&spec_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Parse vN.md format
            if let Some(num_str) = name_str.strip_prefix('v').and_then(|s| s.strip_suffix(".md")) {
                if let Ok(num) = num_str.parse::<u32>() {
                    revisions.push(num);
                }
            }
        }

        revisions.sort_unstable();
        Ok(revisions)
    }

    /// Validate a thread ID for filesystem safety.
    fn validate_id(id: &str) -> Result<(), PersistenceError> {
        if id.is_empty() {
            return Err(PersistenceError::InvalidId("ID cannot be empty".to_string()));
        }

        if id.contains('/') || id.contains('\\') {
            return Err(PersistenceError::InvalidId(
                "ID cannot contain path separators".to_string(),
            ));
        }

        if id.contains("..") {
            return Err(PersistenceError::InvalidId(
                "ID cannot contain path traversal".to_string(),
            ));
        }

        // Only allow printable ASCII (alphanumeric, dash, underscore)
        for ch in id.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
                return Err(PersistenceError::InvalidId(format!(
                    "ID contains invalid character: {ch}"
                )));
            }
        }

        Ok(())
    }

    /// Get the path to a thread's directory.
    fn thread_dir(&self, id: &str) -> PathBuf {
        self.base_path.join("threads").join(id)
    }
}

/// Write content atomically using temp file + fsync + rename.
fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    // Generate unique temp filename using timestamp and process ID
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let tmp_name = format!("{file_name}.{timestamp}.{pid}.tmp");
    let tmp_path = path.with_file_name(tmp_name);

    let result = (|| {
        let mut file = File::create(&tmp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        // Best-effort cleanup
        let _ = fs::remove_file(&tmp_path);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thread::{Thread, ThreadPhase};
    use tempfile::TempDir;

    fn setup_test_store() -> (TempDir, ThreadStore) {
        let temp = TempDir::new().unwrap();
        let store = ThreadStore::new(temp.path()).unwrap();
        (temp, store)
    }

    #[test]
    fn test_new_creates_threads_dir() {
        let temp = TempDir::new().unwrap();
        let _store = ThreadStore::new(temp.path()).unwrap();
        assert!(temp.path().join("threads").exists());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let (_temp, store) = setup_test_store();

        let mut thread = Thread::new("Test Thread");
        thread.phase = ThreadPhase::Finalized;

        store.save(&thread).unwrap();
        let loaded = store.load(&thread.id).unwrap();

        assert_eq!(loaded.id, thread.id);
        assert_eq!(loaded.title, thread.title);
        assert!(matches!(loaded.phase, ThreadPhase::Finalized));
    }

    #[test]
    fn test_save_creates_thread_directory() {
        let (temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();

        assert!(temp.path().join("threads").join(&thread.id).exists());
        assert!(temp
            .path()
            .join("threads")
            .join(&thread.id)
            .join("thread.json")
            .exists());
    }

    #[test]
    fn test_load_not_found() {
        let (_temp, store) = setup_test_store();

        let result = store.load("nonexistent");
        assert!(matches!(result, Err(PersistenceError::ThreadNotFound(_))));
    }

    #[test]
    fn test_load_unsupported_schema() {
        let (temp, store) = setup_test_store();

        // Create a thread with a future schema version
        let thread_dir = temp.path().join("threads").join("test-id");
        fs::create_dir_all(&thread_dir).unwrap();

        let future_json = r#"{"schema_version": 999, "id": "test-id", "title": "Test"}"#;
        fs::write(thread_dir.join("thread.json"), future_json).unwrap();

        let result = store.load("test-id");
        assert!(matches!(result, Err(PersistenceError::UnsupportedSchema(999, 1))));
    }

    #[test]
    fn test_exists() {
        let (_temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        assert!(!store.exists(&thread.id));

        store.save(&thread).unwrap();
        assert!(store.exists(&thread.id));
    }

    #[test]
    fn test_delete() {
        let (_temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();
        assert!(store.exists(&thread.id));

        store.delete(&thread.id).unwrap();
        assert!(!store.exists(&thread.id));
    }

    #[test]
    fn test_delete_clears_active() {
        let (_temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();
        store.set_active(&thread.id).unwrap();

        assert!(store.get_active().unwrap().is_some());

        store.delete(&thread.id).unwrap();
        assert!(store.get_active().unwrap().is_none());
    }

    #[test]
    fn test_delete_not_found() {
        let (_temp, store) = setup_test_store();

        let result = store.delete("nonexistent");
        assert!(matches!(result, Err(PersistenceError::ThreadNotFound(_))));
    }

    #[test]
    fn test_list_empty() {
        let (_temp, store) = setup_test_store();

        let list = store.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_list_sorted_by_updated_at() {
        let (_temp, store) = setup_test_store();

        let thread1 = Thread::new("Thread 1");
        let mut thread2 = Thread::new("Thread 2");

        // Make thread2 older
        thread2.updated_at = thread1.updated_at - chrono::Duration::hours(1);

        store.save(&thread1).unwrap();
        store.save(&thread2).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].title, "Thread 1"); // More recent first
        assert_eq!(list[1].title, "Thread 2");
    }

    #[test]
    fn test_list_marks_active() {
        let (_temp, store) = setup_test_store();

        let thread1 = Thread::new("Thread 1");
        let thread2 = Thread::new("Thread 2");

        store.save(&thread1).unwrap();
        store.save(&thread2).unwrap();
        store.set_active(&thread2.id).unwrap();

        let list = store.list().unwrap();
        let active_count = list.iter().filter(|s| s.is_active).count();
        assert_eq!(active_count, 1);

        let active = list.iter().find(|s| s.is_active).unwrap();
        assert_eq!(active.id, thread2.id);
    }

    #[test]
    fn test_list_skips_corrupted() {
        let (temp, store) = setup_test_store();

        let thread = Thread::new("Valid Thread");
        store.save(&thread).unwrap();

        // Create a corrupted thread directory
        let corrupted_dir = temp.path().join("threads").join("corrupted-id");
        fs::create_dir_all(&corrupted_dir).unwrap();
        fs::write(corrupted_dir.join("thread.json"), "not valid json").unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Valid Thread");
    }

    #[test]
    fn test_get_active_none() {
        let (_temp, store) = setup_test_store();
        assert!(store.get_active().unwrap().is_none());
    }

    #[test]
    fn test_get_active_returns_none_for_missing_thread() {
        let (temp, store) = setup_test_store();

        // Write an active_thread file pointing to nonexistent thread
        fs::write(temp.path().join("active_thread"), "nonexistent").unwrap();

        assert!(store.get_active().unwrap().is_none());
    }

    #[test]
    fn test_set_and_get_active() {
        let (_temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();
        store.set_active(&thread.id).unwrap();

        let active = store.get_active().unwrap();
        assert_eq!(active, Some(thread.id));
    }

    #[test]
    fn test_set_active_fails_for_nonexistent() {
        let (_temp, store) = setup_test_store();

        let result = store.set_active("nonexistent");
        assert!(matches!(result, Err(PersistenceError::ThreadNotFound(_))));
    }

    #[test]
    fn test_clear_active() {
        let (_temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();
        store.set_active(&thread.id).unwrap();

        assert!(store.get_active().unwrap().is_some());

        store.clear_active().unwrap();
        assert!(store.get_active().unwrap().is_none());
    }

    #[test]
    fn test_save_and_load_spec() {
        let (_temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();

        let rev1 = store.save_spec(&thread.id, "# Spec v1").unwrap();
        assert_eq!(rev1, 1);

        let rev2 = store.save_spec(&thread.id, "# Spec v2").unwrap();
        assert_eq!(rev2, 2);

        let content1 = store.load_spec(&thread.id, 1).unwrap();
        assert_eq!(content1, "# Spec v1");

        let content2 = store.load_spec(&thread.id, 2).unwrap();
        assert_eq!(content2, "# Spec v2");
    }

    #[test]
    fn test_list_specs() {
        let (_temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();

        assert!(store.list_specs(&thread.id).unwrap().is_empty());

        store.save_spec(&thread.id, "v1").unwrap();
        store.save_spec(&thread.id, "v2").unwrap();
        store.save_spec(&thread.id, "v3").unwrap();

        let revisions = store.list_specs(&thread.id).unwrap();
        assert_eq!(revisions, vec![1, 2, 3]);
    }

    #[test]
    fn test_validate_id_empty() {
        let result = ThreadStore::validate_id("");
        assert!(matches!(result, Err(PersistenceError::InvalidId(_))));
    }

    #[test]
    fn test_validate_id_path_separator() {
        let result = ThreadStore::validate_id("foo/bar");
        assert!(matches!(result, Err(PersistenceError::InvalidId(_))));

        let result = ThreadStore::validate_id("foo\\bar");
        assert!(matches!(result, Err(PersistenceError::InvalidId(_))));
    }

    #[test]
    fn test_validate_id_traversal() {
        let result = ThreadStore::validate_id("..");
        assert!(matches!(result, Err(PersistenceError::InvalidId(_))));

        let result = ThreadStore::validate_id("foo/../bar");
        assert!(matches!(result, Err(PersistenceError::InvalidId(_))));
    }

    #[test]
    fn test_validate_id_valid() {
        assert!(ThreadStore::validate_id("abc-123_DEF").is_ok());
        assert!(ThreadStore::validate_id("a1b2c3d4-e5f6-7890-abcd-ef1234567890").is_ok());
    }

    #[test]
    fn test_thread_all_fields_preserved() {
        let (_temp, store) = setup_test_store();

        let mut thread = Thread::new("Full Thread");
        thread.phase = ThreadPhase::Running { iteration: 5 };
        thread.current_spec_revision = 3;
        thread.current_run_id = Some("run-123".to_string());

        store.save(&thread).unwrap();
        let loaded = store.load(&thread.id).unwrap();

        assert_eq!(loaded.id, thread.id);
        assert_eq!(loaded.title, thread.title);
        assert_eq!(loaded.current_spec_revision, 3);
        assert_eq!(loaded.current_run_id, Some("run-123".to_string()));

        if let ThreadPhase::Running { iteration } = loaded.phase {
            assert_eq!(iteration, 5);
        } else {
            panic!("Expected Running phase");
        }
    }

    #[test]
    fn test_exists_returns_false_for_corrupted() {
        let (temp, store) = setup_test_store();

        // Create a corrupted thread directory with invalid JSON
        let corrupted_dir = temp.path().join("threads").join("corrupted-thread");
        fs::create_dir_all(&corrupted_dir).unwrap();
        fs::write(corrupted_dir.join("thread.json"), "not valid json").unwrap();

        assert!(!store.exists("corrupted-thread"));
    }

    #[test]
    fn test_exists_returns_false_for_unsupported_schema() {
        let (temp, store) = setup_test_store();

        // Create a thread with unsupported schema version
        let thread_dir = temp.path().join("threads").join("future-thread");
        fs::create_dir_all(&thread_dir).unwrap();
        fs::write(
            thread_dir.join("thread.json"),
            r#"{"schema_version": 999, "id": "future-thread", "title": "Future"}"#,
        )
        .unwrap();

        assert!(!store.exists("future-thread"));
    }

    #[test]
    fn test_get_active_returns_none_for_empty_file() {
        let (temp, store) = setup_test_store();

        // Write an empty active_thread file
        fs::write(temp.path().join("active_thread"), "").unwrap();

        assert!(store.get_active().unwrap().is_none());
    }

    #[test]
    fn test_get_active_returns_none_for_whitespace_only() {
        let (temp, store) = setup_test_store();

        // Write a whitespace-only active_thread file
        fs::write(temp.path().join("active_thread"), "   \n  ").unwrap();

        assert!(store.get_active().unwrap().is_none());
    }

    #[test]
    fn test_load_error_missing_schema_version() {
        let (temp, store) = setup_test_store();

        // Create a thread without schema_version
        let thread_dir = temp.path().join("threads").join("no-schema");
        fs::create_dir_all(&thread_dir).unwrap();
        fs::write(
            thread_dir.join("thread.json"),
            r#"{"id": "no-schema", "title": "No Schema"}"#,
        )
        .unwrap();

        let result = store.load("no-schema");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PersistenceError::InvalidData(_)));
    }

    #[test]
    fn test_atomic_write_no_temp_files_on_success() {
        let (temp, store) = setup_test_store();

        let thread = Thread::new("Test Thread");
        store.save(&thread).unwrap();

        // Check no .tmp files remain
        let thread_dir = temp.path().join("threads").join(&thread.id);
        for entry in fs::read_dir(&thread_dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            assert!(!name.ends_with(".tmp"), "Found temp file: {}", name);
        }
    }

    #[test]
    fn test_serialized_json_format() {
        let (temp, store) = setup_test_store();

        let mut thread = Thread::new("Format Test");
        thread.phase = ThreadPhase::Running { iteration: 3 };

        store.save(&thread).unwrap();

        // Read the raw JSON and verify format
        let json_path = temp.path().join("threads").join(&thread.id).join("thread.json");
        let content = fs::read_to_string(&json_path).unwrap();
        let raw: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Check schema_version exists
        assert_eq!(raw["schema_version"], 1);

        // Check phase format uses tagged enum: {"type": "Running", "data": {"iteration": 3}}
        let phase = &raw["phase"];
        assert_eq!(phase["type"], "Running");
        assert_eq!(phase["data"]["iteration"], 3);
    }
}
