# Developer roadmap (implementation plan)

This roadmap is a **build plan**, not a user-facing feature list. It is structured to ship a usable, polished “assistant-like” multi-model TUI while keeping the loop engine correct, testable, and auditable.

## Key decisions (recommended)

### Language + UI stack

- **Rust** for the shipped binary.
- **ratatui + crossterm** for the TUI.
- Use local model CLIs for all model calls (no API keys).

Rationale:
- Single self-contained binary per platform is the best install story.
- TUI quality and performance are strong.
- Process execution (spawn, stream, timeout, kill) is robust in Rust.

### Default run strategy

- Default model selection: `round_robin` across all detected models (skip cooldowns).
- Completion: required verifiers pass **and** exact `<promise>…</promise>`.

## Milestones

Each milestone includes a “definition of done” that should be testable.

### Milestone 0 — Repo bootstrap

Deliverables:
- Cargo workspace skeleton (`ralf` binary).
- Docs in place (`README.md`, `SPEC.md`, `docs/*`).
- CI:
  - formatting (rustfmt),
  - lint (clippy),
  - tests.
- Release plan docs (`install/INSTALL_SPEC.md`).

DoD:
- `cargo test` passes in CI.
- `cargo fmt --check` and `cargo clippy` pass in CI.

### Milestone 1 — Engine core (no TUI)

Deliverables:
- Config/state storage under `.ralf/`:
  - `config.json`, `state.json`, `cooldowns.json`, `runs/<run-id>/…`, `changelog/<model>.md`.
- Model discovery:
  - detect `claude|codex|gemini` on PATH,
  - validate they are callable (`--help`),
  - write a default config containing only detected models.
- Loop runner (headless):
  - iteration loop,
  - rate-limit detection + cooldown,
  - verifiers (at least `tests`),
  - completion policy (tests + promise tag),
  - per-model changelog entries.

DoD:
- `ralf doctor --json` returns discovered models.
- `ralf run` works against fixture “stub models” in tests (no real CLIs).
- Integration tests validate:
  - cooldown is written,
  - changelog entry is appended,
  - completion requires verifiers + promise.

### Milestone 2 — TUI foundation (beautiful shell)

Deliverables:
- TUI shell with:
  - theming (colors, typography), consistent layout,
  - command palette/help overlay,
  - two panels/tabs framework,
  - non-blocking event loop,
  - log viewer component (tail + scroll).
- “Welcome / Setup” screens implemented, wired to engine:
  - detect repo + models,
  - generate config,
  - run `probe` with timeouts,
  - show actionable fixes and allow disabling a model.

DoD:
- Running `ralf` opens the TUI and completes setup in one terminal.
- Setup writes `.ralf/config.json` and records last probe results.

### Milestone 3 — Spec Studio MVP (chat + finalize)

Deliverables:
- Spec Studio screen:
  - transcript pane,
  - draft prompt/spec pane,
  - input box and model selector,
  - “Finalize” flow.
- Thread persistence:
  - `.ralf/spec/threads/<id>.jsonl`
  - `.ralf/spec/drafts/<timestamp>.md`
- One-shot model invocation for chat turns:
  - pass bounded transcript + draft to the model,
  - stream output when CLI supports it (best-effort),
  - record artifacts.
- Finalize writes `PROMPT.md` and transitions to Run Dashboard.

DoD:
- User can author a multi-turn spec dialog and produce a valid `PROMPT.md` without leaving the TUI.
- Validation blocks finalize if `<promise>…</promise>` is missing.

#### Future Enhancements (Spec Studio)
- **Thread history/resume**: Welcome screen shows list of threads (like Claude's conversation history). User can resume in-progress specs or start new ones.
- **Skip Welcome for active work**: If config exists and a thread is in progress, go directly to Spec Studio.
- **Mouse selection**: Enable mouse support for selecting/copying transcript text.
- **Export transcript**: Keybinding to export full transcript to file for external use.
- **Archive/delete threads**: Manage old threads from Welcome screen.

### Milestone 4 — Loop Runner dashboard MVP

Deliverables:
- Run Dashboard screen:
  - run_id, iteration, selected model, elapsed time,
  - cooldowns,
  - timeline events,
  - tabs: model output, verifiers, git summary, changelog preview.
- Run control:
  - start run with settings (branch, max iterations/seconds, model subset),
  - cancel run.
- Robust process control:
  - timeouts,
  - kill tree,
  - avoid TUI corruption (PTY handling).

DoD:
- A run can be started and canceled from the TUI.
- TUI remains responsive during model execution and verifier runs.

#### Future Enhancements (Run Dashboard)
- **Configurable max iterations**: Settings overlay to configure max iterations before starting a run.
- **Run history**: Show previous runs with their outcomes (completed/failed/cancelled).
- **Live cooldown countdown**: Decrement cooldown timers in real-time instead of clearing on iteration start.
- **Detailed diff viewer**: Expand Git tab to show actual file diffs, not just changed file list.
- **Pause/Resume**: Add ability to pause after current iteration and resume later.

### Milestone 5a — AI-powered criteria verification

Deliverables:
- **Criteria Verification Engine**:
  - After model outputs `<promise>COMPLETE</promise>`, invoke a verifier model.
  - Verifier model receives: criteria list + current repo state (git diff, file contents).
  - Verifier responds with structured PASS/FAIL for each criterion.
  - Parse structured response and extract per-criterion results.
- **UI Updates**:
  - Display criteria with ✓/✗ in the Criteria pane based on verification.
  - Show verification in progress state.
  - Only complete run if all criteria verified as PASS.
  - If criteria fail, continue to next iteration (model can retry).

DoD:
- After promise detected, criteria are verified by a different model.
- Criteria pane shows PASS/FAIL status for each criterion.
- Run only completes when all criteria pass; otherwise continues iterating.

### Milestone 5b — TUI integration & snapshot testing

Deliverables:
- **Snapshot Testing Infrastructure**:
  - Test utilities module with `create_test_terminal()`, `create_test_app()` helpers.
  - Widget snapshot tests using insta (StatusBar, KeyHint, LogViewer, TextInput).
  - Screen snapshot tests for all screens (Welcome, Setup, SpecStudio, RunDashboard).
  - Consistent 80x24 terminal dimensions, filtered dynamic content.
- **E2E Test Harness**:
  - Mock event injection for keyboard input simulation.
  - Screen state capture and assertion helpers.
  - User flow tests: Welcome→Setup, navigation, Run Dashboard states.
- **CI Integration**:
  - All snapshot tests run in CI.
  - New/changed snapshots require explicit review and commit.

DoD:
- At least 5 widget snapshot tests and 4 screen snapshot tests exist.
- Welcome → Setup navigation flow test passes.
- Run Dashboard state transitions test passes.
- Criteria verification display test passes.
- `cargo test` includes all snapshot tests, CI passes.

### Milestone 5c — Review rounds

Deliverables:
- **Review Round screen**:
  - Run "spec review" prompts through other available models.
  - Present findings as structured checklist.
  - Apply changes into the draft prompt/spec.

DoD:
- User can run at least one review round and apply suggestions into the draft inside the TUI.

### Milestone 6 — Production polish + release

Deliverables:
- Installer (`install/install.sh`, `install/uninstall.sh`) matching `install/INSTALL_SPEC.md`.
- Release automation (GitHub Actions):
  - build cross-platform binaries,
  - attach artifacts,
  - generate `SHA256SUMS`.
- Troubleshooting docs expanded with real failure modes (Keychain, timeouts, CLI auth prompts).

DoD:
- `curl | bash` installs a pinned version and `ralf --version` works.
- “First run experience” is smooth for users with at least one model installed.

## Test strategy

### Unit tests
- JSON config schema parsing/validation.
- Model selection algorithms (round robin, priority).
- Rate-limit detection regex matching.
- Promise extraction.

### Integration tests (no real providers)
- Stub model scripts/binaries that:
  - output a promise,
  - output a rate limit string,
  - hang (to test timeouts),
  - write a file (to simulate "work done").
- Criteria parsing tests:
  - extract bullet points from various markdown formats.
- Assert:
  - changelog content includes required fields,
  - state/cooldowns updated as expected,
  - completion triggers on promise tag.

### TUI integration tests (Milestone 5b)
- **insta**: Snapshot testing for visual regression
  - Capture TestBackend buffer state
  - Compare against reference snapshots
  - Widget and screen-level snapshots
- **E2E test harness**: Mock event injection for user flows
  - Keyboard input simulation
  - Screen state capture and assertions
- Priority test flows:
  - Welcome → Setup → config saved
  - Navigation (Tab, Escape, ?)
  - Run Dashboard state transitions
  - Criteria verification display

### Manual smoke tests
- `claude`, `codex`, `gemini` actual probes on macOS.
- Gemini Keychain prompt path mitigation flow.

## Key risks and mitigations

- **CLI auth prompts block the process**: always run probes with timeouts; surface clear actions to user.
- **Streaming differences across CLIs**: support “best-effort streaming” but never rely on it for correctness.
- **Context explosion**: enforce strict size budgets for transcript + repo context; provide “summarize thread” action.
- **TUI complexity**: keep engine pure (no UI concerns) and communicate via typed events.

