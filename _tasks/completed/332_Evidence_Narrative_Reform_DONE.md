# Task 332: Evidence Narrative Reform - Natural Flow

## Status
- **State**: pending
- **Priority**: high
- **Type**: enhancement
- **Depends on**: None

## Problem Statement

The current evidence system over-complicates by:
1. Showing evidence IDs (`e_001`, `e_002`) to the model when they're mainly needed for internal tracking
2. Having `read_evidence` tool that confuses models (they call it without evidence existing)
3. Not presenting evidence in a natural, conversational flow in the tool-calling pipeline

**Current behavior (broken):**
```
user: where is epub task located?
assistant: <read_evidence ids=["e_001"]>
tool: Error: Evidence entry not found
```

The model never sees evidence IDs in tool-calling mode (`tool_loop.rs:1000-1033`), so it doesn't know evidence exists or how to use it.

## Proposed Solution

Implement natural evidence flow as described:

```
shell: rg "epub.*task" -n
→ Found: src/document_adapter.rs:1662: // TODO: Complete full EPUB extraction (Task 251)
Reflection: EPUB task (Task 251) is in src/document_adapter.rs:1662 as a placeholder.
```

### Design Principles
1. **Natural flow**: action → output → reflection (no IDs in prompt)
2. **IDs internal only**: Keep `e_001` format internally for debugging/compaction edge cases
3. **Remove `read_evidence` tool**: No longer needed if evidence is presented naturally
4. **Evidence = summary**: The "reflection" is the `summary` field from `EvidenceLedger`

## Implementation Plan

### Phase 1: Modify Tool-Calling Loop (`tool_loop.rs`)

**File**: `src/tool_loop.rs:1000-1033`

Change tool result insertion to include evidence summary:

```rust
// Current (broken): Just tool output, no evidence context
messages.push(ChatMessage {
    role: "tool".to_string(),
    content: model_content,  // Just raw output
    ...
});

// Proposed: Natural evidence flow
let evidence_summary = crate::evidence_ledger::get_session_ledger()
    .and_then(|ledger| {
        // Find evidence entry for this tool call
        ledger.entries.iter()
            .rev()
            .find(|e| /* match by tool call */)
            .map(|e| format!("\nReflection: {}", e.summary))
    })
    .unwrap_or_default();

messages.push(ChatMessage {
    role: "tool".to_string(),
    content: format!("{}\n{}", model_content, evidence_summary),
    ...
});
```

### Phase 2: Update EvidenceLedger (`evidence_ledger.rs`)

Add method to get latest evidence for a tool call:

```rust
impl EvidenceLedger {
    pub(crate) fn get_latest_summary(&self) -> Option<&str> {
        self.entries.last().map(|e| e.summary.as_str())
    }
}
```

### Phase 3: Deprecate `read_evidence` Tool

**Files**: 
- `src/tools/tool_evidence.rs` - Mark as deprecated
- `src/tool_calling.rs:86` - Remove from tool dispatch
- `src/tool_loop.rs:928-931` - Remove from exclusion list for evidence collection

Add deprecation notice:
```rust
// TODO(T332): Remove after natural evidence flow is implemented
// Use direct tool output with reflection instead
pub(crate) fn read_evidence_tool_definition() -> serde_json::Value {
    serde_json::json!({ "deprecated": true, ... })
}
```

### Phase 4: Update Prompt (`prompt_core.rs`)

Per AGENTS.md Rule 8, this requires user approval.

**Current prompt** (`src/prompt_core.rs:43-55`):
```
Tool workflow:
1. Discover capabilities with tool_search
2. Execute commands with shell, read, or search
3. Call respond when evidence answers the request
```

**Proposed change**:
```
Tool workflow:
1. Discover capabilities with tool_search
2. Execute commands with shell, read, or search
3. Each tool result includes a reflection on how it helps answer the request
4. Call respond when evidence answers the request
```

### Phase 5: Update Compaction Logic

Ensure turn summaries (`intel_units_turn_summary.rs`) work with the new format.

The compaction already uses `build_step_results_narrative()` which includes evidence summaries - this should work without changes.

## Verification

### Build & Test
```bash
cargo build
cargo test
```

### Behavioral Test
1. Start new session: `cargo run`
2. Ask: "where is epub task located?"
3. **Expected behavior**:
   - Model calls `shell` with `rg "epub.*task" -n`
   - Tool result shows: output + "Reflection: ..."
   - Model calls `respond` with answer
   - NO `read_evidence` calls

### Scenario Test
Add scenario in `tests/scenarios/`:
```gherkin
Scenario: Evidence natural flow
  Given the user asks "where is epub task located?"
  Then the model should execute shell command
  And the tool result should include a reflection
  And the model should NOT call read_evidence
  And the model should call respond with the answer
```

## Success Criteria

- [ ] Tool-calling loop shows evidence as: action → output → reflection
- [ ] Evidence IDs (`e_001`) not shown in model prompt
- [ ] `read_evidence` tool deprecated/removed
- [ ] No model confusion about evidence collection
- [ ] All existing tests pass
- [ ] New scenario test passes
- [ ] Compaction works with new format

## Notes

- Keep evidence IDs internally for `ledger.json` persistence (debugging)
- The "reflection" text comes from `summarize_tool_result()` in `evidence_summary.rs`
- This aligns with Elma's philosophy: "Semantic continuity from user intent to final answer"
