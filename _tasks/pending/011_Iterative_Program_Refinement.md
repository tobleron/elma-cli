# Task 011: Iterative Program Refinement Loop

## Status
PENDING

## Problem
Elma produces one program and executes it without iteration. Real autonomous agents iterate: plan → execute → observe → revise.

## Evidence
From "summarize AGENTS.md" session:
- Model produces program with empty `content` field in edit step
- No mechanism to detect incomplete execution and revise
- Single-shot execution model

## Goal
Implement a refinement loop that lets the model revise its program based on execution feedback.

## Implementation Steps

1. **Create new module** `src/refinement.rs`:
   ```rust
   pub struct RefinementContext {
       pub original_objective: String,
       pub step_results: Vec<StepResult>,
       pub failures: Vec<ExecutionFailure>,
       pub iteration: u32,
   }
   
   pub async fn refine_program(
       client: &reqwest::Client,
       chat_url: &Url,
       cfg: &Profile,
       context: &RefinementContext,
   ) -> Result<Program>;
   ```

2. **Update `run_autonomous_loop`** in `src/orchestration.rs`:
   ```rust
   async fn run_autonomous_loop_with_refinement(...) -> LoopOutcome {
       let mut program = build_initial_program(...).await;
       
       for iteration in 0..max_refinement_iterations {
           let (step_results, _) = execute_program(..., &program).await?;
           
           if is_objective_achieved(&program.objective, &step_results) {
               break;
           }
           
           program = refine_program_based_on_results(
               program, 
               &step_results, 
               iteration
           ).await?;
       }
   }
   ```

3. **Add objective achievement detection**:
   ```rust
   fn is_objective_achieved(
       objective: &str,
       step_results: &[StepResult],
   ) -> bool;
   ```

4. **Add refinement prompt template**:
   - Show original objective
   - Show what was executed
   - Show what failed or is incomplete
   - Ask model to revise program

5. **Configure max iterations** (default: 3):
   - Add to args: `--max-refinement-iterations`
   - Add to profile config

## Acceptance Criteria
- [ ] Programs can be revised based on execution results
- [ ] Objective achievement is detected automatically
- [ ] Maximum refinement iterations is configurable
- [ ] Each refinement iteration is logged
- [ ] Session trace shows refinement history

## Files to Modify
- `src/orchestration.rs` - Add refinement loop
- `src/refinement.rs` - New module (create)
- `src/types.rs` - Add refinement config
- `src/defaults.rs` - Add refinement prompt template

## Priority
VERY HIGH - Core autonomous reasoning capability

## Dependencies
- None blocking
