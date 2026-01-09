# ralf

Multi-model "Ralph Wiggum" autonomous loops with a first-class TUI.

ralf is a standalone CLI + TUI that orchestrates multiple AI model CLIs (Claude, Codex, Gemini) in an autonomous loop until your task is verifiably complete.

## Features

- **Multi-model by default**: Uses all available model CLIs with round-robin selection
- **No API keys required**: Works with locally installed model CLIs
- **Two-phase workflow**:
  - **Spec Studio**: Interactive chat-based spec refinement
  - **Loop Runner**: Autonomous execution with progress tracking
- **Programmatic completion**: Tasks complete only when tests pass AND explicit completion tag is found
- **Safety-focused**: Rate-limit detection, cooldowns, and circuit breakers

## Installation

### Quick Install (Latest)

```bash
curl -fsSL https://raw.githubusercontent.com/dremy/ralf/main/install/install.sh | bash
```

### Pinned Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/dremy/ralf/v0.1.0/install/install.sh | \
  bash -s -- --version v0.1.0 --sha256 <CHECKSUM>
```

### From Source

```bash
git clone https://github.com/dremy/ralf.git
cd ralf
cargo install --path crates/ralf-cli
```

## Usage

```bash
# Open the TUI (default)
ralf

# Check available models
ralf doctor

# Initialize ralf in current repo
ralf init

# Run autonomous loop
ralf run --max-iterations 50
```

## Commands

| Command   | Description                                      |
|-----------|--------------------------------------------------|
| `ralf`    | Open the TUI (default)                          |
| `tui`     | Open the TUI (explicit)                         |
| `doctor`  | Detect models and print diagnostics             |
| `init`    | Initialize `.ralf/` directory and config        |
| `probe`   | Probe models with timeout                       |
| `run`     | Run the autonomous loop                         |
| `status`  | Print current state and cooldowns               |
| `cancel`  | Cancel the current run                          |

## Documentation

- [Specification](SPEC.md)
- [Roadmap](docs/ROADMAP.md)
- [State Machine](docs/state-machine.md)
- [Model Configuration](docs/MODELS.md)
- [Configuration Reference](docs/CONFIG.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)

## Requirements

- Git (recommended)
- At least one model CLI installed:
  - `claude` (Anthropic CLI)
  - `codex` (OpenAI Codex CLI)
  - `gemini` (Google Gemini CLI)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.
