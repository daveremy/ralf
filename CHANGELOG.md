# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

### Next: Milestone 1 — Engine Core
