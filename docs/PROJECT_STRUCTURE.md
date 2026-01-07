# Project structure (planned)

This is a planning doc to make the repo easy to navigate once implementation begins.

## Proposed layout (Rust)

- `crates/ralf-engine/`
  - config, state, cooldowns
  - modal adapters (CLI process execution, timeouts)
  - verification runners
  - changelog writer
- `crates/ralf-tui/`
  - Spec Studio screens
  - Run Dashboard screens
  - shared widgets (tabs, log viewers)
- `crates/ralf-cli/`
  - command-line entrypoint (non-TUI mode, useful for scripting)
- `docs/` (user docs)
- `install/` (install/uninstall scripts)

## Non-Rust alternatives

If Python+Textual is chosen instead, keep the same conceptual split:
- `ralf/engine/`
- `ralf/tui/`
- `ralf/cli/`

