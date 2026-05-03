# Session Forensics & Improvement Task Generator

## Purpose

Analyze one completed `elma-cli` session with forensic rigor.

Your job is to identify failures, inconsistencies, inefficiencies, missing safeguards, and architectural improvement opportunities, then create concrete task files under:

_tasks/pending/

This task exists to close the feedback loop:

User runs elma-cli → session artifacts are produced → AI audits the session → improvement tasks are created → user reviews → fixes are implemented → user tests again.

Do not merely summarize the session. Treat the session as evidence for improving the system.

---

## Scope

Analyze ALL available artifacts for the latest or specified session, including but not limited to:

- `session.md`
- `session.json`
- `trace_debug.log`
- `reasoning_audit.jsonl`
- `terminal_transcript.txt`
- tool call logs
- evidence files
- summaries
- generated patches
- final answer
- HTTP/model/server logs if present
- any intermediate planning or routing records

If an expected artifact is missing, record that as a finding. Missing observability is itself a valid problem if it blocks diagnosis.

---

## Core Rule

Create tasks only for confirmed, evidence-backed problems.

Do not create speculative tasks unless the missing evidence itself proves an observability gap.

Every task must answer:

1. What exactly happened?
2. Why is it a problem?
3. What source files likely need to change?
4. What concrete implementation should be done?
5. How can the fix be verified?

---

# Analysis Procedure

## Phase 1 — Locate Session Inputs and Existing Tasks

Before analyzing the session:

1. Identify the session directory or latest session artifacts.
2. Identify the user's original request.
3. Identify the final answer produced by elma-cli.
4. Scan all task directories to avoid duplicates:

- `_tasks/pending/`
- `_tasks/active/`
- `_tasks/completed/`
- `_tasks/deferred/`
- `_tasks/postponed/`

5. Determine the next available task number by scanning all task filenames across all task directories.

Do not overwrite existing tasks.

---

# Phase 2 — Session Reconstruction

Reconstruct the session timeline in this order:

1. User request
2. Intent classification
3. Complexity assessment
4. Route selection
5. Planning or decomposition
6. Tool calls
7. Tool outputs
8. Evidence captured
9. Reasoning transitions between cycles
10. Final answer
11. Stop reason
12. Errors, retries, or budget exhaustion

Produce a concise timeline before creating tasks.

If the timeline cannot be reconstructed, create an observability task.

---

# Phase 3 — Failure Analysis Checklist

Analyze every category below.

For each item, mark one of:

- PASS
- FAIL
- PARTIAL
- UNKNOWN

For every FAIL or PARTIAL, cite concrete evidence from artifacts.

---

## A. Semantic Continuity

Compare the user's original request with the final answer.

Check:

- Did the final answer directly answer the user?
- Was the user's intent preserved from classification through finalization?
- Did the system silently narrow, broaden, or mutate the task?
- Did "read/analyze" degrade into "list/search"?
- Did "fix/create" degrade into "recommend"?
- Did "all files/all docs" become "some files/docs"?
- Did the final answer claim completion without enough evidence?
- Were unresolved requirements explicitly disclosed?

Flag any point where meaning changed.

---

## B. Task Classification and Routing

Check:

- Was the task type correctly classified?
- Was complexity assessed correctly?
- Was the route appropriate for the task?
- Did the system choose too shallow or too expensive a route?
- Did the route match the available tools?
- Was PLAN mode used when unnecessary?
- Was PLAN mode skipped when needed?
- Were basic file operations over-planned?
- Were high-risk operations under-planned?

Create tasks for routing bugs, classification mistakes, or missing route guards.

---

## C. Tool Call Correctness

Inspect every tool call.

For each failed or suspicious call, record:

- tool name
- arguments
- expected behavior
- actual behavior
- error message
- retry count
- whether retry was intelligent or duplicated
- whether schema knowledge was missing
- whether an alternative tool path existed

Check:

- Invalid parameter names
- Wrong paths
- Bad glob patterns
- Broken JSON/TOML/DSL payloads
- Repeated identical failed calls
- Tool result ignored by later reasoning
- Tool output too large and unsummarized
- Tool output missing from evidence context
- Tool errors not propagated into the final answer

---

## D. Budget and Iteration Efficiency

Check:

- How many cycles were used?
- Was the task's real complexity compatible with the selected budget?
- Did the static budget of 3 cycles limit success?
- Were cycles wasted on repeated discovery?
- Did the agent forget facts from earlier cycles?
- Did it re-open or re-search the same material unnecessarily?
- Did it stop before the minimum viable answer was reached?
- Was there a better deterministic path?

Create tasks for:

- dynamic iteration budgeting
- duplicate work suppression
- cross-cycle memory/evidence carryover
- budget-aware finalization
- early failure detection

---

## E. Evidence and Context Hygiene

Check:

- Was every important tool result captured as evidence?
- Did later cycles receive earlier evidence?
- Were summaries accurate and lossless enough?
- Were citations or references tied to real artifacts?
- Did the model rely on uncaptured context?
- Did the model need schema information it did not have?
- Did evidence files contain enough detail to debug later?
- Was there a mismatch between evidence and final claims?

Create tasks for:

- missing evidence capture
- evidence summarization drift
- weak context assembly
- missing schema injection
- bad evidence indexing
- lack of source-grounded final answers

---

## F. Finalization Honesty

Check:

- Was the true stop reason surfaced?
- Did finalization distinguish completed vs incomplete work?
- Did the summary fabricate success?
- Did the answer hide tool failures?
- Did it say "done" when no successful write/read/tool call happened?
- Did it explain what remains unfinished?
- Did it suggest next steps only when needed?

Create tasks for dishonest or overconfident finalization.

---

## G. Logical Coherence

Check:

- Did reasoning match gathered facts?
- Did the model contradict tool outputs?
- Did it claim files were read when they were only listed?
- Did it cite evidence that does not exist?
- Did it confuse similarly named files?
- Did it mix facts from different cycles incorrectly?
- Did it infer beyond the available evidence?

Create tasks for consistency guards or evidence-based answer validation.

---

## H. HTTP / Model / Parser Layer

Check for:

- HTTP errors
- timeouts
- retries
- server disconnects
- malformed model responses
- JSON parse failures
- schema validation failures
- unusually short model responses
- unusually long latency
- truncation
- context overflow
- max token exhaustion
- bad stop sequence behavior

For each issue, identify whether it is:

- model-specific
- transport-specific
- prompt/schema-specific
- system-wide

Create tasks for retry policy, parser hardening, model output validation, timeout handling, or better diagnostics.

---

## I. Small-Model Robustness

Because elma-cli is intended to work reliably with small local models, check:

- Did the prompt require too much implicit reasoning?
- Were instructions too long or ambiguous?
- Could deterministic preprocessing have reduced model burden?
- Could a schema, enum, DSL, grammar, or validator have prevented failure?
- Did the model need to remember too much across cycles?
- Were tool names or arguments too semantically similar?
- Was output format too fragile?
- Was error recovery delegated to the model instead of code?

Create tasks favoring deterministic, Rust-side safeguards over prompt-only fixes.

---

## J. Architecture and Codebase Improvement Opportunities

If the session reveals deeper systemic problems, create architectural tasks.

Examples:

- route planner refactor
- evidence store redesign
- task complexity scoring improvements
- tool schema registry
- deterministic retry planner
- final answer verifier
- session timeline builder
- model response sanitizer
- structured event log
- compact DSL migration
- large file refactor
- integration tests for common session failures

Architectural tasks must still be grounded in session evidence.

---

# Task Generation Rules

For each confirmed root-cause problem, create ONE task file under:

_tasks/pending/

Do not create multiple tasks for symptoms of the same root cause.

Do not create tasks already covered by existing pending/active/completed/deferred/postponed tasks.

If an existing task partially covers the issue, either:

1. reference it in the summary and do not create a duplicate, or
2. create a narrower follow-up task only for the uncovered part.

---

# Task Filename Format

Use:

_tasks/pending/NNN-short-kebab-title.md

Where `NNN` is the next available number across all task directories.

Examples:

_tasks/pending/014-preserve-evidence-across-cycles.md
_tasks/pending/015-prevent-false-completion-finalization.md

---

# Task File Template

Each task file must use this structure:

```markdown
# Task NNN: Short Title

## Type

One of:

- Bug
- Refactor
- Architecture
- Observability
- Test Coverage
- Model Robustness
- Tooling
- Finalization
- Performance

## Severity

One of:

- Critical
- High
- Medium
- Low

## Scope

One of:

- Session-specific
- System-wide
- Model-specific
- Tool-specific
- Architecture-wide

## Session Evidence

Quote or summarize the exact evidence from session artifacts.

Include:

- artifact filename
- relevant excerpt
- timestamp/cycle if available
- tool call ID if available
- final-answer excerpt if relevant

## Problem

Explain why this behavior is wrong, risky, inefficient, or misleading.

## Root Cause Hypothesis

Explain the likely root cause.

Do not overstate certainty. Use clear language:

- "Confirmed:"
- "Likely:"
- "Possible:"

## Proposed Solution

Give a concrete implementation plan.

Include:

- files to inspect
- files likely to change
- functions/modules likely involved
- new structs/enums/helpers if useful
- validation rules
- error handling changes
- logging/evidence changes
- prompt/schema changes only if truly needed

Prefer deterministic code fixes over prompt-only fixes.

## Acceptance Criteria

The task is complete only when:

- [ ] criterion 1
- [ ] criterion 2
- [ ] criterion 3

## Verification Plan

Describe how to verify the fix.

Include at least one of:

- unit test
- integration test
- replay of the failed session
- synthetic session fixture
- manual command
- log inspection

## Dependencies

List related tasks, or write:

None.

## Notes

Optional extra implementation notes.