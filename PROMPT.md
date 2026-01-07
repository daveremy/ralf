# Task

Bootstrap `ralf` into a real open source project and implement the v0.1 MVP described in `SPEC.md`.

This is a standalone project (do not depend on `ralph-loop`). `ralf` must use local model CLIs (Claude, Codex, Gemini) and must not require API keys.

## Scope (v0.1 MVP)

### 1) Repository + releases

- Initialize a clean OSS repo structure (Rust recommended).
- Add CI (format + lint + tests).
- Add release workflow that builds binaries and produces `SHA256SUMS`.
- Add `install/install.sh` and `install/uninstall.sh` per `install/INSTALL_SPEC.md`.

### 2) Engine core (no real providers in CI)

Implement the headless engine (can be used by both CLI and TUI):
- `.ralf/` runtime layout:
  - `.ralf/config.json`
  - `.ralf/state.json`
  - `.ralf/cooldowns.json`
  - `.ralf/runs/<run-id>/...` logs/artifacts
  - `.ralf/changelog/<model>.md` (per model, required)
- Model discovery for `claude|codex|gemini`:
  - detect binaries on PATH
  - validate callable (e.g. `--help`)
  - generate default config including only detected models
- Model invocation:
  - one-shot prompt execution (stdin or final arg)
  - timeouts
  - capture stdout/stderr to run logs
- Rate-limit detection + cooldown:
  - pattern-based detection
  - persist cooldown_until/reason/observed_at
  - skip cooled down models
- Model selection:
  - implement `round_robin` and `priority` strategies (default: `round_robin`)
- Verification:
  - configurable verifiers (default required: `tests`)
  - completion requires required verifiers pass AND exact `<promise>…</promise>` match (default: `COMPLETE`)
- Changelogs:
  - append per-iteration entries to `.ralf/changelog/<model>.md`
  - include run_id/iter/model/status/reason/prompt_hash/git summary/verifier results/log paths

Important: CI tests must use stub model binaries/fixtures. Do not call real provider CLIs in CI.

### 3) TUI (beautiful MVP shell)

Implement a first usable TUI (Rust + ratatui recommended):
- Welcome/Setup screen:
  - detect repo/models
  - generate/save `.ralf/config.json`
  - run probes with timeouts and show actionable results
- Spec Studio MVP:
  - chat transcript + draft prompt pane
  - model selector (use only available models)
  - “Finalize” writes `PROMPT.md` and transitions to Run Dashboard
- Run Dashboard MVP:
  - start/cancel run
  - show iteration, selected model, cooldowns
  - show model output tail, verifier results, git diff summary

Do not implement “Review Round” UI yet unless it is quick and clean.

## Acceptance criteria

- `SPEC.md` and `docs/DEV_ROADMAP.md` are treated as source-of-truth for MVP behavior.
- `ralf --help` documents CLI commands: `tui`, `doctor`, `init`, `probe`, `run`, `status`, `cancel`.
- Running `ralf` in a git repo opens the TUI and can:
  - create `.ralf/config.json`,
  - create/edit a prompt in Spec Studio,
  - start a bounded run,
  - show progress and artifacts.
- Headless engine passes automated tests (unit + integration with stub models).
- Installer works per `install/INSTALL_SPEC.md`.
- Output exactly: <promise>COMPLETE</promise>

## Constraints

- Do not require API keys; use local CLIs only.
- Keep user secrets out of logs (avoid printing env vars; redact obvious key patterns if surfaced).
- Avoid “clever” hidden behavior; always write artifacts to `.ralf/`.
- Default to shared working tree; allow an option to run on a branch.

## Verification

- Add and run a fast local test suite (e.g. `cargo test`) and keep it green.
- Ensure CI workflow passes (fmt, clippy, tests).

