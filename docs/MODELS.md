# Models

`ralf` uses locally installed CLIs (no API keys required by default).

Supported (v0):
- Claude CLI (`claude`)
- OpenAI Codex CLI (`codex`)
- Google Gemini CLI (`gemini`)

`ralf` maintains a per-model adapter with:
- `command_argv` (non-interactive one-shot invocation)
- `prompt_mode` (`stdin` or `arg`)
- timeout and rate-limit detection patterns

`ralf` detects available models automatically and generates a config containing only those models.

