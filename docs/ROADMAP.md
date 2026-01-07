# Roadmap

For the detailed implementation plan, see `docs/DEV_ROADMAP.md`.

## v0.1 (MVP)

- Standalone `ralf` binary (Rust) with:
  - modal detection + guided setup
  - Spec Studio (chat + finalize `PROMPT.md`)
  - Loop Runner (multi-modal round robin, cooldowns, changelogs)
  - completion policy: required verifiers + promise tag
- Release binaries + `install/install.sh` (curl|bash) with pinned versions + checksum support.

## v0.2

- Spec Studio review rounds across all available modals.
- Better context packing (git diffs + last verifier output) with size limits.
- Circuit breakers (no-diff, repeated failures).
- Role pipeline option (builder/reviewer/doc/tester).
