# ralf — Specification (Standalone Multi-Modal TUI + Loop Engine)

## Summary

`ralf` is a standalone CLI + TUI that provides a **phased, assistant-like experience**:

1) **Spec / Prompt phase (“Spec Studio”)**: interactive multi-modal dialog to converge on a high-quality spec and a stable `PROMPT.md` with explicit completion criteria.
2) **Run phase (“Loop Runner”)**: an autonomous multi-modal loop that iterates on the working tree until programmatic verification passes and the completion promise is produced.

`ralf` uses **installed model CLIs** (Claude, Codex, Gemini) and does **not** require users to obtain API keys.

## Background: the “Ralph Wiggum” method

“Ralph” loops keep feeding an agent a stable prompt until it is *actually* done.

Key principles:
- **Stable prompt; mutable world**: the prompt is stable; progress is changes in the repo.
- **Programmatic completion**: “done” must be checkable (tests/build/lint) plus an explicit completion promise tag.
- **Safety**: budgets, backoff, cooldowns, and circuit breakers prevent runaway loops.
- **Auditability**: persistent logs, diffs, and changelogs per modal.

## Goals

- Provide a **beautiful TUI** that feels like a modern AI assistant, with:
  - a guided Spec Studio,
  - a Run Dashboard,
  - clear progress and strong safety affordances.
- Provide a robust loop engine:
  - multi-modal (Claude/Codex/Gemini) by default,
  - per-modal rate-limit detection + cooldown,
  - best practices each iteration (git, tests, docs, changelogs).
- Be a high-quality **open source** repository:
  - strong project structure, docs, CI, releases,
  - easy install (curl|bash acceptable), reproducible releases.

## Non-goals

- Replacing provider CLIs or managing provider accounts.
- Requiring API keys as a prerequisite for basic operation.
- Fully autonomous “unsafe” operation without explicit user opt-in.

## Target UX

### Entry point

Users run `ralf` from a repo directory:

```bash
cd /path/to/repo
ralf
```

`ralf` immediately:
- detects if this is a git repo (warns if not),
- detects which modals are runnable (`claude`, `codex`, `gemini`),
- offers guided setup (auto-config) and remembers choices under `.ralf/`.

### Default experience: one command, everything inside the UI

The primary user journey should require **one terminal**:

1) `ralf` opens into a **Welcome / Setup** screen (if `.ralf/config.json` missing).
2) `ralf` transitions into **Spec Studio** (chat-like spec dialog).
3) When the user clicks **Finalize**, `ralf` writes `PROMPT.md` and transitions into **Loop Runner**.
4) When the loop completes, `ralf` shows a **Completion Summary** with artifacts and next actions.

### Two phases (core mental model)

**Phase A: Spec Studio**
- A chat-like interface that lets the user:
  - discuss requirements with one or more modals,
  - draft/iterate a spec and acceptance criteria,
  - run multi-modal “review rounds”,
  - finalize `PROMPT.md` (the stable loop prompt).

**Phase B: Loop Runner**
- A dashboard that runs the autonomous loop with:
  - live iteration logs,
  - per-modal outputs,
  - cooldown/rate-limit visibility,
  - verifier results (tests, etc.),
  - git status/diff summaries,
  - pause/cancel controls.

### “Feels like an assistant”

The UI should:
- have a single obvious “next action” most of the time,
- show a timeline and artifacts, not just chat text,
- make safety controls prominent,
- minimize “go to another terminal window” needs.

## Design constraints (hard requirements)

### Uses CLIs, not APIs

- The engine must invoke local CLIs for modals (no SDKs / keys required by default).
- It must support one-shot operation per modal, driven by stdin or an argument.

### Multi-modal by default (unless user restricts)

- By default, `ralf` uses **all detected modals** (`claude`, `codex`, `gemini`) during a run.
- Users can restrict to a subset for a run or for a project via config/UI.

### Default completion policy: tests + promise tag

The loop completes only when:
- required verifiers pass (default: `tests`), AND
- the agent output contains an exact `<promise>…</promise>` matching configured promise (default: `COMPLETE`).

### Working tree preference: shared working tree

Default behavior is a **shared working tree** (no worktrees), with:
- optional `--branch <name>` (or UI toggle) to isolate changes on a branch.

### Per-modal changelog required

Each iteration appends an entry to:
- `.ralf/changelog/<modal>.md` (required per modal)
- `.ralf/changelog/global.md` (optional rollup)

Changelog entries must include:
- run_id, iteration, modal name, status, reason,
- prompt hash,
- git branch + dirty + changed files summary,
- verifier results + links to logs.

### Rate-limit detection per modal

- Each modal has a set of patterns to detect rate-limits / caps.
- When detected: modal enters cooldown and is skipped until cooldown expires.
- Cooldown metadata is persisted.

## Supported modals (v0)

Required:
- `claude` (Anthropic CLI)
- `codex` (OpenAI Codex CLI)
- `gemini` (Google Gemini CLI)

Optional later (not required):
- additional modals can be added via config, but no first-class UX is required initially.

### Default safe invocations

Users indicated “run outside sandbox” flags are desired:
- Claude: `--dangerously-skip-permissions`
- Codex: `exec --dangerously-bypass-approvals-and-sandbox -`
- Gemini: `-y` plus flags to avoid sandbox hangs (commonly: `--sandbox=false -e none`), prompt via `-p`.

`ralf` should ship with versioned presets, but keep all of this configurable.

## Architecture

### One executable, two layers

- **Engine**: loop orchestration, config/state, modal adapters, cooldowns, verification, changelogs.
- **TUI**: Spec Studio + Run Dashboard, built on top of engine APIs.

### Recommended implementation language

**Rust** is recommended for:
- single self-contained binaries per platform,
- a high-quality TUI ecosystem (e.g. `ratatui` + `crossterm`),
- strong process control for spawning CLIs.

Alternative acceptable:
- Python + Textual (beautiful UI; requires Python runtime and packaged dependencies).

This spec assumes **Rust + ratatui** as the primary target, with an installer that ships binaries.

## CLI surface (non-TUI mode)

The TUI is the default, but `ralf` must also be usable non-interactively for scripting and CI-like workflows.

Proposed commands:

- `ralf` (default): opens the TUI
- `ralf tui`: opens the TUI (explicit)
- `ralf doctor [--json]`: detects modals, prints diagnostics
- `ralf init [--profile NAME]`: writes `.ralf/config.json` and `.ralf/` scaffolding in the current repo
- `ralf probe [--json] [--modal NAME]`: one-shot probe per modal with timeout to detect auth prompts/hangs
- `ralf run [--max-iterations N] [--max-seconds N] [--branch NAME] [--modals a,b,c]`: runs the autonomous loop
- `ralf status [--json]`: prints state + cooldowns
- `ralf cancel`: cancels the current run (best-effort) and writes state

The CLI and TUI must share one engine implementation; do not duplicate orchestration logic.

## Repository structure (open source)

Required layout:

- `src/` (engine + tui)
- `crates/` (optional split: `engine`, `tui`, `cli`)
- `docs/`
  - `ROADMAP.md`
  - `TROUBLESHOOTING.md`
  - `MODALS.md`
  - `CONFIG.md`
- `install/`
  - `install.sh` (curl|bash)
  - `uninstall.sh`
- `.github/workflows/ci.yml`
- `LICENSE`, `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`
- `CHANGELOG.md` (project release notes; distinct from per-modal run changelogs)

Runtime layout inside a user repo:

- `.ralf/` (gitignored)
  - `config.json`
  - `state.json`
  - `cooldowns.json`
  - `runs/<run-id>/...` (logs/artifacts)
  - `changelog/<modal>.md` (required)
  - `changelog/global.md` (optional)
  - `spec/` (spec studio transcripts, drafts)

## Configuration

Config is JSON for stable parsing and easy portability.

### Top-level keys (draft)

```json
{
  "modal_priority": ["claude", "codex", "gemini"],
  "modal_selection": "round_robin",
  "required_verifiers": ["tests"],
  "completion_promise": "COMPLETE",
  "checkpoint_commits": false,
  "modals": [
    {
      "name": "claude",
      "prompt_mode": "stdin",
      "command_argv": ["claude", "-p", "--output-format", "text", "--dangerously-skip-permissions"],
      "timeout_seconds": 300,
      "rate_limit_detection": { "patterns": ["429", "rate limit", "quota", "too many requests"], "default_cooldown_seconds": 900 }
    }
  ],
  "verifiers": [
    { "name": "tests", "command_argv": ["bash", "-lc", "npm test"], "run_when": "on_change", "timeout_seconds": 1800 }
  ]
}
```

### Modal selection strategies

- `priority`: pick first non-cooldown modal each iteration (simple fallback).
- `round_robin` (recommended default): rotate across available modals, skipping cooldowns.
- Future: `roles` (builder/reviewer/doc/tester).

## Engine behavior

### Iteration pipeline

Each iteration performs:

1) Preflight:
   - verify git repo and working tree policy,
   - load config/state/cooldowns,
   - ensure run directory exists.
2) Modal selection:
   - choose next modal (round robin by default),
   - skip cooldown modals.
3) Context assembly:
   - stable prompt text (`PROMPT.md`),
   - lightweight context bundle (recent diffs, last verifier failures) bounded by size.
4) Modal invocation:
   - run CLI with timeout,
   - capture stdout/stderr to `.ralf/runs/<run-id>/<modal>.log`.
5) Rate-limit detection:
   - if matched, write cooldown and mark iteration as rate-limited for that modal.
6) Git bookkeeping:
   - snapshot `git status --porcelain`,
   - diffstat, changed files list.
   - optional checkpoint commit.
7) Verification:
   - run verifiers depending on `run_when`,
   - always run required verifiers when promise tag is present.
8) Completion evaluation:
   - done only when required verifiers pass and promise matches.
9) Changelog + state update:
   - append per-modal changelog entry,
   - update `.ralf/state.json`.

### Rate-limits & cooldowns

Cooldowns persisted as:

```json
{
  "claude": { "cooldown_until": 1710000000, "reason": "rate limit", "observed_at": 1709999900 }
}
```

When all modals are cooling down:
- sleep until the earliest cooldown expires, clamped (e.g. 60s),
- show an explicit UI status.

### Best-practice requirements per iteration

Required:
- always capture git status + diff summary,
- always run verifiers per config policy,
- always write changelog entries (per modal),
- always persist artifacts under `.ralf/`.

Optional (config):
- checkpoint commits after each iteration (message: `ralf(<modal>): iter <n> (run <run_id>)`).

## TUI specification

### Overall UI principles

- **Guided**: always show “what’s next”.
- **Transparent**: show what it is doing and why (selected modal, cooldown reasons).
- **Interruptible**: pause/cancel prominently.
- **Artifact-first**: always link to logs/diffs/verifier outputs.

### Screen-by-screen flows (wireframes)

The sections below define the “assistant-like” phased experience in explicit screens and transitions.

#### Screen 0: Welcome / Repo detection

Purpose: confirm where we are, what is available, and what `ralf` will do next.

Displayed:
- repo path and git status (repo detected / not detected)
- detected modals (`claude`, `codex`, `gemini`) with status: available / missing / runnable / needs attention
- primary call-to-action button: `Continue setup` (if not configured) or `Open Spec Studio` (if configured)

Wireframe (conceptual):

```
┌ ralf ───────────────────────────────────────────────────────────────┐
│ Repo: /path/to/repo   Git: OK (branch main, clean)                  │
│ Modals:  claude ✅   codex ✅   gemini ⚠ (oauth prompt risk)         │
│                                                                      │
│ Next: Setup → Spec Studio → Run Loop                                │
│                                                                      │
│ [Continue setup]   [Open Spec Studio]   [Quit]                      │
└──────────────────────────────────────────────────────────────────────┘
```

Transitions:
- If `.ralf/config.json` missing → Screen 1
- If config present → Screen 2

#### Screen 1: Setup (auto-config + probe + fixes)

Purpose: generate a working config without sending the user to other tools/windows.

Steps:
1) Detect available modals (PATH + `--help` call).
2) Choose selection mode (default: `round_robin`).
3) Confirm verifier defaults (default: `tests`).
4) Run `probe` per modal (with timeout) and show results.
5) If a modal needs intervention, show a guided fix (or “skip this modal”).
6) Save `.ralf/config.json` and initialize `.ralf/` directories.

Fixes must be explicit and safe:
- Gemini macOS Keychain prompt: show explanation and a “Pin gemini+node path” toggle.
- Missing modal: show install hint and allow proceeding without it.
- Hang/timeout: allow increasing timeout or disabling that modal.

Wireframe:

```
┌ Setup ──────────────────────────────────────────────────────────────┐
│ Detected modals                                                     │
│  ✅ claude     ✅ codex     ⚠ gemini (may prompt Keychain)           │
│                                                                      │
│ Run mode:  (•) round_robin   ( ) priority                            │
│ Completion: required verifiers + <promise>COMPLETE</promise>         │
│                                                                      │
│ Probe results                                                       │
│  claude: OK                                                         │
│  codex:  OK                                                         │
│  gemini: TIMEOUT (likely auth prompt)  [Fix…] [Disable gemini]       │
│                                                                      │
│ [Save config] [Back]                                                │
└──────────────────────────────────────────────────────────────────────┘
```

#### Screen 2: Spec Studio (chat + spec drafting)

Purpose: interactive multi-turn dialog to converge on the implementation prompt/spec.

Key design: this should feel like Claude/Codex chat:
- left: transcript
- right: structured spec/prompt being built
- bottom: input box with model selector and “tools” actions

Conversation model:
- Each user message and model response is appended to `.ralf/spec/threads/<thread-id>.jsonl`.
- The prompt passed to each modal is synthesized from:
  - system instructions (stable),
  - the ongoing transcript (bounded),
  - the current draft spec/prompt (bounded),
  - optional repo context (bounded).

Wireframe:

```
┌ Spec Studio ────────────────────────────────────────────────────────┐
│ Transcript (left)                      Draft (right)                │
│ ────────────────────────────────       ───────────────────────────  │
│ You: We need feature X...              Title: …                      │
│ Claude: Questions…                     Goals: …                      │
│ You: Clarify…                          Acceptance: …                 │
│                                       Verifiers: tests, lint         │
│                                       Promise: COMPLETE              │
│                                                                      │
│ Model: [claude ▼]   [Send] [Review round] [Finalize] [Run] [Help]    │
│ >                                                              _     │
└──────────────────────────────────────────────────────────────────────┘
```

#### Screen 3: Review Round (multi-modal spec review)

Purpose: “have the other models review the spec” until it is final.

Behavior:
- For each non-selected modal, run a one-shot “spec review” prompt that returns structured feedback (prefer JSON, fall back to text parsing).
- Present feedback as a checklist with severity (blocker/warn/nice-to-have).
- Provide “Apply” actions that either:
  - update the Draft (structured fields), or
  - append actionable tasks to an “Open Questions” section.

Wireframe:

```
┌ Review Round ────────────────────────────────────────────────────────┐
│ Target: Draft v7    Reviewers: codex, gemini                         │
│                                                                      │
│ [BLOCKER] Missing acceptance criterion for edge case Y   [Apply]     │
│ [WARN]    Tests to add: …                                [Apply]     │
│ [NICE]    Docs update suggestion …                        [Apply]     │
│                                                                      │
│ [Back to Spec]  [Run another round]                                  │
└──────────────────────────────────────────────────────────────────────┘
```

#### Screen 4: Finalize Prompt (preflight + write files)

Purpose: ensure the prompt is convergent and checkable.

Checks performed before enabling “Finalize”:
- `PROMPT.md` includes: Task, Acceptance criteria, Constraints, Verification, `<promise>…</promise>`
- Verifiers exist and are runnable (at least syntactically; can be verified via probe/doctor)
- Run limits set (max iterations / optional max seconds)

Finalize writes:
- `PROMPT.md` at repo root
- optional `SPEC.md` at repo root (user toggle)
- `.ralf/spec/drafts/<timestamp>.md` snapshot

#### Screen 5: Run Dashboard (loop runner)

Purpose: run and monitor the loop, with rich observability.

Must show:
- iteration, selected modal, status
- cooldowns and reasons
- verifier results with drill-down logs
- git diffstat and changed file list
- current modal output stream (if available)

Wireframe:

```
┌ Run ────────────────────────────────────────────────────────────────┐
│ run_id=…  iter=3  modal=codex  status=running  elapsed=00:12:03      │
│ Cooldowns: gemini 320s (rate limit)                                  │
│                                                                      │
│ Tabs: [Timeline] [Modal Output] [Verifiers] [Git] [Changelog]        │
│                                                                      │
│ Timeline                                                            │
│  - iter 3 start (modal=codex)                                        │
│  - verifier tests: exit=1                                            │
│                                                                      │
│ Controls: [Pause] [Cancel] [Skip modal] [Open diff]                  │
└──────────────────────────────────────────────────────────────────────┘
```

#### Screen 6: Completion Summary

Purpose: show what happened and what to do next.

Displayed:
- completion status (promise + verifiers ok)
- link to `.ralf/runs/<run-id>/`
- git diff summary and branch name
- per-modal changelog links
- suggested next actions: “open PR”, “run full test suite”, “start new spec thread”

### Spec Studio (Phase A)

Primary UI elements:
- Left: “Chat” transcript (multi-turn).
- Right: “Spec Draft” viewer/editor (structured sections).
- Bottom: input box with:
  - model selector (available modals),
  - “Send”, “Review round”, “Finalize prompt”.

Key behaviors:
- Maintains conversation transcript in `.ralf/spec/threads/<id>.jsonl`.
- Allows switching the active modal mid-conversation.
- “Review round” runs a one-shot review prompt through *other available modals* and summarizes:
  - missing acceptance criteria,
  - missing verifiers,
  - ambiguous requirements,
  - risky areas.
- “Finalize” writes:
  - `PROMPT.md` (stable prompt) at repo root,
  - optionally `SPEC.md` in repo root or `.ralf/spec/SPEC.md` depending on user choice.

Prompt format requirements (final):
- must contain acceptance criteria,
- must include exact promise tag,
- must specify verification commands.

### Loop Runner (Phase B)

Primary UI elements:
- Top bar: run_id, elapsed time, iteration, current modal, status.
- Tabs:
  - Timeline (events),
  - Modal output (per modal),
  - Verifiers (exit codes + logs),
  - Git (diffstat + file list),
  - Changelog preview.

Controls:
- Start run: choose branch, max iterations, promise text, selection mode.
- Pause/resume (future if implemented via process control).
- Cancel (writes cancel state and stops processes).
- “Skip modal” / “cooldown modal” (manual override).

### Beautiful UI requirement

The final UI should be comparable in polish to mainstream terminal TUIs:
- consistent layout and colors,
- discoverable keybindings (help overlay),
- smooth updates (no flicker),
- clear error surfaces (auth prompt, timeout, missing CLI).

## Modal detection and setup

On startup in a repo:
- detect installed modals by checking PATH for `claude`, `codex`, `gemini`,
- run a fast `--help` check to confirm the binary is callable,
- offer to write `.ralf/config.json` from presets:
  - include only available modals by default,
  - default to `round_robin` selection.

Gemini macOS OAuth prompt mitigation:
- detect if `gemini` is a node script and pin the underlying `node` path (optional flow),
- warn user if a GUI prompt may block unattended runs.

## Installation and distribution

### Primary install: release binaries + installer

- Publish prebuilt binaries for macOS/Linux (and Windows if feasible).
- Provide `install/install.sh` supporting:
  - `--version vX.Y.Z` (default latest),
  - `--prefix` (default `~/.local`),
  - checksum verification (`--sha256`),
  - `--dry-run`.

### Minimal dependencies

- `ralf` binary includes all runtime dependencies.
- External requirements:
  - git (recommended; required for best UX),
  - shell for verifiers (configurable),
  - model CLIs installed by the user (claude/codex/gemini).

## Testing & CI (requirements)

CI must run:
- unit tests for config/state parsing and modal selection logic,
- integration tests that stub modal CLIs (fixtures) and verify changelog/state outputs,
- lint/format (Rustfmt/Clippy).

The project must not require real provider access in CI.

## Roadmap (high level)

v0.1:
- Loop engine with round-robin multi-modal, cooldowns, tests+promise completion, changelogs.
- Spec Studio MVP: chat transcript + finalize PROMPT.md (no fancy review synthesis required).
- Run Dashboard MVP with logs/verifiers/git panels.

v0.2:
- Multi-modal review rounds in Spec Studio + merge tooling.
- Circuit breakers (no-diff repeats, repeated verifier failures).
- Role pipeline mode (optional).
