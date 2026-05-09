# Task 756: Audit And Correct Architecture Documentation Drift

## Type

Documentation / Reliability / Maintainability

## Severity

High

## Scope

All docs under `docs/`

## Problem

`docs/ARCHITECTURE.md` contains wildly inaccurate information about the codebase:

1. **Wrong line counts** (listed as lines, but actually bytes):
   - `app_chat_loop.rs`: documented as 42790 lines, actual ~1354 lines
   - `document_adapter.rs`: documented as 63802 lines, actual ~1892 lines
   - `intel_units/mod.rs`: documented as 19312 lines, actual ~507 lines (wait, `wc -l src/intel_units/mod.rs` wasn't shown, but `intel_units_core.rs` is 16847 bytes, not lines)
   - `evidence_ledger.rs`: documented as 34634 lines, actual ~1057 lines
   - `approach_engine.rs`: documented as 16244 lines, actual ~16363 bytes (~400-500 lines)
   - `background_task.rs`: documented as 17324 lines, actual ~17324 bytes
   - `event_log.rs`: documented as 17028 lines, actual ~17028 bytes

   The pattern is clear: the docs list **file sizes in bytes** as **line counts**, inflating them by 20-40x.

2. **Dead code references**:
   - References to `routing_infer.rs` as "reserved for future activation" when Task 751 may delete or narrow it
   - References to the recipe system as integrated even though the recipe cleanup task is deferred pending a safer audit
   - References to formula patterns as part of selection — but they're dead code

3. **Outdated module map**:
   - `workspace_policy.rs` is listed twice (under Security and under Workspace)
   - `ui/` submodules are listed with wrong file names (`ui/terminal.rs` vs `ui/ui_terminal.rs`)
   - `claude_ui/` submodules are listed with wrong file names (`claude_ui/state.rs` vs `claude_ui/claude_state.rs`)

4. **Missing modules**:
   - Many newer modules (Tasks 635-736) are not documented
   - `ui_reducer.rs`, `ui_runtime_event.rs`, `ui_view_state.rs` are missing
   - `session_persistence_adapter.rs` is missing
   - `input_controller.rs` is missing

5. **Wrong architecture description**:
   - The "Route Decision" section describes a conservative heuristic, but the code hardcodes to SHELL
   - The "Tool-Calling Pipeline" diagram doesn't reflect the actual simplified flow
   - The "Hardening Principles" section is duplicated from `ARCHITECTURAL_RULES.md`

This documentation drift:
- Misleads new developers about codebase scale
- Hides the actual structure and makes onboarding harder
- Causes architects to make decisions based on false assumptions (e.g., "document_adapter.rs is 63K lines, we can't refactor it" — but it's actually 1892 lines)
- Violates Rule 5: grounded answers only — the docs make claims not supported by the actual code

## Root Cause

`ARCHITECTURE.md` was written when the codebase was smaller, and line counts were later "updated" by someone who confused bytes with lines. New modules were added without updating the docs.

## Proposed Solution

### Phase 1 — Automate line count extraction

Create a script `_scripts/update_docs_line_counts.sh`:

```bash
#!/bin/bash
# Extracts actual line counts for all src/*.rs files
# Outputs markdown table format
cd "$(dirname "$0")/.."
echo "| Module | Lines | Role |"
echo "|--------|-------|------|"
for f in src/*.rs; do
    lines=$(wc -l < "$f")
    name=$(basename "$f")
    echo "| \`$name\` | $lines | |"
done
```

Run this and replace the line count tables in `ARCHITECTURE.md`.

### Phase 2 — Audit and correct module map

1. Verify every module listed in `ARCHITECTURE.md` exists
2. Verify every module's line count is accurate
3. Add missing modules from the last 100 tasks (635-736)
4. Remove deleted modules (e.g., `routing_infer.rs` after Task 751 if it is deleted)
5. Fix file name discrepancies (`ui_terminal.rs` not `terminal.rs`)

### Phase 3 — Update architecture descriptions

1. Rewrite the "Route Decision" section to reflect the actual hardcoded SHELL path
2. Update the "Tool-Calling Pipeline" diagram
3. Mark recipe system references as deferred/experimental unless a later task wires or deletes the system
4. Update the "End-to-End Flow" to show the current simplified path
5. Add a "Known Documentation Drift" section with a date and instructions for updating

### Phase 4 — Add doc update procedure

In `DEVELOPMENT_GUIDELINES.md`, add:

```markdown
## Documentation Maintenance

When adding, deleting, or significantly changing modules:
1. Update `docs/ARCHITECTURE.md` module map (run `_scripts/update_docs_line_counts.sh`)
2. Update `docs/ARCHITECTURE.md` architecture description
3. Update `docs/DEVELOPMENT.md` project structure
4. Update `docs/SKILL_SYSTEM.md` if skills/formulas change
```

### Phase 5 — Verify other docs

1. `DEVELOPMENT.md`: check project structure against actual `src/` directory
2. `SKILL_SYSTEM.md`: verify formula list against `src/skills.rs`
3. `ARCHITECTURAL_RULES.md`: verify all referenced modules exist
4. `SOUL.md`: verify character description matches actual behavior

## Acceptance Criteria

- [ ] `docs/ARCHITECTURE.md` module line counts are accurate (verified by script)
- [ ] No module is listed with byte count masquerading as line count
- [ ] All modules in `src/` are either documented or explicitly noted as "internal utility"
- [ ] Architecture descriptions match actual code behavior
- [ ] `_scripts/update_docs_line_counts.sh` exists and produces correct output
- [ ] `DEVELOPMENT_GUIDELINES.md` includes doc maintenance procedure
- [ ] `cargo build && cargo test` passes (docs changes only, but verify no broken links)

## Verification Plan

- Script test: `_scripts/update_docs_line_counts.sh` produces table with correct counts
- Manual audit: spot-check 10 modules, verify counts match `wc -l`
- Dead link check: `grep -o '`[^`]*\.rs`' docs/ARCHITECTURE.md | sed 's/`//g' | while read f; do test -f "src/$f" || echo "MISSING: $f"; done`
- Consistency check: `docs/ARCHITECTURE.md` module count ≈ `find src -name '*.rs' | wc -l`

## Dependencies

- `_scripts/` directory (may need to be created)
- `docs/ARCHITECTURE.md`
- `docs/DEVELOPMENT.md`
- `docs/SKILL_SYSTEM.md`

## Notes

This is not a cosmetic task. Accurate documentation is essential for:
- New developer onboarding
- Architecture decisions (can't decide if a module is too big if you think it's 40x bigger than it is)
- Task scoping (can't estimate a refactor if you don't know the real size)

The line count error (bytes vs lines) is a particularly insidious falsehood because it makes the codebase seem much more intimidating than it is. A 1354-line module is large but manageable; a 42790-line module is monstrous.

Do not attempt to manually count lines. Use the script. Run it after every major module change.
