# T001_Troubleshoot_JSON_Parse_Error_From_Json_Outputter

## Issue
The `json_outputter` is receiving model responses that contain:
1. Prose introduction ("Here is a valid JSON object...")
2. Markdown code fences (```)
3. The actual JSON object

Despite the system prompt explicitly stating: "Return ONLY one valid JSON object. No prose. No code fences. No backticks."

## Evidence
From session `s_1774823259_583791000/reasoning_audit.jsonl`:

```json
{"final_text":"Here is a valid JSON object that matches the target schema:\n\n```\n{\n  \"objective\": \"understand current project from workspace evidence\",...
```

The model (llama_3.2_1b_instruct_uncensored.i1_q6_k.gguf) is not following the JSON-only instruction.

## Root Cause
The `extract_first_json_object()` function in `src/routing.rs` can extract JSON from text, but it didn't handle markdown code fences properly. When models output markdown-wrapped JSON, the extraction would fail or include the fence markers.

## Fix Implemented

### Enhanced `extract_first_json_object()` with markdown stripping
Added a new helper function `strip_markdown_wrappers()` that:
1. Removes leading prose before the first code fence
2. Handles language specifiers like ```json
3. Removes trailing code fence markers

The `extract_first_json_object()` function now calls `strip_markdown_wrappers()` before extracting the JSON object.

### Files Modified
- `src/routing.rs` - Added `strip_markdown_wrappers()` and enhanced `extract_first_json_object()`
- Added 5 unit tests to verify the fix handles:
  - Markdown code fences with language specifiers
  - Plain JSON without fences
  - Prose before and after fences
  - Full JSON extraction from markdown-wrapped output

### Test Results
```
running 9 tests
test routing::tests::strip_markdown_wrappers_handles_no_fences ... ok
test routing::tests::strip_markdown_wrappers_handles_prose_before_fence ... ok
test routing::tests::strip_markdown_wrappers_removes_code_fences ... ok
test routing::tests::extract_json_from_pure_json ... ok
test routing::tests::extract_json_from_markdown_wrapped ... ok
...
test result: ok. 9 passed; 0 failed
```

## Verification Steps
1. ✅ Build succeeds: `cargo build`
2. ✅ All routing tests pass: `cargo test routing::tests`
3. ⏳ Run `cargo run -- "hi"` with the same model to verify end-to-end

## Next Steps
Test with the actual llama_3.2_1b_instruct_uncensored model to confirm the fix resolves the JSON parsing error in practice.
