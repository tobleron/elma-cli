# 286 Fix Model Response Truncation In Tool Calling Pipeline

## Problem
Elma's 4B local model (Huihui-Qwen3.5-4B) sometimes generates truncated responses in the tool-calling pipeline, particularly when asked to provide proof or evidence. The model starts to respond with phrases like "Let me show you the proof..." but fails to actually execute the necessary tool calls to demonstrate the evidence, resulting in incomplete answers.

## Root Cause Analysis
From session trace logs:
1. Model response was only 133 bytes: "Let me show you the proof — I determined the timezone by checking the system's actual timezone configuration on this macOS machine."
2. System detected truncation and retried 3 times with increased max_tokens
3. Model generates statements of intent without following through with actual tool execution
4. 4B model struggles with multi-step reasoning requiring: understanding → decision → tool invocation → result presentation

## Impact
- Users receive incomplete responses that promise evidence but don't deliver it
- Trust erosion when Elma says "Let me show you proof" but shows nothing
- Wasted computational resources on retry loops
- Poor user experience for verification/request-for-evidence queries

## Solution Requirements
1. Detect when model generates intent-only responses without tool calls
2. Force actual evidence gathering when user requests proof/verification
3. Improve truncation detection for multi-part responses
4. Ensure tool-calling pipeline doesn't accept incomplete answers as final

## Implementation Plan
### Phase 1: Detection Enhancement
- [ ] Add pattern detection for "Let me show"/"I will demonstrate"/"Allow me to prove" phrases without accompanying tool calls
- [ ] Enhance `final_answer_needs_retry()` to detect intent-only responses
- [ ] Add heuristics for when model describes actions but doesn't execute them

### Phase 2: Response Validation
- [ ] Create validation step that checks if model response contains actionable evidence
- [ ] For proof requests, require actual tool output (commands executed, files read, etc.)
- [ ] Implement fallback to direct evidence gathering when intent-only detected

### Phase 3: Pipeline Improvements
- [ ] Increase max_tokens for verification/request-for-evidence queries
- [ ] Add mid-turn checkpoint for evidence gathering phases
- [ ] Modify tool_loop to detect and correct incomplete reasoning chains

### Phase 4: Testing
- [ ] Create test cases for proof/request-for-evidence scenarios
- [ ] Verify fix with timezone proof scenario from original issue
- [ ] Test with other verification requests (file contents, command outputs, etc.)

## Success Criteria
- When user asks for proof/evidence, Elma actually executes tool calls to gather it
- No more "Let me show you..." statements without accompanying evidence
- Tool-calling pipeline rejects incomplete answers and forces proper execution
- Response times remain acceptable despite additional validation

## Related Files
- src/tool_loop.rs - Main tool calling loop logic
- src/intel_units/intel_units_maestro.rs - Step generation
- src/app_chat_loop.rs - Planning and routing logic
- src/orchestration_core.rs - Tool calling pipeline
