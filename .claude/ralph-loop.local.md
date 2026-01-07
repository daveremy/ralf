---
active: true
iteration: 1
max_iterations: 50
completion_promise: "COMPLETE"
started_at: "2026-01-07T04:17:54Z"
---

# Task: Implement ralf Engine Core (Milestone 1)

Implement the headless engine for ralf — the core orchestration logic that can be used by both CLI and TUI.

## Context

This is **Milestone 1** per `docs/DEV_ROADMAP.md`. The repo bootstrap (Milestone 0) is complete. Now we implement the engine without any TUI work.

Read `SPEC.md` sections on Configuration, Engine behavior, and Rate-limits for full context.

## Scope

### 1. Configuration & State Storage

Implement `.ralf/` runtime directory structure in `ralf-engine`:

```
.ralf/
├── config.json       # Model selection, verifiers, completion policy
├── state.json        # Current run state (run_id, iteration, status)
├── cooldowns.json    # Per-model cooldown tracking
├── runs/<run-id>/    # Per-run logs and artifacts
│   ├── <model>.log   # Stdout/stderr from model invocations
│   └── verifier.log  # Verifier output
└── changelog/
    └── <model>.md    # Per-model iteration logs (required)
```

**Config schema** (`config.json`):
```json
{
  "model_priority": ["claude", "codex", "gemini"],
  "model_selection": "round_robin",
  "required_verifiers": ["tests"],
  "completion_promise": "COMPLETE",
  "models": [
    {
      "name": "claude",
      "command_argv": ["claude", "-p", "--output-format", "text", "--dangerously-skip-permissions"],
      "timeout_seconds": 300,
      "rate_limit_patterns": ["429", "rate limit", "quota", "too many requests"],
      "default_cooldown_seconds": 900
    }
  ],
  "verifiers": [
    { "name": "tests", "command_argv": ["cargo", "test"], "timeout_seconds": 300 }
  ]
}
```

**Cooldowns schema** (`cooldowns.json`):
```json
{
  "claude": {
    "cooldown_until": 1710000000,
    "reason": "rate limit detected",
    "observed_at": 1709999900
  }
}
```

### 2. Model Discovery

Implement `ralf doctor` command in `ralf-engine`:

- Detect `claude`, `codex`, `gemini` binaries on PATH
- Validate each is callable (run with `--help`, check exit code)
- Return discovery results as structured data
- Support `--json` flag for machine-readable output

Implement `ralf init` command:
- Create `.ralf/` directory structure
- Generate default `config.json` with only detected models
- Use sensible defaults from SPEC.md

Implement `ralf probe` command:
- Run each model with a simple test prompt and timeout
- Detect hangs (auth prompts, OAuth flows)
- Report results with actionable guidance

### 3. Model Invocation

Implement model execution in `ralf-engine`:

- One-shot prompt execution (pass prompt via stdin or argument per model config)
- Configurable timeout per model
- Capture stdout/stderr to `.ralf/runs/<run-id>/<model>.log`
- Return structured result (exit code, output, duration, rate_limit_detected)

### 4. Rate-Limit Detection & Cooldown

- Pattern-based detection using configurable regex patterns per model
- When detected:
  - Write cooldown entry to `cooldowns.json`
  - Skip model in subsequent iterations until cooldown expires
- When all models cooling: sleep until earliest expires (clamped to reasonable max)

### 5. Model Selection Strategies

Implement in `ralf-engine`:

- `round_robin` (default): Rotate through available models, skip those in cooldown
- `priority`: Use first non-cooldown model from priority list

### 6. Verification System

- Configurable verifiers (command + timeout)
- Default required verifier: `tests`
- Run verifiers after model iteration
- Capture output to `.ralf/runs/<run-id>/verifier.log`

### 7. Completion Policy

Loop completes only when ALL conditions are met:
- All required verifiers pass (exit code 0)
- Model output contains exact `<promise>COMPLETION_TEXT</promise>` (default: `COMPLETE`)

### 8. Changelog Generation

After each iteration, append entry to `.ralf/changelog/<model>.md`:

```markdown
## Run <run_id> — Iteration <n>

- **Model**: claude
- **Status**: success | rate_limited | timeout | error
- **Reason**: <why this status>
- **Prompt hash**: <sha256 of prompt>
- **Git branch**: main
- **Git dirty**: true
- **Changed files**: src/lib.rs, tests/test_foo.rs
- **Verifier results**:
  - tests: pass
- **Logs**: .ralf/runs/<run_id>/claude.log
```

### 9. Loop Runner (Headless)

Implement `ralf run` command:

- Accept options: `--max-iterations`, `--max-seconds`, `--branch`, `--models`
- Read `PROMPT.md` from repo root as the stable prompt
- Execute iteration pipeline:
  1. Select model (per strategy, skip cooldowns)
  2. Invoke model with prompt
  3. Check for rate-limit patterns
  4. Run verifiers
  5. Check completion (verifiers pass + promise tag)
  6. Write changelog entry
  7. Update state
  8. Repeat or exit

Implement `ralf status` command:
- Print current run state and cooldowns
- Support `--json` flag

Implement `ralf cancel` command:
- Write cancel state
- Best-effort process termination

## Testing Strategy

**Critical**: All tests must use stub/mock models. Do NOT call real provider CLIs in tests.

### Stub Models

Create test fixtures (shell scripts or Rust test binaries) that simulate:
- Success with promise tag
- Success without promise tag
- Rate limit response
- Timeout (hang)
- Error exit

### Unit Tests

- Config parsing and validation
- Model selection algorithms
- Rate-limit pattern matching
- Promise tag extraction
- Changelog formatting

### Integration Tests

- Full iteration cycle with stub model
- Cooldown written when rate limit detected
- Changelog entry appended with required fields
- Completion requires verifiers pass AND promise present
- State persistence across iterations

## Acceptance Criteria

1. **`ralf doctor --json`** returns discovered models with availability status
2. **`ralf init`** creates `.ralf/` with valid `config.json`
3. **`ralf run`** executes loop against stub models in tests
4. **Cooldowns work**: rate-limit detection triggers cooldown, model is skipped
5. **Changelogs work**: each iteration appends entry with required fields
6. **Completion works**: loop exits only when verifiers pass AND promise found
7. **All tests pass**: `cargo test --workspace --locked`
8. **Clippy clean**: `cargo clippy --workspace --all-features --locked -- -D warnings`

## Constraints

- Do NOT implement TUI — this is headless engine only
- Do NOT call real provider CLIs in tests
- Keep the engine independent of UI concerns
- Use async where appropriate (tokio) for process spawning and timeouts
- All public types and functions must have doc comments

## Verification

```bash
# Build
cargo build --workspace --locked

# Lint
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Test
cargo test --workspace --locked

# CLI smoke tests
cargo run -- doctor --json
cargo run -- init
cargo run -- status --json
```

## Completion

When all acceptance criteria are met and all verification commands pass, output exactly:

<promise>COMPLETE</promise>
