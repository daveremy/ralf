# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Milestone 1: Engine Core — COMPLETE

#### Added
- **Config module** (`config.rs`): Configuration types with JSON serialization
  - `Config`, `ModelConfig`, `VerifierConfig`, `ModelSelection`
  - Default configurations for claude, codex, gemini
  - Load/save to `.ralf/config.json`
- **State module** (`state.rs`): Run state and cooldown management
  - `RunState` with lifecycle methods (start, complete, cancel, fail)
  - `Cooldowns` with per-model tracking and expiry
  - Persistence to `.ralf/state.json` and `.ralf/cooldowns.json`
- **Discovery module** (`discovery.rs`): Model discovery and probing
  - `discover_models()` finds known CLIs on PATH
  - `probe_model()` tests responsiveness with timeout
  - Auth detection and actionable suggestions
- **Runner module** (`runner.rs`): Core execution logic
  - `invoke_model()` with async timeout handling
  - `run_verifier()` for verification execution
  - `select_model()` with round-robin and priority strategies
  - `check_promise()` and `extract_promise()` for completion detection
  - `hash_prompt()` for changelog deduplication
  - `get_git_info()` for branch and changed files tracking
- **Changelog module** (`changelog.rs`): Per-model changelog generation
  - `ChangelogEntry` struct with all iteration metadata
  - Markdown output to `.ralf/changelog/<model>.md`
  - `IterationStatus` enum (Success, RateLimited, Timeout, Error, VerifierFailed)
- **CLI commands**: Wired up all commands to engine
  - `doctor` - discovers and displays available models
  - `init` - creates `.ralf/` directory structure and config
  - `probe` - tests model responsiveness with configurable timeout
  - `status` - shows run state and cooldowns
  - `cancel` - cancels active run
- **Test stubs** (`tests/stubs/`): Shell scripts for integration testing
  - `stub-model-ok`, `stub-model-fail`, `stub-model-ratelimit`
  - `stub-model-timeout`, `stub-model-no-promise`, `stub-model-help`
  - `stub-verifier-pass`, `stub-verifier-fail`

#### Dependencies Added
- `which` - for binary discovery on PATH
- `regex` - for rate-limit pattern matching and promise extraction
- `sha2` - for prompt hashing
- `chrono` - for timestamps
- `uuid` - for run ID generation
- `tempfile` (dev) - for tests

### Milestone 0: Repo Bootstrap — COMPLETE

#### Added
- Cargo workspace with 3 crates (`ralf-engine`, `ralf-tui`, `ralf-cli`)
- CLI skeleton using clap with subcommands: `tui`, `doctor`, `init`, `probe`, `run`, `status`, `cancel`
- GitHub Actions CI workflow (fmt, clippy, test with `--locked`)
- Release workflow with musl for Linux, tar.gz packaging, test gate
- Install/uninstall scripts with version pinning and checksum verification
- Centralized workspace lints in root `Cargo.toml`
- `rust-toolchain.toml` to pin stable channel
- Project files: LICENSE (MIT), CONTRIBUTING, CODE_OF_CONDUCT, SECURITY
- Comprehensive documentation: SPEC.md, DEV_ROADMAP.md, and docs/

### Next: Milestone 2 — TUI Foundation
