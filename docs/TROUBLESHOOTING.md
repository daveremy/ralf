# Troubleshooting

## Gemini macOS Keychain prompt

If you use Gemini OAuth login, macOS may prompt to allow `node` to access a Keychain item (e.g. `gemini-cli-workspace-oauth`).

`ralf` should detect and surface this risk during setup and provide a guided mitigation (“pin the node path”).

## CLI hangs / no output

Set a per-model timeout and run `probe` from Spec Studio/Setup to detect interactive prompts.

