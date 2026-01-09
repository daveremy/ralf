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

## External Reviews with AI Assistants

**IMPORTANT**: Always use external AI reviews for both specs and implementations. This catches issues early and provides diverse perspectives.

### Gemini CLI

Gemini has a large context window - ideal for reviewing multiple files together.

```bash
# Basic usage with file inclusion
gemini -p "@file1.rs @file2.rs <prompt>"

# Include entire directories
gemini -p "@src/ @tests/ <prompt>"

# Include all files in current directory
gemini --all_files -p "<prompt>"
```

**Spec Review:**
```bash
gemini -p "@SPEC-feature.md @docs/state-machine.md @docs/ROADMAP.md \
Review this spec for: 1) completeness 2) correctness 3) consistency with existing architecture 4) testability. \
Provide PASSED or FAILED verdict with specific issues."
```

**Implementation Review:**
```bash
gemini -p "@crates/ralf-engine/src/feature.rs @SPEC-feature.md \
Review this implementation against the spec. Check: 1) all acceptance criteria met 2) error handling 3) test coverage 4) safety. \
Provide PASSED or FAILED verdict."
```

### Codex CLI (OpenAI)

Codex has two non-interactive modes: `exec` and `review`.

#### codex exec - Headless Execution
Runs Codex non-interactively with a prompt. Can read/write files.

```bash
# Basic headless execution
codex exec "<prompt>"

# With sandbox mode (can write to workspace)
codex exec --full-auto "<prompt>"

# Output last message to file
codex exec -o output.txt "<prompt>"

# Read prompt from stdin (for long prompts)
cat prompt.txt | codex exec -
```

**Spec Review:**
```bash
codex exec "Read SPEC-feature.md and any referenced docs. \
Review for: 1) completeness 2) correctness 3) testability 4) edge cases. \
Provide PASSED or FAILED verdict with specific issues."
```

**Implementation Review:**
```bash
codex exec "Read crates/ralf-engine/src/feature.rs and SPEC-feature.md. \
Review implementation against spec. Check: 1) acceptance criteria 2) error handling 3) tests 4) safety. \
Provide PASSED or FAILED verdict."
```

#### codex review - Code Review Mode
Specialized for reviewing git changes. Reviews diffs against base branch or uncommitted changes.

```bash
# Review uncommitted changes
codex review --uncommitted

# Review changes against main branch
codex review --base main

# Review a specific commit
codex review --commit abc123

# Custom review prompt
codex review --base main "Focus on security and error handling"
```

### Review Workflow

For each spec or implementation, get reviews from BOTH Gemini and Codex:

```bash
# Run in parallel when possible

# Gemini review
gemini -p "@SPEC-f4.md @docs/state-machine.md Review this spec..."

# Codex review
codex exec "Read SPEC-f4.md and docs/state-machine.md. Review this spec..."
```

### Common Review Prompts

**Spec Review Template:**
```
Review this spec for:
1) Completeness - are all requirements captured?
2) Correctness - does the design make sense?
3) Testability - can acceptance criteria be verified?
4) Edge cases - what's missing?
5) Consistency - does it fit the existing architecture?

Provide a PASSED or FAILED verdict with specific issues.
```

**Implementation Review Template:**
```
Review this implementation against the spec. Check for:
1) Correctness - does it implement the spec correctly?
2) Safety - any security or memory safety issues?
3) Error handling - are all error cases covered?
4) Test coverage - are tests comprehensive?
5) Acceptance criteria - are all criteria met?

Provide a PASSED or FAILED verdict with specific issues.
```

### Troubleshooting

- **Codex stdin error**: Use `codex exec "<prompt>"` not piped input
- **Gemini not loading files**: Ensure `@` prefix on file paths
- **Review truncated**: Don't use `head` to limit output - get full review

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

See `docs/ROADMAP.md` for the project roadmap and `docs/state-machine.md` for the thread state machine design.
