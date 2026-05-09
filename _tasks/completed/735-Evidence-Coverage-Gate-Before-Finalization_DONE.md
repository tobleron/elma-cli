# Task 735: Inject Continuation When Evidence Coverage Is Below Task Scope

## Type

Bug / Model Robustness

## Severity

Critical

## Scope

System-wide

## Session Evidence

Session `s_1778153291_412614000`:

**User request:** "read all docs and give me opinion about elma-cli project."

**Trace timeline:**
```
trace: planning_source=maestro ladder_level=MasterPlan
trace: tool_loop: starting max_iterations=20 stagnation_threshold=8
trace: tool_loop: iteration 1/20 → workspace_info (250-line output, docs/ visible at lines 79-94 with 18 files listed)
trace: tool_loop: iteration 2/20 → read README.md (root-level README, NOT docs/README.md)
trace: tool_loop: iteration 3/20 → model makes no tool calls, pipeline produces answer
trace: tool_calling_pipeline: answer_len=2959 iterations=3 tool_calls=2 stopped=false
```

**Final answer excerpt:** "Overall assessment: A sophisticated local-first AI agent with strong architectural foundations but maturity gaps in evidence synthesis and tool reliability."

The model read 2 pieces of evidence (workspace_info, root README.md) and produced a 2959-character opinion. It had 20 budget iterations but used only 3. The `docs/` directory with 18 files was clearly visible in workspace_info output but the model never explored it. The model never called `ls docs/` or `read docs/*`.

**Previous fix tests (732, 734) were irrelevant because:**
- Task 732 (strategy shift on stuck read): requires 2+ duplicate reads → 0 occurred
- Task 734 (multi-file read hint): requires `ls`/`glob` before `read` → no `ls`/`glob` occurred

The fixes were designed for a model that IS exploring but gets stuck. This model never entered exploration. It made a one-shot answer from workspace_info + README.md.

**The output is factually suspect:**
The opinion mentions "Tool reliability needs work: `read` returning empty content, `exists` with malformed paths" — this is the project's self-description paraphrased, not evidence from actual source code. The model is performing literary analysis of README.md, not codebase analysis.

**Why previous sessions (s_1778151871_491015000) DID trigger the fixes:**
In the earlier session, the user said "read all docs under docs" which triggered `ls docs/` → `read docs/README.md` → stuck. In THIS session, the user said "read all docs" (without path) and the model read root README.md, which is logically a "doc" file. The model thinks it fulfilled the request.

## Problem

The model has a **completion-adequacy gap:** it stops producing tools (and thus triggers finalization) before its evidence coverage matches the task scope. The system allocates budget (20 iterations for MasterPlan) but doesn't check whether the model used that budget to achieve adequate coverage. The model self-terminates early and the system lets it.

Specifically:
1. No mechanism exists to detect that the model read only 1 of 18 available documentation files when asked to "read all docs"
2. The model stops calling tools after 2 iterations and the pipeline produces an answer — no adequacy check
3. The MasterPlan budget of 20 iterations goes unused because the model self-terminates at iteration 3

## Root Cause Hypothesis

Confirmed: Two missing system-level checks:

**A.** After the tool loop produces a final answer (model stops making tool calls), the system should verify that the model's evidence coverage is adequate for the task complexity and scope. For MasterPlan/OPEN_ENDED tasks with "read", "inspect", "review", "analyze" intent, require at least `MIN_EVIDENCE_FILES` distinct files to be read.

**B.** When the model's final answer is produced from < N tool outputs (where N depends on complexity), inject a continuation prompt asking the model to gather more evidence before finalizing. Example: *"You've only read 1 file. The task scope requires reading documentation across the project. Use ls docs/ to discover files and read at least a representative sample before providing your opinion."*

## Proposed Solution

Add an **evidence-coverage gate** between tool-loop completion and final answer delivery:

### Phase 1 — Coverage threshold

In `src/tool_loop.rs`, after the tool loop completes (model stops calling tools), check:

1. Count distinct file reads (`read` tool calls with non-empty path that succeeded)
2. Count distinct directory listings (`ls`, `glob`, `search` tools that succeeded)
3. Compare against minimum thresholds based on task complexity:
   - DIRECT: no minimum
   - INVESTIGATE: at least 1 read OR 1 listing
   - MULTISTEP: at least 2 reads OR 1 listing + 1 read
   - OPEN_ENDED/MASTERPLAN: at least 3 reads OR 1 listing + 2 reads
4. If user request contains scope keywords ("all", "every", "each", "comprehensive", "full", "deep"), add +2 to the threshold

### Phase 2 — Continuation injection

If coverage is below threshold and iterations budget remains (> 25% of max remaining):

1. Build a continuation message listing what was found so far and what's still needed
2. Reset the stop policy budget (give the model `remaining_budget - 1` more iterations)
3. Re-enter the tool loop

### Phase 3 — Maximum continuations

Cap evidence-coverage continuations at 2 (after 2 continuations, accept the answer even if coverage is low).

Implementation plan:

- `src/tool_loop.rs`: After `tool_calling_pipeline` produces answer but before sending to finalization:
  - Call `check_evidence_coverage(messages, complexity, user_request)`
  - If coverage insufficient, inject continuation prompt and re-enter loop
  - Track `coverage_continuation_count` (max 2)

- `src/evidence_ledger.rs` or new module: Add `count_distinct_files_read(messages) -> usize` and `count_directory_listings(messages) -> usize`

- `src/tool_loop.rs`: Add `extract_scope_keywords(user_request) -> bool` to detect amplified scope

## Acceptance Criteria

- [ ] User says "read all docs and give me opinion" → model reads at least 3 distinct files or 1 listing + 2 reads before being allowed to finalize.
- [ ] User says "read src/main.rs" → no extra coverage required (1 file is adequate for the scope).
- [ ] Model with 20-iteration budget and coverage at 0 → gets continuation prompt, reads files, can finalize.
- [ ] Model that exhausted budget without adequate coverage → allowed to finalize anyway (deadline > coverage).
- [ ] Continuation count capped at 2 — don't loop forever on impossible tasks.
- [ ] Replaying session `s_1778153291_412614000` shows the model getting a continuation prompt to explore docs/.

## Verification Plan

- Unit test: `count_distinct_files_read` with sample messages → returns correct count.
- Unit test: `extract_scope_keywords("read all docs")` → true. `read this file` → false.
- Integration test: mock tool loop with 1 file read + OPEN_ENDED complexity → continuation injected.
- Replay session with coverage gate enabled → model reads 3+ files.

## Dependencies

None.

## Notes

This task addresses the root cause of the session failure. Tasks 732 and 734 were necessary but insufficient — they fix navigation issues when the model IS exploring. This task fixes the more fundamental problem: the model doesn't explore at all when it can form a plausible answer from minimal data.
