# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in ralf, please report it responsibly:

1. **Do not** open a public issue
2. Email the maintainers directly with details
3. Include steps to reproduce the vulnerability
4. Allow reasonable time for a fix before public disclosure

## Security Considerations

ralf executes external CLI tools (claude, codex, gemini) and runs user-defined
verification commands. Users should:

- Only run ralf in trusted repositories
- Review prompts and verification commands before execution
- Be aware that model outputs may contain arbitrary code
- Keep model CLIs and ralf updated to latest versions

## Secrets and Credentials

ralf is designed to avoid exposing secrets:

- Environment variables are not logged
- Obvious API key patterns are redacted if surfaced
- All artifacts are written to `.ralf/` which should be gitignored

If you believe secrets have been exposed through ralf, please report it
immediately following the process above.
