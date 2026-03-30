# Task 016: Fix Raw Output Display for Shell Commands

## Status
PENDING

## Problem
When Elma executes shell commands (e.g., "run tree command"), the raw terminal output is not shown to the user. Instead, Elma provides a COMPACT summary without the actual command output.

## Evidence
From session trace (s_1774778220_978544000):
- User: "run tree command"
- Evidence mode selector chose: COMPACT
- Expected: RAW or RAW_PLUS_COMPACT to show actual tree output
- Result: User saw explanation without raw command output

## Root Cause Analysis

The evidence mode selector in `src/orchestration_helpers.rs` (or related module) is choosing COMPACT mode when it should choose RAW or RAW_PLUS_COMPACT for direct command execution requests.

Current logic likely:
1. Checks if route is CHAT → defaults to COMPACT
2. Doesn't recognize "run X command" as requiring RAW output
3. Reply instructions may not be properly propagated to mode selector

## Implementation Steps

1. **Find evidence mode selector** - Locate the code that chooses RAW/COMPACT/RAW_PLUS_COMPACT

2. **Add command execution detection**:
   ```rust
   fn should_show_raw_output(
       user_message: &str,
       reply_instructions: &str,
       step_results: &[StepResult],
   ) -> bool {
       // Detect direct command execution requests
       let command_patterns = [
           "run tree",
           "run cargo",
           "run ls",
           "show output",
           "display output",
           "execute",
       ];
       
       // Check if user explicitly asked to see output
       user_message.contains("run") 
           || user_message.contains("show")
           || reply_instructions.contains("output")
   }
   ```

3. **Update mode selection logic**:
   ```rust
   // In evidence mode selector
   if should_show_raw_output(&user_message, &reply_instructions, &step_results) {
       return EvidenceMode::RAW_PLUS_COMPACT;  // Show output + explanation
   }
   ```

4. **Add config option** (optional):
   ```toml
   # In orchestrator.toml or new evidence_mode.toml
   prefer_raw_for_commands = true
   ```

5. **Test with various commands**:
   - "run tree"
   - "run cargo test"
   - "ls -la"
   - "show me the files"

## Acceptance Criteria
- [ ] "run tree command" shows actual tree output
- [ ] "run cargo test" shows test output
- [ ] Direct command requests use RAW or RAW_PLUS_COMPACT mode
- [ ] Explanatory requests still use COMPACT mode
- [ ] User can still request summary-only if desired

## Files to Modify
- `src/orchestration_helpers.rs` - Evidence mode selector
- `src/types.rs` - May need EvidenceMode config
- `src/defaults.rs` - Add evidence mode config template

## Priority
MEDIUM - Affects user experience but not core reasoning

## Dependencies
- None blocking
- Related to Task 013 (decoupled classifications may affect mode selection)
