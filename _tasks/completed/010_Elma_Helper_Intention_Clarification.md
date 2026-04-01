# Task 010: Elma Helper - Intention Clarification Layer ✅ COMPLETE

## Priority
**P0 - CRITICAL** (Replaces direct speech act classification)

## Status
**COMPLETE** - Implemented and tested

## Architecture

### Current Flow (Fragile):
```
User Input → Speech Act Classifier → Route → Execute
     ↓
  Often misclassifies ambiguous input
```

### New Flow (Robust):
```
User Input → Elma Helper → Helper Response → Classifier → Route → Execute
     ↓              ↓
  Ambiguous    Clear, action-oriented
               (easier to classify!)
```

## Why This Works Better

1. **LLMs understand LLM output better than human ambiguity**
   - Human: "ls -ltr" (could be chat, info, or action)
   - Helper: "ACTION: execute shell command ls -ltr to list files" (clearly ACTION)

2. **Helper interprets intention in agent terms**
   - Translates vague requests into actionable language
   - Makes classification trivial

3. **Fallback behavior**
   - If helper says "respond conversationally" → CHAT route
   - If helper says "execute command" → ACTION route → Reflection runs

## Implementation

### 1. Elma Helper Intel Unit ✅

**Config:** `config/*/angel_helper.toml`
- Temperature: 0.4 (deterministic)
- Output format: ACTION:, INFO:, or CHAT: prefix
- Examples included in prompt

### 2. Integration in app_chat_core.rs ✅

```rust
// Step 1: Run Elma Helper first
let helper_response = angel_helper_intention(...);

// Step 2: Parse helper's intention
let helper_intention = parse_helper_intention(&helper_response);

// Step 3: Use helper output to guide classification
let route_decision = infer_route_prior(
    ...,
    &format!("{}\n\nIntention: {}", line, helper_response),
);

// Step 4: Reflection only for ACTION
let needs_reflection = helper_intention.eq_ignore_ascii_case("ACTION");
```

### 3. Files Modified ✅
- `src/defaults_evidence.rs` - Added `default_angel_helper_config()`, `angel_helper_intention()`, `parse_helper_intention()`
- `src/app.rs` - Added `angel_helper_cfg` to LoadedProfiles
- `src/app_bootstrap_profiles.rs` - Load angel_helper.toml
- `src/app_chat_core.rs` - Run helper before classification, use for reflection trigger
- `config/*/angel_helper.toml` - Created for all models

## Acceptance Criteria
- [x] Elma helper intel unit created
- [x] Helper runs before speech act classification
- [x] Helper output guides classification
- [x] "ls -ltr" → Helper says "ACTION" → Classifier says ACTION_REQUEST
- [x] Reflection runs for ACTION helper responses
- [x] CHAT helper responses skip reflection
- [x] All 50 tests pass

## Expected Impact
- **+50% speech act accuracy** (helper clarifies ambiguity)
- **+40% reflection trigger accuracy** (only for true actions)
- **-60% classification errors** (LLM output easier to classify)
