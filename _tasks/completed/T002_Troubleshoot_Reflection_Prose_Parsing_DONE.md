# T002_Troubleshoot_Reflection_Prose_Parsing

## Issue
The reflection module failed with "No JSON object found in reflection response" error.

From session `s_1774823259_583791000/trace_debug.log`:
```
trace: reflection_failed error=No JSON object found in reflection response
💡 Retry 1/4 (temp=0.0, strategy=standard)
```

## Root Cause
The llama_3.2_1b_instruct_uncensored model is too weak to follow the JSON-only instruction for the reflection task. Instead of returning JSON, it returned numbered prose.

## Fix Implemented

### Added prose fallback parser
Enhanced `parse_reflection_response()` in `src/reflection.rs` to:
1. First try to extract and parse JSON (existing behavior)
2. If JSON extraction fails, fall back to parsing structured prose
3. Extract confidence from keywords ("very confident", "no confidence", etc.)
4. Extract numbered list items from sections like "What Could Go Wrong"
5. Return a sensible default if all parsing fails

### New Functions Added
- `parse_reflection_prose()` - Parses numbered prose responses
- `extract_section()` - Extracts content between section headers  
- `split_prose_points()` - Splits prose into individual bullet points

### Test Results
```
running 6 tests
test reflection::tests::test_parse_reflection_prose ... ok
test reflection::tests::test_parse_reflection_prose_with_percentage ... ok
...
test result: ok. 6 passed
```

All 34 tests in the project pass.

## Files Modified
- `src/reflection.rs` - Enhanced `parse_reflection_response()` with prose fallback, added 2 new tests

## Verification
1. ✅ Build succeeds: `cargo build`
2. ✅ All 34 tests pass: `cargo test`
3. ✅ Prose parser handles actual problematic response from session

## Related
- T001: Fixed JSON extraction from markdown-wrapped output (json_outputter issue)
- T002: Fixed prose parsing for reflection module (this task)

Both fixes make Elma more robust when working with weaker models that don't follow JSON-only instructions.
