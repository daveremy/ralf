# Configuration

Runtime config lives at `.ralf/config.json` in the target repo you’re operating on.

Key ideas:
- model selection is configuration (command argv), not code
- multi-modal selection defaults to round-robin across available modals
- completion defaults to “tests + `<promise>…</promise>`”

See `SPEC.md` for the draft schema.

