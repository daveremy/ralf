---
active: true
iteration: 1
max_iterations: 30
completion_promise: "COMPLETE"
started_at: "2026-01-07T03:55:35Z"
---

# Task: Bootstrap ralf as a Rust Project

Set up `ralf` as a production-ready Rust workspace with CI, testing infrastructure, and release automation.

## Context

`ralf` is a multi-model autonomous loop engine with a TUI. This prompt focuses on **Milestone 0: Repo Bootstrap** — creating the foundational project structure before any feature implementation.

Read `SPEC.md` and `docs/DEV_ROADMAP.md` for full context.

## Scope

### 1. Cargo Workspace Structure

Create a Cargo workspace with three crates:

```
Cargo.toml              # workspace root
crates/
├── ralf-engine/        # headless engine (config, state, model adapters, verification)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── ralf-tui/           # TUI layer (ratatui + crossterm)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
└── ralf-cli/           # CLI entrypoint (binary)
    ├── Cargo.toml
    └── src/
        └── main.rs
```

Requirements:
- `ralf-cli` depends on `ralf-engine` and `ralf-tui`
- `ralf-tui` depends on `ralf-engine`
- The final binary is named `ralf`
- Use Rust 2021 edition
- Add these dependencies (latest stable versions):
  - `ralf-engine`: `serde`, `serde_json`, `thiserror`, `tokio` (async runtime)
  - `ralf-tui`: `ratatui`, `crossterm`
  - `ralf-cli`: `clap` (derive feature)

### 2. Minimal CLI Skeleton

Implement a basic CLI in `ralf-cli` using clap with these subcommands:

```
ralf                    # Default: opens TUI (stub for now)
ralf tui                # Opens TUI (stub for now)
ralf doctor [--json]    # Placeholder: "doctor not implemented"
ralf init               # Placeholder: "init not implemented"
ralf probe [--json]     # Placeholder: "probe not implemented"
ralf run                # Placeholder: "run not implemented"
ralf status [--json]    # Placeholder: "status not implemented"
ralf cancel             # Placeholder: "cancel not implemented"
ralf --version          # Print version
ralf --help             # Print help
```

The CLI should:
- Print version from `Cargo.toml`
- Show help text describing each command
- Exit cleanly with appropriate exit codes

### 3. GitHub Actions CI

Create `.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Format check
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
      - name: Test
        run: cargo test --all
```

### 4. Release Workflow

Create `.github/workflows/release.yml`:

- Triggered on tag push (`v*`)
- Builds binaries for:
  - `x86_64-unknown-linux-gnu`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
- Generates `SHA256SUMS`
- Creates GitHub release with attached artifacts

### 5. Install Scripts

Create `install/install.sh`:
- Supports `--version`, `--prefix`, `--dry-run`, `--sha256`
- Detects OS and architecture
- Downloads appropriate binary from GitHub releases
- Verifies checksum if provided
- Installs to `$PREFIX/bin/ralf` (default: `~/.local/bin`)

Create `install/uninstall.sh`:
- Removes `ralf` binary from install location

### 6. Project Files

Ensure these files exist and are appropriate:
- `LICENSE` (MIT)
- `README.md` (update with install instructions and project overview)
- `CONTRIBUTING.md` (contribution guidelines)
- `CODE_OF_CONDUCT.md` (Contributor Covenant)
- `SECURITY.md` (security policy)
- `CHANGELOG.md` (keep a changelog format, empty for now)
- `.gitignore` (Rust + IDE patterns)
- `rustfmt.toml` (sensible defaults)
- `.clippy.toml` (if needed)

### 7. Basic Tests

Add at least one test per crate to verify the test infrastructure works:
- `ralf-engine`: unit test for a placeholder function
- `ralf-tui`: unit test for a placeholder function
- `ralf-cli`: integration test that runs `ralf --help` and checks exit code

## Acceptance Criteria

All of the following must be true:

1. **Build succeeds**: `cargo build --release` completes without errors
2. **Tests pass**: `cargo test --all` passes
3. **Format clean**: `cargo fmt --all -- --check` passes
4. **Lint clean**: `cargo clippy --all-targets --all-features -- -D warnings` passes
5. **CLI works**:
   - `cargo run -- --help` prints help for all subcommands
   - `cargo run -- --version` prints version
   - `cargo run -- doctor` prints placeholder message
6. **Workspace structure**: All three crates exist with proper dependencies
7. **CI configured**: `.github/workflows/ci.yml` exists and would pass
8. **Release configured**: `.github/workflows/release.yml` exists
9. **Install scripts**: `install/install.sh` and `install/uninstall.sh` exist and are executable

## Constraints

- Do NOT implement actual features yet — this is scaffolding only
- Do NOT add dependencies beyond what is specified
- Keep code minimal but idiomatic Rust
- Use `#![deny(warnings)]` in lib.rs files to enforce clean code
- All public items should have doc comments

## Verification

Run these commands to verify completion:

```bash
# Build
cargo build --release

# Format check
cargo fmt --all -- --check

# Lint check
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all

# CLI smoke test
cargo run -- --help
cargo run -- --version
cargo run -- doctor
```

All commands must succeed with exit code 0 (except `doctor` which should print a message and exit 0).

## Completion

When all acceptance criteria are met and all verification commands pass, output exactly:

<promise>COMPLETE</promise>
