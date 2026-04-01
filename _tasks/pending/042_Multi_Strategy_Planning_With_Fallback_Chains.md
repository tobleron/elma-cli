# Task 042: Multi-Strategy Planning with Fallback Chains

## Context
Elma Philosophy states: *"prioritizes accuracy and reliability over speed"* and *"flexibility to improvise solutions that rigid rule-based systems would miss"*

Currently, Elma has:
- Single retry loop with temperature escalation (orchestration_retry.rs)
- JSON repair pipeline (routing_parse.rs)
- Self-questioning for method selection (SHELL vs INTERNAL)

But Elma CANNOT:
- Plan multiple alternative strategies upfront
- Execute fallback chains (try A, if fails try B, then C)
- Learn which strategies work best for which task types
- Adapt mid-execution when a strategy is failing

## Problem
Without multi-strategy planning, Elma:
- Commits to a single approach even when it's failing
- Wastes retries on variations of the same failing strategy
- Cannot pivot to alternative approaches proactively
- Misses opportunities for more reliable execution

## Objective
Implement multi-strategy planning with explicit fallback chains that try alternative approaches when the primary strategy fails.

## Implementation Steps

1. **Create strategy module** `src/strategy.rs`:
   ```rust
   pub enum ExecutionStrategy {
       Direct,           // Execute immediately
       InspectFirst,     // Gather evidence, then act
       PlanThenExecute,  // Create plan, then execute
       SafeMode,         // Dry-run first, then execute
       Incremental,      // Small steps with verification
       Delegated,        // Use specialized tool/service
   }

   pub struct StrategyChain {
       pub primary: ExecutionStrategy,
       pub fallbacks: Vec<ExecutionStrategy>,
       pub current_attempt: usize,
   }

   impl StrategyChain {
       pub fn next_strategy(&mut self) -> Option<ExecutionStrategy>;
       pub fn record_failure(&mut self, error: &str);
       pub fn record_success(&mut self);
   }
   ```

2. **Implement strategy selection logic**:
   ```rust
   pub async fn select_strategy_chain(
       task: &str,
       complexity: &ComplexityAssessment,
       risk: RiskLevel,
       memories: &[FormulaMemoryRecord],
   ) -> Result<StrategyChain> {
       // High risk → SafeMode first
       // Complex → InspectFirst or PlanThenExecute
       // Simple → Direct
       // Past failures → avoid those strategies
   }
   ```

3. **Create strategy-specific prompts**:
   ```toml
   # strategy_direct.toml
   system_prompt = """
   Execute this task directly with minimal overhead.
   Assume the straightforward approach will work.
   If you encounter obstacles, report them for fallback.
   """

   # strategy_inspect_first.toml
   system_prompt = """
   Before executing, gather evidence about:
   - Current state of relevant files/directories
   - Potential obstacles or conflicts
   - Required preconditions
   
   Then propose an execution plan based on findings.
   """

   # strategy_safe_mode.toml
   system_prompt = """
   Execute this task in safe/dry-run mode first:
   - Use --dry-run flags where available
   - Preview changes before making them
   - Verify preconditions
   
   After dry-run succeeds, execute for real.
   """
   ```

4. **Integrate with orchestration loop**:
   ```rust
   // In orchestration_loop.rs or new orchestration_strategy.rs
   pub async fn execute_with_strategy_chain(
       task: &str,
       mut chain: StrategyChain,
       context: &ExecutionContext,
   ) -> Result<Program> {
       while let Some(strategy) = chain.next_strategy() {
           trace(&args, &format!("trying strategy: {:?}", strategy));
           
           match execute_with_strategy(task, strategy, context).await {
               Ok(program) => {
                   chain.record_success();
                   return Ok(program);
               }
               Err(error) => {
                   chain.record_failure(&error);
                   trace_verbose(&format!("strategy failed: {}", error));
                   // Continue to next fallback strategy
               }
           }
       }
       
       // All strategies exhausted
       Err(anyhow::anyhow!("All strategies exhausted"));
   }
   ```

5. **Add strategy-aware retry logic**:
   ```rust
   // Replace or extend orchestrate_with_retries
   pub async fn orchestrate_with_strategy_fallback(
       args: &Args,
       client: &reqwest::Client,
       // ... other params
       strategy_chain: StrategyChain,
   ) -> Result<AutonomousLoopOutcome> {
       // Instead of just retrying with higher temperature,
       // switch to fallback strategy
   }
   ```

6. **Log strategy effectiveness**:
   ```rust
   pub struct StrategyLog {
       pub timestamp: u64,
       pub task_type: String,
       pub strategy: String,
       pub attempt_number: usize,
       pub success: bool,
       pub error_if_failed: Option<String>,
       pub execution_time_ms: u64,
   }

   // Save to sessions/{session}/strategy_log.jsonl
   // Use for learning which strategies work best
   ```

7. **Add strategy recommendations to prompts**:
   ```rust
   // In orchestration_planning.rs
   let strategy_hint = match &strategy_chain.primary {
       ExecutionStrategy::Direct => "Use the most straightforward approach.",
       ExecutionStrategy::InspectFirst => "First gather evidence, then propose action.",
       ExecutionStrategy::SafeMode => "Start with dry-run/preview, then execute.",
       ExecutionStrategy::PlanThenExecute => "Create a detailed plan before acting.",
       ExecutionStrategy::Incremental => "Break into small verifiable steps.",
   };
   
   let prompt = format!(
       "{}\n\nStrategy: {}\nInstruction: {}",
       base_prompt, strategy_hint, task
   );
   ```

## Acceptance Criteria
- [ ] Strategy enum defined with 6+ strategies
- [ ] StrategyChain manages primary + fallbacks
- [ ] Strategy selection based on task complexity/risk
- [ ] Strategy-specific prompts created
- [ ] Orchestration loop uses strategy chains
- [ ] Strategy effectiveness is logged
- [ ] Fallback chains work automatically on failure

## Files to Create
- `src/strategy.rs` - Strategy module
- `config/{model}/strategy_*.toml` - Strategy-specific prompts

## Files to Modify
- `src/orchestration_loop.rs` - Integrate strategy chains
- `src/orchestration_retry.rs` - Strategy-aware retries
- `src/orchestration_planning.rs` - Strategy hints in prompts
- `src/intel.rs` - Strategy selection logic

## Priority
HIGH - Core to reliability and improvisation philosophy

## Dependencies
- Self-questioning (existing)
- Task 041 (Analogy-Based Reasoning) - complementary

## Philosophy Alignment
- **"Prioritizes accuracy and reliability over speed"**
- **"Flexibility to improvise solutions that rigid rule-based systems would miss"**
- **"Adaptive reasoning"** - adapt strategy when failing

## Example Fallback Chains

### Scenario 1: File Deletion
```
Task: "delete old log files"

Strategy Chain:
1. SafeMode → find . -name '*.log' -mtime +30 (preview only)
   ↓ (user approves)
2. Direct → find . -name '*.log' -mtime +30 -delete
   ↓ (if fails: permission denied)
3. InspectFirst → check permissions, find alternative
   ↓ (discovers sudo needed)
4. Direct (with sudo) → sudo find . -name '*.log' -mtime +30 -delete
```

### Scenario 2: Code Search
```
Task: "find where fetch_ctx_max is defined"

Strategy Chain:
1. Direct → rg 'fetch_ctx_max' .
   ↓ (if no results)
2. InspectFirst → check file types, search patterns
   ↓ (discovers it's in a macro)
3. Incremental → search for related terms, trace usage
```

### Scenario 3: Dependency Update
```
Task: "update dependencies"

Strategy Chain:
1. SafeMode → cargo update --dry-run
   ↓ (if shows breaking changes)
2. PlanThenExecute → create migration plan
3. Incremental → update one dependency at a time with tests
```
