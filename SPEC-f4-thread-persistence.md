# F4: Thread Persistence

## Promise

Provide reliable persistence for Thread state with atomic writes, schema versioning, and active thread tracking. Threads persist across ralf sessions and can be listed, loaded, and resumed.

## Scope Clarification

**F4 covers Thread persistence only.** The following are handled elsewhere:
- **Transcripts**: Handled by `chat.rs` (existing `.ralf/spec/threads/<id>.jsonl`)
- **Run logs/artifacts**: Handled by `runner.rs` (existing `.ralf/runs/<run-id>/`)
- **GitBaseline**: Embedded in Thread struct (per F1), not a separate file

The `Thread` struct contains pointers (`current_run_id`, `baseline`) to these external artifacts. F4 persists the Thread with its pointers intact.

## Deliverables

**File:** `crates/ralf-engine/src/persistence.rs` (new)

### New Types

```rust
/// Error type for persistence operations.
#[derive(Debug, thiserror::Error)]
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
    pub phase: String,           // Display name of current phase
    pub phase_category: u8,      // 1-5 for grouping (Abandoned = 5)
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// On-disk format with schema versioning.
/// Schema version at top level for easy version detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThreadFile {
    schema_version: u32,
    #[serde(flatten)]
    thread: Thread,
}

const CURRENT_SCHEMA_VERSION: u32 = 1;
```

### Storage Layout

```
.ralf/
├── threads/
│   ├── <thread-id>/
│   │   ├── thread.json          # Thread state (with schema_version)
│   │   └── spec/
│   │       ├── v1.md            # Spec revision 1
│   │       ├── v2.md            # Spec revision 2
│   │       └── ...
│   └── <thread-id>/
│       └── ...
├── active_thread                # Contains ID of active thread (plain text, trimmed)
├── runs/                        # (existing, managed by runner.rs)
└── spec/                        # (existing, managed by chat.rs)
```

**Note:** `active_thread` means "currently selected in UI", NOT "currently running". The preflight check for "no other thread running" is done by scanning threads for `Running` phase, not by this file.

### ThreadStore Struct

```rust
/// Manages thread persistence.
pub struct ThreadStore {
    base_path: PathBuf,  // Path to .ralf directory
}

impl ThreadStore {
    /// Create a new ThreadStore.
    /// Creates the threads directory if it doesn't exist.
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, PersistenceError>;

    /// Save a thread with atomic write pattern.
    /// Creates thread directory if needed.
    /// Uses: write to unique tmp file, fsync, rename to target, cleanup on error.
    pub fn save(&self, thread: &Thread) -> Result<(), PersistenceError>;

    /// Load a thread by ID.
    /// Returns UnsupportedSchema error if version > CURRENT_SCHEMA_VERSION.
    /// Applies migrations if version < CURRENT_SCHEMA_VERSION.
    pub fn load(&self, id: &str) -> Result<Thread, PersistenceError>;

    /// Check if a thread exists (has valid thread.json).
    pub fn exists(&self, id: &str) -> bool;

    /// Delete a thread and all its data.
    /// Clears active_thread if this was the active thread.
    pub fn delete(&self, id: &str) -> Result<(), PersistenceError>;

    /// List all threads with summary info.
    /// Sorted by updated_at descending (most recent first).
    /// Skips directories with missing/corrupted thread.json (logs warning).
    pub fn list(&self) -> Result<Vec<ThreadSummary>, PersistenceError>;

    /// Get the active thread ID, if any.
    /// Returns None if file missing, empty, or points to non-existent thread.
    pub fn get_active(&self) -> Result<Option<String>, PersistenceError>;

    /// Set the active thread ID.
    /// Validates that the thread exists.
    pub fn set_active(&self, id: &str) -> Result<(), PersistenceError>;

    /// Clear the active thread.
    pub fn clear_active(&self) -> Result<(), PersistenceError>;

    /// Save a spec revision for a thread.
    /// Assigns next revision number (scans existing, increments max).
    /// Returns the revision number assigned.
    pub fn save_spec(&self, thread_id: &str, content: &str) -> Result<u32, PersistenceError>;

    /// Load a specific spec revision.
    pub fn load_spec(&self, thread_id: &str, revision: u32) -> Result<String, PersistenceError>;

    /// List available spec revisions for a thread.
    /// Returns sorted list of revision numbers.
    pub fn list_specs(&self, thread_id: &str) -> Result<Vec<u32>, PersistenceError>;

    /// Validate a thread ID for filesystem safety.
    /// Rejects empty, contains path separators, or traversal patterns.
    fn validate_id(id: &str) -> Result<(), PersistenceError>;
}
```

## Implementation Notes

### Atomic Writes

All writes use the atomic pattern to prevent corruption:
1. Generate unique temp filename: `thread.json.<random>.tmp`
2. Write content to temp file
3. Call `sync_all()` on the file to ensure data is on disk
4. Rename temp file to target (atomic on POSIX)
5. On any error, attempt to clean up temp file

```rust
fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let random: u64 = rand::random();
    let tmp_name = format!("{}.{:x}.tmp", path.file_name().unwrap().to_str().unwrap(), random);
    let tmp_path = path.with_file_name(tmp_name);

    let result = (|| {
        let mut file = File::create(&tmp_path)?;
        file.write_all(content)?;
        file.sync_all()?;  // fsync
        fs::rename(&tmp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);  // Best-effort cleanup
    }
    result
}
```

**Cross-platform note:** On Windows, `std::fs::rename` may fail if destination exists. Consider using the `atomicwrites` crate or platform-specific `ReplaceFile` API for robust cross-platform support. For v1, POSIX semantics are sufficient.

### Schema Versioning

The `ThreadFile` uses `#[serde(flatten)]` to embed Thread fields alongside `schema_version`:

```json
{
  "schema_version": 1,
  "id": "abc-123",
  "title": "Add feature X",
  "phase": {"type": "Running", "data": {"iteration": 3}},
  "current_run_id": "run-001",
  "baseline": {"branch": "main", "commit_sha": "abc123...", "captured_at": "2024-01-01T00:00:00Z"},
  ...
}
```

Note: ThreadPhase uses tagged enum serialization (`#[serde(tag = "type", content = "data")]`):
- Unit variants: `{"type": "Drafting"}`, `{"type": "Configuring"}`, `{"type": "Implemented"}`
- Data variants: `{"type": "Running", "data": {"iteration": 3}}`, `{"type": "Stuck", "data": {"diagnosis": {...}}}`

When loading:
1. Parse JSON to raw Value, extract `schema_version`
2. If version > CURRENT_SCHEMA_VERSION, return `UnsupportedSchema` error
3. If version < CURRENT_SCHEMA_VERSION, apply migrations in order
4. Deserialize to Thread

### Thread ID Validation

Thread IDs are UUIDs generated by `Thread::new()`. Validation rules:
- Non-empty
- No path separators (`/`, `\`)
- No traversal patterns (`..`)
- Only printable ASCII (alphanumeric, dash, underscore)

### Active Thread Semantics

- `active_thread` file contains a single thread ID (trimmed of whitespace)
- "Active" means "currently selected in UI for display/interaction"
- Does NOT mean "currently running" - that's determined by thread phase
- On app restart, if a thread is in `Running` phase, the app should transition it to `Paused` (handled by app startup, not F4)
- `get_active()` returns `None` if file is missing, empty, or points to deleted thread

### Handling Corrupted Data

- `list()`: Skip directories with missing/invalid thread.json, log warning
- `load()`: Return appropriate error (InvalidData, Json, UnsupportedSchema)
- `exists()`: Return false for corrupted threads
- `get_active()`: Return None if active thread doesn't exist

## Acceptance Criteria

- [ ] `ThreadStore::new()` creates threads directory if missing
- [ ] `save()` uses atomic write pattern (unique tmp, fsync, rename, cleanup)
- [ ] `save()` creates thread directory if needed
- [ ] `load()` returns `ThreadNotFound` for missing threads
- [ ] `load()` returns `UnsupportedSchema` for future versions
- [ ] `load()` correctly deserializes all Thread fields
- [ ] `exists()` returns true/false correctly
- [ ] `delete()` removes thread directory and all contents
- [ ] `delete()` clears active if deleting active thread
- [ ] `list()` returns summaries sorted by updated_at desc
- [ ] `list()` marks active thread with is_active=true
- [ ] `list()` skips corrupted thread directories gracefully
- [ ] `get_active()` returns None when no active thread
- [ ] `get_active()` returns None when active points to missing thread
- [ ] `get_active()` returns Some(id) when active and valid
- [ ] `set_active()` fails if thread doesn't exist
- [ ] `clear_active()` removes active_thread file
- [ ] `save_spec()` saves content and returns revision number
- [ ] `load_spec()` loads correct revision
- [ ] `list_specs()` returns available revisions sorted
- [ ] Thread survives save/load round-trip (all fields preserved)
- [ ] Schema version is written and checked on load
- [ ] Thread ID validation prevents path traversal
- [ ] `cargo build -p ralf-engine` succeeds
- [ ] `cargo clippy -p ralf-engine` has no warnings
- [ ] `cargo test -p ralf-engine` passes
- [ ] At least 18 unit tests covering persistence operations

## Non-Goals (for F4)

- Schema migrations (no migrations needed for v1)
- Concurrent access / locking (single-process assumption)
- Encryption of thread data
- Remote/cloud storage
- Backup/restore functionality
- Transcript persistence (handled by chat.rs)
- Run artifact persistence (handled by runner.rs)
- Auto-pausing Running threads on restart (app startup responsibility)

## Testing Strategy

Unit tests will use temporary directories:

```rust
fn setup_test_store() -> (TempDir, ThreadStore) {
    let temp = TempDir::new().unwrap();
    let store = ThreadStore::new(temp.path()).unwrap();
    (temp, store)
}
```

Tests should cover:
1. Save and load round-trip (all Thread fields preserved)
2. Atomic write behavior (file exists after save, no partial files)
3. List ordering (most recent first)
4. Active thread tracking (set, get, clear, invalid)
5. Spec revision management (save, load, list, gaps)
6. Error cases (not found, invalid ID, path traversal)
7. Schema version handling (current, future/unsupported)
8. Corrupted data handling (missing fields, invalid JSON)
9. Thread ID validation (valid UUIDs, reject traversal)
