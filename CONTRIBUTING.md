# Contributing to ralf

Thank you for your interest in contributing to ralf!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/ralf.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run tests: `cargo test --all`
6. Run lints: `cargo clippy --all-targets --all-features -- -D warnings`
7. Format code: `cargo fmt --all`
8. Commit your changes
9. Push to your fork and submit a pull request

## Development Setup

### Prerequisites

- Rust (stable, latest recommended)
- Git

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test --all
```

### Code Style

- Run `cargo fmt --all` before committing
- Ensure `cargo clippy --all-targets --all-features -- -D warnings` passes
- All public items should have doc comments

## Pull Request Guidelines

- Keep changes focused and atomic
- Write clear commit messages
- Add tests for new functionality
- Update documentation as needed
- Ensure CI passes before requesting review

## Reporting Issues

When reporting issues, please include:

- ralf version (`ralf --version`)
- Operating system and version
- Steps to reproduce
- Expected vs actual behavior
- Any relevant logs or error messages

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
