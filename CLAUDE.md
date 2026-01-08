# ralf Development Guidelines

## Development Workflow

Use this disciplined workflow for all feature development:

### 1. Branch
```bash
git checkout -b feat/<feature-name>
```

### 2. Spec Phase
- Create `SPEC-<feature-name>.md` with:
  - **Promise**: What this delivers
  - **Deliverables**: Files, types, functions
  - **Acceptance Criteria**: Testable checklist
  - **Non-Goals**: What's explicitly out of scope
- Get external reviews (Gemini, Codex)
- Iterate until spec is solid

### 3. Implementation Loop
- Implement according to spec
- Write tests alongside implementation
- Verify: `cargo build`, `cargo clippy`, `cargo test`
- Iterate until all acceptance criteria pass

### 4. Review Phase
- Get external reviews of implementation
- Address any issues found
- Ensure all criteria are met

### 5. Commit & Merge
```bash
git commit -m "feat(scope): description"
git checkout main
git merge feat/<feature-name> --no-ff
git branch -d feat/<feature-name>
```

## External Reviews

Use Gemini and Codex for spec/implementation reviews:

```bash
# Spec review
gemini -p "@SPEC-file.md @relevant-docs... Review this spec. Consider: 1) completeness 2) correctness 3) testability"

# Implementation review
codex exec "Review implementation in <file> against spec in <spec-file>. Any bugs or issues?"
```

## Commit Message Format

```
<type>(<scope>): <description>

<body>
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## Code Standards

- All code must pass `cargo clippy -D warnings`
- All tests must pass
- New features require tests
- Keep modules focused and small

## Architecture

See `docs/DEVELOPMENT_PLAN.md` for the overall architecture and roadmap.

See `docs/state-machine.md` for the thread state machine design.
