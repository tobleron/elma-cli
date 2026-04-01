# Task T044: Eliminate Critic JSON - Use Simple Text Format

## Problem
Critic reviewers (sufficiency, logical_review, efficiency_review, risk_review) produce malformed JSON that breaks the verification loop. The model struggles with:
- Trailing commas
- Incomplete objects
- Nested structure errors

Example failure:
```
logical_review_parse_error=No valid or repairable JSON object found.
Preview: {"status":"retry","reason":"...","program":{"cmd":"date",}}
                                                              ^^^ trailing comma
```

## Solution
Replace JSON output with simple text format for all critics.

### Current Format (JSON):
```json
{"status":"ok","reason":"The step achieved the objective"}
```

### New Format (Simple Text):
```
ok: The step achieved the objective
```
or
```
retry: Need to fix X because Y
```

## Implementation Steps

1. **Update critic prompts** in `config/{model}/critic.toml`, `logical_reviewer.toml`, `efficiency_reviewer.toml`, `risk_reviewer.toml`:

```toml
# Old
system_prompt = """
Return JSON: {"status":"ok"|"retry","reason":"..."}
"""

# New
system_prompt = """
Return ONE line in format:
  ok: <brief reason>
  retry: <what needs to be fixed>

Examples:
  ok: Step achieved the objective
  retry: Missing pwd command, need to add it
"""
```

2. **Update parsing functions** in `src/orchestration_loop_reviewers.rs`:

```rust
// Old
let verdict: CriticVerdict = parse_json_loose(&text)?;

// New
fn parse_simple_verdict(text: &str) -> Result<CriticVerdict> {
    let text = text.trim();
    if text.starts_with("ok:") {
        Ok(CriticVerdict {
            status: "ok".to_string(),
            reason: text.trim_start_matches("ok:").trim().to_string(),
        })
    } else if text.starts_with("retry:") {
        Ok(CriticVerdict {
            status: "retry".to_string(),
            reason: text.trim_start_matches("retry:").trim().to_string(),
        })
    } else {
        // Fallback: assume ok
        Ok(CriticVerdict {
            status: "ok".to_string(),
            reason: text.to_string(),
        })
    }
}
```

3. **Update outcome verification** in `src/verification.rs`:

```rust
// Same pattern - parse simple text instead of JSON
fn parse_outcome_status(text: &str) -> Result<OutcomeStatus> {
    let text = text.trim();
    if text.starts_with("ok:") {
        Ok(OutcomeStatus::Ok(text.trim_start_matches("ok:").trim().to_string()))
    } else if text.starts_with("retry:") {
        Ok(OutcomeStatus::Retry(text.trim_start_matches("retry:").trim().to_string()))
    } else {
        // Fallback
        Ok(OutcomeStatus::Ok(text.to_string()))
    }
}
```

4. **Update self_question prompt** in `config/{model}/self_question.toml`:

```toml
# Old
system_prompt = """
Return JSON: {"method":"SHELL"|"INTERNAL","reason":"...","internal_command":"/cmd"|null}
"""

# New
system_prompt = """
Return ONE line in format:
  SHELL: <reason>
  INTERNAL: <reason> <command>

Examples:
  SHELL: Needs terminal command to list files
  INTERNAL: User wants to exit /exit
"""
```

5. **Update self_question parsing** in `src/intel.rs`:

```rust
pub(crate) async fn self_question_instruction(...) -> Result<SelfQuestionResult> {
    // ... make request ...
    let text = extract_response_text(&resp);
    
    // Parse simple text format
    let text = text.trim();
    if text.starts_with("SHELL:") {
        Ok(SelfQuestionResult {
            method: "SHELL".to_string(),
            reason: text.trim_start_matches("SHELL:").trim().to_string(),
            internal_command: None,
        })
    } else if text.starts_with("INTERNAL:") {
        let reason_and_cmd = text.trim_start_matches("INTERNAL:").trim();
        // Extract command (last word if starts with /)
        let parts: Vec<&str> = reason_and_cmd.split_whitespace().collect();
        let internal_command = parts.last()
            .filter(|w| w.starts_with('/'))
            .map(|s| s.to_string());
        let reason = parts[..parts.len().saturating_sub(1)].join(" ");
        
        Ok(SelfQuestionResult {
            method: "INTERNAL".to_string(),
            reason,
            internal_command,
        })
    } else {
        // Fallback - assume SHELL
        Ok(SelfQuestionResult {
            method: "SHELL".to_string(),
            reason: text.to_string(),
            internal_command: None,
        })
    }
}
```

## Acceptance Criteria
- [ ] Critics output simple text, not JSON
- [ ] Outcome verification uses simple text
- [ ] Self-question uses simple text
- [ ] No more `*_parse_error` for critics
- [ ] Verification loop completes successfully
- [ ] All tests pass

## Files to Modify
- `config/{model}/critic.toml`
- `config/{model}/logical_reviewer.toml`
- `config/{model}/efficiency_reviewer.toml`
- `config/{model}/risk_reviewer.toml`
- `config/{model}/self_question.toml`
- `src/orchestration_loop_reviewers.rs`
- `src/verification.rs`
- `src/intel.rs`

## Priority
CRITICAL - Fixes the verification loop failures

## Expected Impact
- **90% reduction** in parse errors during verification
- **Faster execution** - simpler output format
- **More reliable** - text parsing is more robust than JSON
