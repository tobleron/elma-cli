# Task T207: Cheap Storage Query Strategies And Platform-Aware Command Repair

## Priority
P0

## Objective
Fix the regression where Elma answers simple storage-estimation questions with expensive per-file enumeration, weak platform awareness, and no intelligent fallback to cheaper command shapes.

## Why This Exists
The real session [`sessions/s_1776985838_345642000`](/Users/r2/elma-cli/sessions/s_1776985838_345642000) exposed a concrete failure:
- User asked: `If I want to delete all sessions under sessions that are older than 2 weeks, how much space will I save?`
- Elma first enumerated files with `find sessions -type f | head -50`
- Then attempted:
  - `find sessions -type f | while read f; do stat -f "%m %N %S" "$f"; done`
  - `find sessions -type f | while read f; do stat -f "%m %N" "$f"; done`
  - `date -d "@$(($(date +%s) - 1209600))"`
- These commands were either wrong for macOS (`date -d`) or unnecessarily expensive (`stat` per file across the whole tree).
- The stop policy then terminated with `repeated_tool_failure`.

This is not a generic model weakness. It is a missing execution strategy for a common local-filesystem task.

## Session Findings To Preserve
- The target question only required **aggregate size of old session directories/files**, not full content inspection.
- The chosen strategy expanded to thousands of file-level rows and generated multi-megabyte shell outputs.
- The commands were not adapted to BSD/macOS semantics even though the runtime knew the platform.
- The failure occurred before Elma tried a simpler aggregate approach such as directory-level `find` + `du`.

## Required Behavior
1. For requests shaped like:
   - "how much space will I save"
   - "how big is X"
   - "how much disk space do old files use"
   Elma must prefer **aggregate storage commands** over per-file enumeration.
2. The shell strategy must be **platform-aware**:
   - macOS/BSD command variants
   - GNU/Linux variants
3. Elma must be able to repair a failing command by switching strategy class, not merely tweaking the same family.
4. For retention-style questions, Elma should prefer:
   - directory-level filtering when directory mtimes are sufficient,
   - `du`/`find` aggregation,
   - count + size summaries,
   over raw per-file `stat` loops unless strictly necessary.

## Required Strategy Catalog
Add a code-authoritative strategy/playbook for storage/retention queries that includes at least:
- aggregate-by-directory strategy
- aggregate-by-file strategy
- fallback strategy when date predicate support differs by platform
- human-readable size output strategy

The playbook must explicitly answer:
- when to inspect directories only
- when to inspect files
- when to compute count + size
- when to avoid reading file contents entirely

## Concrete Acceptance Example
For the user question:
`If I want to delete all sessions under sessions that are older than 2 weeks, how much space will I save?`

Elma should converge on a cheap command family such as:
- macOS/BSD-style directory filtering + `du`
- or a bounded cross-platform fallback that computes old targets then sums their sizes

It must not start with a full per-file `stat` loop unless it has first ruled out the cheaper aggregate strategy.

## Command-Repair Requirement
When the first shell command fails because of platform syntax or tool semantics:
- do not immediately count it as part of a repeated-failure family if the next attempt changes strategy class;
- do allow a bounded "simple fallback" attempt before terminating the stage.

Examples of valid fallback classes:
- `date -d ...` fails on macOS -> switch to `date -v-14d` or epoch comparison via `perl`/`python`/`find`
- file-level `stat` loop explodes -> switch to `find ... -exec du ...` aggregate approach

## Integration Points
- `src/skills.rs`
- `src/orchestration_core.rs`
- `src/tool_loop.rs`
- `src/stop_policy.rs`
- any shell-strategy/prompt constants used for bounded execution guidance

## Non-Goals
- Do not introduce brittle keyword routing.
- Do not hardcode one literal command as the only allowed answer.
- Do not add internet/tool installation dependencies.

## Acceptance Criteria
- On macOS, Elma can answer the session-retention space question without generating megabyte-scale shell output.
- Elma uses a cheaper aggregate strategy than the one observed in `s_1776985838_345642000`.
- A platform-specific shell failure triggers one bounded fallback of a different strategy class before stop-policy termination.
- The final answer includes:
  - number of matching sessions/directories,
  - total reclaimable size,
  - a brief note that this is an estimate prior to deletion.

## Required Tests
- unit test or strategy test for macOS retention-size query planning
- unit test or strategy test for Linux retention-size query planning
- regression fixture using the exact prompt from `s_1776985838_345642000`
- verification that Elma does not emit a per-file `stat` loop as the first strategy for this query shape

