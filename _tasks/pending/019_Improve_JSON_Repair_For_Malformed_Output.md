# 019_Improve_JSON_Repair_For_Malformed_Output

## Problem
Session `s_1774826560_84116000` showed:
```
trace: orchestrator_repair_parse_error=No JSON object found
```

The granite-4.0-h-micro model sometimes produces JSON that even the repair pipeline cannot fix, causing orchestration failures.

Current repair flow in `src/ui.rs`:
1. Try to parse model output as JSON
2. If fails, call `compile_json_once()` with json_outputter profile
3. If that fails, call `legacy_repair_json_text()`
4. If all fail → error "No JSON object found"

The issue is that step 2-3 may not be aggressive enough for severely malformed output.

## Objective
Enhance JSON repair pipeline to handle more severely malformed model output.

## Technical Tasks

- [ ] **Add multi-stage repair strategy**
  - Stage 1: `extract_first_json_object()` (already exists)
  - Stage 2: `strip_markdown_wrappers()` (already exists)
  - Stage 3: Try to extract JSON from common patterns:
    - "Here is the JSON: ```json {...} ```"
    - "{...} Explanation: ..."
    - "```{...}```" (no language specifier)
  - Stage 4: Aggressive extraction - find first `{` and last `}` and validate
  - Stage 5: Call json_outputter LLM for repair (current fallback)

- [ ] **Add JSON validation before returning**
  - Ensure extracted JSON is valid with `serde_json::from_str::<Value>()`
  - If invalid, continue to next repair stage
  - Track which stage succeeded for debugging

- [ ] **Improve json_outputter prompt**
  - Add examples of malformed input → fixed output
  - Emphasize: "Extract JSON even if surrounded by prose"
  - Add few-shot examples of common failure modes

- [ ] **Add repair metrics**
  - Track how often each repair stage is needed
  - Log which models require repair most often
  - Save to session for analysis

- [ ] **Add fallback for complete repair failure**
  - If all repair stages fail, return structured error with:
    - Original model output (truncated)
    - Which stages were attempted
    - Parse errors from each stage

## Acceptance Criteria
- [ ] "No JSON object found" errors reduced by >50%
- [ ] Repair stage metrics logged to session
- [ ] json_outputter prompt includes malformed examples
- [ ] Complete repair failures include diagnostic info

## Verification
1. Test with known malformed JSON outputs from granite/llama models
2. Confirm repair succeeds in more cases
3. Review repair metrics in session logs
4. Verify fallback error messages are actionable

## Related
- Session: `s_1774826560_84116000`
- Files: `src/ui.rs`, `src/routing.rs`, `src/defaults.rs`
- T001: Robust JSON extraction (completed)
