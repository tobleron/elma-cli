# Task 012: Pre-Execution Reflection

## Status
PENDING

## Problem
The model can reason about contradictions (seen in "summarize AGENTS.md" session), but only after producing output. Pre-execution reflection would catch issues earlier.

## Evidence
From session reasoning_audit.jsonl:
```
"Looking at the priors:
- Route prior: CHAT (1.00) - suggests a conversational response
...
Given the priors strongly suggest reply_only pattern, I should:
- First inspect AGENTS.md (shell step)"
```

Model recognizes the contradiction but guard system blocks execution anyway.

## Goal
Add a reflection step before program execution where the model evaluates whether its program is appropriate.

## Implementation Steps

1. **Create reflection module** `src/reflection.rs`:
   ```rust
   pub struct ProgramReflection {
       pub is_confident: bool,
       pub concerns: Vec<String>,
       pub missing_steps: Vec<String>,
       pub suggested_changes: Vec<String>,
   }
   
   pub async fn reflect_on_program(
       client: &reqwest::Client,
       chat_url: &Url,
       cfg: &Profile,
       program: &Program,
       priors: &Priors,
       workspace: &WorkspaceBrief,
   ) -> Result<ProgramReflection>;
   ```

2. **Add reflection prompt template**:
   ```
   Given this program and the priors, reflect:
   1. Are you confident this will achieve the objective?
   2. What could go wrong?
   3. What's missing?
   4. Do the priors constrain you inappropriately?
   
   Program: {program_json}
   Priors: {priors_summary}
   ```

3. **Integrate into orchestration flow**:
   ```rust
   // In orchestration.rs
   let program = build_program(...).await;
   let reflection = reflect_on_program(&program, &priors).await?;
   
   if !reflection.is_confident {
       // Let model revise before execution
       program = revise_program_based_on_reflection(program, reflection).await?;
   }
   ```

4. **Add reflection logging**:
   - Log reflection results to session trace
   - Include in reasoning_audit.jsonl

5. **Make optional via config**:
   - Add `enable_reflection` flag
   - Default: true for autonomous mode

## Acceptance Criteria
- [ ] Reflection step runs before program execution
- [ ] Model can identify concerns and missing steps
- [ ] Programs can be revised based on reflection
- [ ] Reflection results are logged
- [ ] Reflection can be disabled via config

## Files to Modify
- `src/orchestration.rs` - Add reflection step
- `src/reflection.rs` - New module (create)
- `src/defaults.rs` - Add reflection prompt
- `src/types.rs` - Add reflection config

## Priority
MEDIUM - Catches issues early but adds latency

## Dependencies
- None blocking
