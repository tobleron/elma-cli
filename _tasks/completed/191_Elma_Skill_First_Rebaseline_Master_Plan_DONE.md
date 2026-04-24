# Task 191: Elma Skill-First Rebaseline Master Plan

## Priority
**P0 - PRODUCT REBASELINE**
**Created:** 2026-04-23
**Status:** In Progress
**Supersedes:** 095 (incremental upgrade) and the strict-parity portions of 166 where they conflict with the new product direction.

## Objective
Rebase Elma from a mostly-general local coding agent into a task-first local assistant with explicit stop behavior, portable project initialization, reusable project guidance loading, bounded read-only whole-system exploration, and a branded interactive terminal identity.

The visible interface remains message-first and terminal-safe, but the acceptance target is no longer strict Claude parity. The new target is an Elma-specific UI that preserves the proven parity infrastructure where it improves safety and usability.

## Product Decisions Locked
- One request routes to either a simple path or a persisted main-task path.
- Skills are execution policies, not only prompt snippets.
- Formulas are bounded ordered sequences of 1..3 skill stages selected from a curated catalog.
- Main tasks are persisted in the session ledger by default.
- Project `_tasks` mirroring is reserved for project planning work or explicit user requests.
- `AGENTS.md` and `_tasks/TASKS.md` are first-class project guidance sources.
- `/init` must generate a portable Elma scaffold in any folder.
- The app may show a compact persistent branded header after startup.
- The bottom status/footer bar is reserved for core runtime metrics only; execution mode, queue notices, and process notifications must appear in transcript-native rows instead.
- Operational transparency is a product requirement: budgeting, routing, formula selection, stop reasons, compaction, and hidden processes should be surfaced in collapsible transcript rows.
- Skills must be backed by rich code-authoritative playbooks that define how each skill plans, budgets, scopes evidence, and decides whether to ask the user for guidance.
- Whole-system file discovery is allowed, but read-only and on-demand.
- Backlog cleanup is conservative: postpone unrelated work, delete only stale duplicates and completed drift.
- No runtime auto-installation of external helpers is allowed.

## Canonical Terms
- `skill`: a narrow operating mode with explicit instructions, preferred tools, success conditions, and stop budgets.
- `formula`: an ordered sequence of 1..3 skill stages.
- `simple request`: a request that can be handled without persisted task state.
- `main task`: a request that must create persisted session task state before execution.
- `session ledger`: per-session runtime task persistence under the session root.
- `project mirror`: optional `_tasks` mutation for project-planning work only.

## Main-Task Gate Rules
A request is a `main task` when any of these are true:
- predicted tool calls are 3 or more,
- work is multi-step with ordering/dependency,
- work benefits from resume after interruption,
- work spans multiple files or search roots,
- work needs a multi-skill formula,
- user explicitly asks for planning, tasking, auditing, or structured work tracking.

A request remains `simple` when all of these are true:
- bounded direct response or one-stage operation,
- no meaningful resume value,
- at most 2 predicted tool calls,
- no task/progress disclosure needed beyond normal transcript output.

## Track Structure
1. **Foundation**
   - 192: Stuck detection and stop policy
   - 193: Project guidance loader and `/init` scaffold
   - 194: Skill runtime, formula catalog, and predictive main-task gate
2. **Task and skill orchestration**
   - 195: Runtime task engine and dual ledger
   - 196: Repo explorer and analyzer skill
   - 197: Document intelligence skill stack
   - 198: Read-only whole-system file scout skill
   - 202: Project task steward skill and task protocol
   - 203: Extended ebook and archival format adapters
3. **User-facing integration**
   - 199: `/skills` and execution-plan UX
   - 200: Branded splash and compact header
   - 205: Transcript-native runtime telemetry and final-answer presentation
4. **Backlog normalization**
   - 201: Task inventory normalization and reprioritization
5. **Verification gate**
   - 204: Task 191 completion verification and Task 203 unblock gate

## Dependency Order
Implementation should generally proceed in this order:
1. 193 and 194
2. 195 and 192
3. 199
4. 196, 197, 198
5. 202 and 203
6. 205 and 200
7. 201 cleanup pass at the end and whenever instruction drift appears

## Active Related Tasks
- 023 remains active until transient internal guidance is verified clean under the new formula-routing flow.
- 064 remains active as the real CLI and pseudo-terminal reliability gate for this track.

## Non-Negotiable Rules
- Do not reintroduce word-trigger routing.
- Do not weaken shell safety or terminal cleanup guarantees.
- Do not allow external file mutation outside the workspace.
- Do not let task inventory drift from actual implementation state.
- Do not silently loop when a stop reason is already clear.
- Do not auto-install external helpers during live request execution.
- Do not couple generic runtime task persistence to project `_tasks` mutation.

## Required Shared Types Across This Track
- `RequestClass`
- `SkillId`
- `SkillFormulaId`
- `SkillFormula`
- `FormulaStage`
- `ExecutionPlanSelection`
- `RuntimeTaskRecord`
- `TaskMirrorPolicy`
- `StopReason`
- `StageBudget`
- `DocumentAdapter`
- `DocumentBackend`

## Verification Gate
Before this master plan can be archived:
- `cargo fmt --check`
- `cargo build`
- `cargo test`
- relevant probes including `./ui_parity_probe.sh --all`
- real `cargo run` validation of `/init`, `/skills`, execution-plan display, runtime task persistence, stop-policy behavior, and at least one cross-workspace read-only scenario
- user sign-off
