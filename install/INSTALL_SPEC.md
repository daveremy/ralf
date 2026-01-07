# Install & release specification

This document describes how `ralf` should be distributed as an easy-to-install open source tool.

## Goals

- New machine setup is one command.
- Default install does not require language runtimes (ship a native binary).
- Installs are versioned and reproducible.

## Recommended install flow

Pinned install (recommended):

```bash
curl -fsSL https://raw.githubusercontent.com/<OWNER>/ralf/<TAG>/install/install.sh | \
  bash -s -- --version <TAG> --sha256 <TARBALL_SHA256>
```

Convenience install (latest):

```bash
curl -fsSL https://raw.githubusercontent.com/<OWNER>/ralf/main/install/install.sh | bash
```

## Required installer features

- `--version vX.Y.Z` (default: latest)
- `--prefix DIR` (default: `~/.local`)
- `--dry-run`
- checksum verification (`--sha256`)
- print resolved version + install paths before writing

## Release artifacts

Per release tag:
- `ralf-<os>-<arch>` binaries (e.g. `ralf-darwin-arm64`, `ralf-linux-amd64`)
- `SHA256SUMS`

## Optional: TUI-only dependency mode (not recommended)

If Python+Textual is used instead of Rust, publish:
- a `pipx install ralf` path, and
- a `curl | bash` wrapper that installs into a venv under `~/.local/share/ralf/<version>`.

