# Task 066: Limit Summarize Step Output

## Priority
**P1 - RELIABILITY CORE (Tier A)**
**Was Blocked by:** P0-1, P0-2, P0-3 — **NOW UNBLOCKED** (P0 pillars substantially complete per Task 058)

## Status
**PENDING** — Ready to start

## Renumbering Note
- **Old:** Task 015
- **New:** Task 014 (per REPRIORITIZED_ROADMAP.md)

---

Session `s_1774826560_84116000` - granite model produced program with:
```json
{
  "steps": [
    {
      "id": "s1",
      "type": "shell",
      "cmd": "find src -name '*.rs' -print0 | xargs -0 cat"
    },
    {
      "id": "s2",
      "type": "summarize",
      "instructions": "Provide a comprehensive summary..."
    }
  ]
}
```

This command would:
1. Concatenate ALL Rust source files (potentially 100k+ lines)
2. Pass to summarize step with no output limit
3. Likely exceed context window or timeout

Reflection correctly identified this as a concern:
```
"Shell step s1 lacks specific command to list files, risk of reading wrong files or none"
"Summarize step s2 does not specify how to generate the comprehensive summary"
```

But the program was still executed.

## Objective
Add safeguards to prevent `summarize` and `shell` steps from producing excessive output.

## Technical Tasks

- [ ] **Add output size limits for shell steps**
  - Default limit: 50KB raw output
  - If exceeded: truncate with "[truncated N bytes]"
  - Log warning when truncation occurs
  - Configurable via `--max-shell-output` flag

- [ ] **Add summarize step constraints**
  - Maximum input tokens: 8000 (configurable)
  - Maximum output tokens: 2000 (configurable)
  - If input exceeds limit: chunk and summarize iteratively
  - Add explicit instructions: "Summarize in 3-5 sentences per file"

- [ ] **Add preflight check for dangerous commands**
  - Detect `xargs cat`, `find | cat`, `rg . --no-restrict`
  - Warn: "This command may produce excessive output"
  - Suggest alternatives: `find | head -20`, `wc -l` first

- [ ] **Add memory gate for large operations**
  - Before summarize: check if input > threshold
  - If too large: split into batches
  - Batch summarize → aggregate summary

- [ ] **Improve reflection for output size**
  - Add check: "Does shell step risk excessive output?"
  - Add check: "Does summarize step have clear output constraints?"
  - Lower confidence score if risks detected

## Acceptance Criteria
- [ ] Shell output capped at configurable limit
- [ ] Summarize step has explicit token limits
- [ ] Preflight warns about dangerous commands
- [ ] Large file operations are batched automatically

## Verification
1. Test with `find . -type f | xargs cat` command
2. Confirm output is truncated with warning
3. Test summarize with large input
4. Confirm batching occurs for inputs > 8000 tokens
5. Verify preflight rejects/warns about dangerous patterns

## Related
- Session: `s_1774826560_84116000`
- Files: `src/execution_steps.rs`, `src/program.rs`, `src/intel.rs`
- `src/text_utils.rs` - `summarize_shell_output()` already has 12KB limit
