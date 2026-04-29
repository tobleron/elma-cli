# Task 287: Evidence Ledger — Structured Evidence Tracking with Intentional Access

**Status**: Pending  
**Priority**: High  
**Depends on**: None (additive, no breaking changes)  
**Elma Philosophy**: One intel unit = one job, local-first, small-model-friendly, principle-first prompts

## Problem

When Elma executes steps, tool results are accumulated but not tracked as structured evidence. The gaps:

1. **No claim-to-evidence mapping** — final answers make claims with no link back to which step result supports them
2. **Disconnected evidence stores** — tool-calling path uses chat messages, legacy path uses `StepResult` structs. They never merge
3. **No evidence quality or staleness tracking** — a file read at step 1 isn't flagged as stale if step 3 modifies it
4. **Huge tool results bloat context** — a 50KB `find` output eats the context window when a one-line fact would suffice
5. **No intentional evidence access** — if the model needs more detail than a compact summary provides, it has no mechanism to retrieve the raw evidence

## Solution: Evidence Ledger

A lightweight, local-first evidence tracking system that:
- Immediately summarizes tool results into compact factual statements
- Stores raw evidence in separate files on disk
- Integrates compact summaries into existing chat narratives
- Provides a `read_evidence` tool so the model can intentionally retrieve raw evidence when needed

## Architecture

### Data Flow

```
Tool executes → Result produced
    ↓
EvidenceLedger.add_entry()
    ├── Raw output → evidence/<session>/e_001_raw.txt (disk)
    └── Compact summary → ledger entry (in-memory + ledger.json)
    ↓
Compact summary injected into existing chat narrative
    ↓
Model sees: "e_001: ls -la → found AGENTS.md (2.3KB), Cargo.toml (1.1KB)"
    ↓
Model decides it needs more detail
    ↓
Model calls: read_evidence(ids: ["e_001"])
    ↓
System returns: full raw content of e_001_raw.txt
    ↓
... steps continue ...
    ↓
Model drafts final answer
    ↓
[ClaimEvidenceMapper enforcement gate] — NEW
    ↓
Claims extracted from draft → each mapped to EvidenceEntry ids
    ↓
If any factual claim has NO supporting evidence → REJECT
    ↓
Model receives: "Claims X, Y have no evidence. Gather evidence or remove them."
    ↓
Model either: runs more steps to gather evidence, or revises claims
    ↓
Final answer goes to user (all factual claims grounded)
```

### File Layout

```
evidence/
  s_1777223805/
    ledger.json          ← small: ids, summaries, sources, staleness
    e_001_raw.txt        ← full shell output
    e_002_raw.txt        ← full file read
    e_003_raw.txt        ← full search results
```

## Implementation Plan

### Part 1: Core Evidence Ledger (`src/evidence_ledger.rs`)

**Data structures**:

```rust
pub(crate) struct EvidenceLedger {
    session_id: String,
    entries: Vec<EvidenceEntry>,
    claims: Vec<Claim>,
    base_dir: PathBuf,
}

pub(crate) struct EvidenceEntry {
    id: String,              // "e_001"
    source: EvidenceSource,  // which tool produced it
    timestamp: u64,
    summary: String,         // one-line factual summary (goes in narrative)
    raw_path: Option<PathBuf>, // path to raw file on disk (None if small)
    staleness: Staleness,    // FRESH, POTENTIALLY_STALE, STALE
    quality: EvidenceQuality, // DIRECT, INDIRECT, WEAK
}

pub(crate) enum EvidenceSource {
    Shell { command: String, exit_code: i32 },
    Read { path: String },
    Search { path: String, pattern: String },
    Tool { name: String, input: String },
}

pub(crate) struct Claim {
    id: String,
    statement: String,
    supported_by: Vec<String>, // EvidenceEntry ids
    contested_by: Vec<String>,
}
```

**Core API**:

```rust
impl EvidenceLedger {
    pub(crate) fn new(session_id: &str, base_dir: &PathBuf) -> Self;
    pub(crate) fn add_entry(&mut self, source: EvidenceSource, raw_output: &str) -> &EvidenceEntry;
    pub(crate) fn mark_stale(&mut self, path: &str); // marks Read entries for this path as STALE
    pub(crate) fn get_entry(&self, id: &str) -> Option<&EvidenceEntry>;
    pub(crate) fn get_raw(&self, id: &str) -> Result<String>; // reads raw file from disk
    pub(crate) fn compact_summary(&self) -> String; // narrative-ready summary of all entries
    pub(crate) fn persist(&self) -> Result<()>; // writes ledger.json
    pub(crate) fn load(session_id: &str, base_dir: &PathBuf) -> Result<Self>;
}
```

**Immediate summarization logic** (`src/evidence_summary.rs`):

- If raw output < 500 chars: summary = raw output (no truncation needed)
- If raw output >= 500 chars: generate a one-line factual summary
  - Shell: `"command → exit_code, N lines, key finding"`
  - Read: `"path → N lines, N bytes"`
  - Search: `"pattern in path → N matches"`
  - Tool: `"tool_name → success/failure, brief result"`

### Part 2: Integration with Existing Narratives

**Modify `src/intel_narrative_steps.rs`**:

The existing `build_steps_narrative()` and `build_step_results_narrative()` already produce readable text. Add an optional `EvidenceLedger` parameter:

```rust
pub(crate) fn build_steps_narrative(
    program: &Program,
    step_results: &[StepResult],
    ledger: Option<&EvidenceLedger>, // NEW: optional
) -> String
```

When a ledger is provided, each step result narrative line includes the evidence ID and compact summary:

```
Step 1 (shell): Run "ls -la"
  To: list workspace files
  Result: Command executed successfully (output: e_001: found AGENTS.md (2.3KB), Cargo.toml (1.1KB))
```

When no ledger is provided (backward compatible): existing behavior unchanged.

**Modify `src/intel_narrative.rs`**:

All narrative builders (`build_critic_narrative`, `build_sufficiency_narrative`, `build_reviewer_narrative`, etc.) accept an optional `&EvidenceLedger` parameter and pass it through to `build_steps_narrative`.

### Part 3: Evidence Access Tool (`src/tools/tool_evidence.rs`)

Add `read_evidence` as a tool the model can call:

```rust
pub(crate) fn read_evidence_tool(ledger: &EvidenceLedger, ids: Vec<String>) -> ToolResult {
    // For each id:
    //   - If entry exists and has raw file: return full content
    //   - If entry exists but no raw file (was small): return the summary (which IS the raw)
    //   - If entry doesn't exist: error
}
```

**Tool definition** (registered in `tool_registry.rs`):

```
Tool: read_evidence
Description: Retrieve full raw evidence content by evidence ID. Use when compact summaries in the narrative are insufficient.
Parameters: { "ids": ["e_001", "e_002"] }
```

### Part 4: Integration Points (additive, no breaking changes)

| File | Change |
|---|---|
| `src/main.rs` | Declare new modules: `evidence_ledger`, `evidence_summary`, `tools/tool_evidence` |
| `src/tool_loop.rs` | After each tool result, call `ledger.add_entry()` |
| `src/execution_steps.rs` | After each step in legacy path, call `ledger.add_entry()` |
| `src/orchestration_loop.rs` | Create ledger at session start, pass to narrative builders |
| `src/intel_narrative_steps.rs` | Accept optional `&EvidenceLedger`, include compact summaries |
| `src/intel_narrative.rs` | Pass ledger through all narrative builders |
| `src/tool_registry.rs` | Register `read_evidence` tool |

### Part 5: Intel Units (each does one job)

| Intel Unit | Job | Output |
|---|---|---|
| `intel_units_evidence_quality.rs` | Classify a tool result's evidence quality | DIRECT, INDIRECT, or WEAK |
| `intel_units_evidence_staleness.rs` | Determine if existing evidence is stale given new actions | FRESH, POTENTIALLY_STALE, or STALE |
| `intel_units_evidence_sufficiency.rs` | Decide if current evidence is sufficient or more gathering is needed | SUFFICIENT or NEEDS_MORE with reason |

These units are **optional** — they run when the model is strong enough or when explicitly triggered. For weak models, use heuristic fallbacks:
- Quality: shell exit_code=0 + non-empty output → DIRECT, else WEAK
- Staleness: any Write/Edit to a path → mark all Read entries for that path as STALE

### Part 6: Claim-Evidence Enforcement Gate

**Problem**: Without enforcement, the model can still make unsupported claims even when evidence exists. The ledger provides the data, but doesn't force the model to use it.

**Solution**: Add an enforcement gate between the final draft answer and delivery to the user.

#### `src/intel_units/intel_units_claim_mapper.rs`

One job: extract factual claims from the draft answer and map each to supporting `EvidenceEntry` ids.

**Input narrative** (principle-first, no examples needed):

```
DRAFT ANSWER:
<model's proposed response>

AVAILABLE EVIDENCE:
<compact ledger summary with entry IDs and factual summaries>

TASK:
Extract every factual claim from the draft answer. For each claim, identify which evidence entry (by ID) supports it. If no evidence supports a claim, mark it as UNGROUNDED.

Output contract:
{"claims": [{"statement": "...", "evidence_ids": ["e_001"], "status": "GROUNDED|UNGROUNDED"}]}
```

**What counts as a factual claim**:
- Statements about file contents, existence, or structure
- Statements about command outputs or system state
- Statements about specific values, counts, or configurations
- Statements that could be verified by re-running a tool

**What does NOT count** (skip these):
- Opinions, recommendations, or advice
- General knowledge not specific to the workspace
- Restatements of the user's question
- Procedural descriptions ("I ran X to find Y")

#### Enforcement logic in `src/orchestration_loop.rs`

```rust
pub(crate) fn enforce_evidence_grounding(
    draft: &str,
    ledger: &EvidenceLedger,
) -> EvidenceVerdict {
    // 1. Run ClaimEvidenceMapper intel unit
    let verdict = run_claim_mapper(draft, ledger.compact_summary());

    // 2. Check for ungrounded claims
    let ungrounded: Vec<_> = verdict.claims.iter()
        .filter(|c| c.status == "UNGROUNDED")
        .collect();

    if ungrounded.is_empty() {
        EvidenceVerdict::Pass
    } else {
        EvidenceVerdict::Reject {
            ungrounded_claims: ungrounded,
            feedback: format!(
                "These claims have no supporting evidence: {:?}. Either gather evidence via tools or remove them.",
                ungrounded_claims.iter().map(|c| &c.statement).collect::<Vec<_>>()
            ),
        }
    }
}
```

#### Rejection loop

```
Draft answer → enforce_evidence_grounding()
    ↓
If Reject:
    - Feedback appended to conversation: "Ungrounded claims detected..."
    - Model gets one more turn to either:
      a) Run more tool calls to gather missing evidence
      b) Revise the answer to remove unsupported claims
    - If still ungrounded after 1 retry: accept with a warning note
      ("Warning: some claims could not be verified with available evidence")
```

#### Weak model fallback

If the model is too weak for the intel unit, use a heuristic check:
- Extract sentences containing file paths, numbers, or specific identifiers
- Check if any `EvidenceEntry` summary contains matching identifiers
- If no match → flag as potentially ungrounded

This is less precise but prevents obvious hallucinations on weak models.

## Testing

- [ ] Create ledger, add 5 entries (shell, read, search), verify `ledger.json` is correct
- [ ] Add entry with 50KB output, verify raw file created and summary is one line
- [ ] Add entry with 100-char output, verify no raw file (summary = raw)
- [ ] Mark a path as stale, verify Read entries for that path become STALE
- [ ] Call `read_evidence(["e_001"])`, verify full raw content returned
- [ ] Build narrative with ledger, verify compact summaries appear in step results
- [ ] Build narrative without ledger, verify backward compatibility (no change)
- [ ] Draft answer with grounded claims → enforcement passes
- [ ] Draft answer with ungrounded claims → enforcement rejects, model revises
- [ ] Draft answer with ungrounded claims → model cannot gather evidence → accepted with warning
- [ ] Run `cargo test` — all existing tests pass
- [ ] Run `cargo build` — clean compile

## Acceptance Criteria

1. Evidence ledger creates raw files for large outputs and compact summaries for all outputs
2. Compact summaries integrate seamlessly into existing `build_steps_narrative` output
3. `read_evidence` tool is available to the model and returns raw content on demand
4. Staleness tracking works: modifying a file marks previous reads as STALE
5. Claim-evidence mapper correctly identifies grounded vs ungrounded claims
6. Enforcement gate rejects drafts with ungrounded factual claims
7. Model can revise or gather more evidence after rejection
8. Fallback heuristic catches obvious hallucinations on weak models
9. All existing tests pass (backward compatible)
10. `ledger.json` stays under 100KB even with 200+ evidence entries
11. Narrative output includes evidence IDs (e.g., `e_001: ...`) for traceability

## Non-Requirements (Out of Scope)

- Cloud sync or network evidence sharing
- Encryption of evidence files
- Evidence compression (handled by Task 285)
- Keyword-based verification detection (violates architecture rule #1)
- Changes to Maestro prompt or intel unit prompts (this is structural, not prompt-based)

## Related Files

- `src/evidence_ledger.rs` — NEW: core ledger struct and API
- `src/evidence_summary.rs` — NEW: immediate summarization logic
- `src/tools/tool_evidence.rs` — NEW: read_evidence tool
- `src/intel_units/intel_units_evidence_quality.rs` — NEW: quality classification
- `src/intel_units/intel_units_evidence_staleness.rs` — NEW: staleness detection
- `src/intel_units/intel_units_evidence_sufficiency.rs` — NEW: sufficiency check
- `src/intel_units/intel_units_claim_mapper.rs` — NEW: claim-to-evidence mapping (enforcement gate)
- `src/intel_narrative_steps.rs` — MODIFY: accept optional ledger parameter
- `src/intel_narrative.rs` — MODIFY: pass ledger through narrative builders
- `src/tool_loop.rs` — MODIFY: add entries after tool results
- `src/execution_steps.rs` — MODIFY: add entries after step execution
- `src/orchestration_loop.rs` — MODIFY: create ledger, run enforcement gate before final answer
- `src/tool_registry.rs` — MODIFY: register read_evidence tool
