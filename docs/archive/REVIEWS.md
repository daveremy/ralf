# External Reviews of ralf Development Plan

Reviews obtained: January 8, 2025

## Gemini Review (gemini-2.5-pro)

### Executive Summary

The transition from a screen-centric TUI to a **Thread-Centric State Machine** is the correct architectural decision. It aligns the software model with the user's mental model of "tasks" and enables the critical ability to pause, resume, and audit AI work.

However, the current plan underestimates the complexity of **Context Window Management** and **File System/Git State Safety**. While the "Happy Path" is well-defined, the "Messy Path" (merge conflicts, dirty working trees, context limits) needs more rigorous handling in the Foundation phase.

### State Machine Gaps (90% complete for V1)

- **Missing "User Interrupt" transition**: No explicit way to stop a `Running` loop manually
- **Missing "Force Success" from Stuck**: If AI fails but human judges code correct, no path to proceed
- **Dirty State Entry**: No handling for starting a thread with uncommitted changes

**Recommendations:**
- Add `Stuck -> PendingReview` (User Override)
- Add `Running -> Stuck` (User Interrupt)

### High Risks

1. **File System State & Git**
   - Risk: Half-baked filesystem state on failure/abandon
   - Recommendation: Enforce `git stash` or temporary branch before `Running`

2. **Context Window Exhaustion**
   - Risk: In a stuck loop with 10 iterations, context explodes
   - Recommendation: Define context strategy (sliding window, summarization)

3. **Manual Assist Complexity**
   - Risk: AI's internal model becomes stale after human edits
   - Recommendation: Re-read relevant files upon re-entry to `Running`

### Prioritization Recommendations

1. F1-F4 (Foundation): Keep as is
2. **Move Up: Git Safety Layer** - Before Stuck State, implement safe sandbox
3. **Move Down: Commit Preparation** - Users can `git commit` manually
4. **Move Down: Assessment** - Gold-plating for V1

### Answers to Questions

1. **Backward transitions**: Acceptable for V1. Clean break is safer than salvaging
2. **Multi-thread**: **Strictly enforce single-thread for V1** - Block, don't just warn
3. **Quick mode**: Design for it, but defer implementation
4. **Assessment**: Cut from V1
5. **Git integration**: **ralf MUST manage branches** - Can't overwrite main branch

### Final Verdict

> Focus heavily on **Failure Recovery** (Context limits, Git rollbacks, Process interruption) during the Foundation phase, or the tool will be too fragile to trust with real codebases.

---

## Codex Review (gpt-5.2)

### High-Level Take

The thread-centric pivot is the right direction, but the current state machine + plan have several inconsistencies that will create "can't recover safely" situations unless you address git/workspace isolation, post-approval changes, and quick-mode semantics.

### State Machine Issues

1. **Abandon from multiple states not implemented**: Diagram claims it but only allows `Drafting`/`Stuck` → `Abandoned`. Need from *any* non-terminal state.

2. **Manual-assist flow missing from diagram**: Plan mentions "Manual Assist → Running" but no `Stuck -> Running` transition in diagram.

3. **Quick mode contradicts human checkpoints claim**:
   - Quick mode jumps `Running -> Approved`, skipping `Verifying` and `PendingReview`
   - Also states "Approve Review always requires human approval"
   - Plan says quick mode "Auto-approves if tests pass"
   - **These can't all be true**
   - Recommendation: Never auto-transition into `Approved`; auto-advance to `PendingReview` with "recommended approve" affordance

4. **Polishing after approval is risky**: `Approved -> Polishing` allows code changes after human approved correctness. Either:
   - Move polishing *before* `PendingReview`, or
   - Require `Polishing -> PendingReview` before `ReadyToCommit`

5. **Missing "reopen spec" path**: No `Finalized -> Drafting` escape hatch

6. **Missing "reconfigure mid-stream" path**: Once in `Running`, only exits are `Verifying` or `Stuck`. Need user-driven "pause/reconfigure"

7. **Run lifecycle mismatch**: Current runner treats "max iterations reached" as `Completed`, but state machine treats exhaustion as `Stuck`

### Risks & Blind Spots

1. **Unsafe backward transitions**: `PendingReview -> Drafting` is only safe with rollback mechanism. Plan doesn't specify what happens to working tree. **Biggest practical risk.**

2. **Git assumptions undecided**: Diff view depends on git state, `Done { commit_sha }` assumes git exists, but branch strategy is "open question"

3. **State model duplication**: `Assessing { feedback }` AND `assessment_feedback: Option<String>` creates impossible states risk

4. **Scalability**: `messages: Vec<ChatMessage>` in Thread plus `transcript.json` in file layout - confusion about what lives where

5. **Crash consistency**: No mention of atomic writes, partial file recovery, or schema versioning

6. **Missing preflight**: No checks before running (clean tree, tools available, etc.)

### Prioritization Recommendations

1. **Move git/workspace safety into Foundation** - Implement "baseline capture + restore" early
2. **Fix "Approved then Polishing" ordering** - Design flaw that will keep infecting choices
3. **Quick mode as preset, not different workflow** - Same states, fewer stops, more defaults
4. **Keep v1 Thread model minimal** - Identity, phase, spec pointer, run pointer, summary only

### Answers to Questions

1. **Backward transitions**: Not too disruptive IF you have rollback mechanism. Add "reason + required cleanup action" to transition.

2. **Multi-thread**: Defer true multi-thread. Allow multiple saved threads but enforce one active `Running` thread.

3. **Quick mode**: Consider from start as configuration preset, but don't design separate transitions that bypass safety gates.

4. **Assessment**: Valuable but easy to overbuild. Treat as optional "spec lint" tool, not core phase.

5. **Git integration**: ralf should manage minimal local git safety envelope (create thread branch or capture base commit). Keep scope small: no pushing, no PRs.

---

## Consensus Points (Both Reviews Agree)

### Critical Issues

1. **Git/Workspace Safety Must Be Foundational**
   - Both: Can't have backward transitions without rollback story
   - Both: ralf must manage some git safety (branches or stash)
   - Both: Resolve this before building UI that depends on it

2. **Quick Mode Has Contradictions**
   - Both: Current design contradicts "human checkpoint" claims
   - Both: Don't auto-approve; at most auto-advance to review
   - Gemini: Design for it, defer implementation
   - Codex: Implement as preset, not different workflow

3. **Polishing After Approval Is Risky**
   - Both: Code changes after approval without re-review is dangerous
   - Solution: Move polish before review, or require re-review after polish

4. **Single Thread for V1**
   - Gemini: Strictly enforce (block, don't warn)
   - Codex: Defer true multi-thread

5. **Assessment Is Low Priority**
   - Gemini: Cut from V1
   - Codex: Optional "spec lint", not core phase

6. **State Model Should Be Minimal**
   - Both: Avoid duplication (phase enum vs fields)
   - Both: Separate large artifacts from thread state

### Missing From State Machine

| Missing Item | Gemini | Codex |
|--------------|--------|-------|
| User interrupt from Running | ✓ | ✓ |
| Force success from Stuck | ✓ | - |
| Manual assist (Stuck → Running) | - | ✓ |
| Abandon from any state | - | ✓ |
| Finalized → Drafting | - | ✓ |
| Pause/reconfigure from Running | - | ✓ |
| Preflight/preflight failure | ✓ | ✓ |

---

## Action Items (Prioritized)

### Must Fix Before Foundation

1. **Resolve git strategy**: Thread = branch, or at minimum capture baseline + provide restore
2. **Fix polishing order**: Move before review or require re-review
3. **Clarify quick mode**: Same states, fewer stops - not bypass edges
4. **Add missing transitions**: Abandon from anywhere, user interrupt, manual assist

### Foundation Phase Additions

5. **Add Preflight state**: Check clean tree, git repo, tools, parseable spec before Running
6. **Implement baseline capture**: Before entering Running, capture git state for rollback
7. **Define context strategy**: How to handle long conversations/many iterations
8. **Atomic writes + schema versioning**: For crash recovery

### Simplifications

9. **Remove state model duplication**: Phase enum is source of truth, not separate fields
10. **Separate large artifacts**: `thread.json` is index only; transcripts/runs in separate files
11. **Block (not warn) multi-thread Running**: Single active thread for V1
12. **Defer Assessment**: Optional lint tool, not core phase
