# Task 081: Analogy-Based Reasoning Engine

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 041: Analogy-Based Reasoning Engine

## Context
Elma Philosophy states: *"flexibility to improvise solutions that rigid rule-based systems would miss"*

Currently, Elma uses:
- Classification (INSTRUCTION/INFO/CHAT)
- Self-questioning (SHELL vs INTERNAL)
- Formula selection (reply_only, inspect_summarize_reply, etc.)

But Elma CANNOT:
- Recognize "this task is like X I've seen before"
- Transfer solutions from similar past tasks
- Improvise when standard approaches fail
- Reason by analogy ("if X worked for A, try similar approach for B")

## Problem
Without analogy-based reasoning, Elma:
- Treats each task as completely novel
- Misses opportunities to reuse successful patterns
- Cannot improvise when blocked
- Lacks creative problem-solving capabilities

## Objective
Implement an analogy engine that recognizes task similarity and suggests transferred solutions.

## Implementation Steps

1. **Create analogy module** `src/analogy.rs`:
   ```rust
   pub struct TaskAnalogy {
       pub current_task: String,
       pub similar_past_task: String,
       pub similarity_score: f32,
       pub transferred_solution: String,
       pub adaptation_notes: String,
   }

   pub async fn find_analogous_tasks(
       current_objective: &str,
       memory: &FormulaMemory,
   ) -> Result<Vec<TaskAnalogy>>;

   pub async fn suggest_by_analogy(
       current_context: &ExecutionContext,
       similar_task: &TaskAnalogy,
   ) -> Result<Program>;
   ```

2. **Implement task similarity detection**:
   ```rust
   pub fn compute_task_similarity(
       task1: &str,
       task2: &str,
       context1: &WorkspaceContext,
       context2: &WorkspaceContext,
   ) -> f32 {
       // Factors:
       // - Semantic similarity of objectives
       // - Similar file types involved
       // - Similar tools/commands needed
       // - Similar complexity/risk profile
   }
   ```

3. **Create analogy prompt template**:
   ```toml
   # analogy_prompt.toml
   system_prompt = """
   You are Elma's analogy reasoning engine.
   
   Current task: {current_objective}
   
   Similar past task: {past_objective}
   Similarity: {similarity_score}%
   
   Past solution that worked:
   {past_solution}
   
   Question: How can you adapt the past solution to the current task?
   
   Consider:
   - What's the same? (transfer directly)
   - What's different? (adapt accordingly)
   - What constraints are new? (adjust approach)
   
   Return JSON:
   {
     "adapted_approach": "description of adapted solution",
     "transferred_steps": ["step 1", "step 2"],
     "modifications": ["changed X to Y because..."],
     "confidence": 0.0-1.0
   }
   """
   ```

4. **Integrate with formula selection**:
   ```rust
   // In formula_selector.rs or intel.rs
   pub async fn select_formula_with_analogy(
       line: &str,
       memories: &[FormulaMemoryRecord],
   ) -> Result<FormulaSelection> {
       // First try direct memory match
       if let Some(matched) = find_direct_memory_match(line, memories) {
           return matched;
       }
       
       // Fall back to analogy-based selection
       if let Some(analogy) = find_analogous_task(line, memories).await? {
           return suggest_formula_by_analogy(analogy);
       }
       
       // Default formula selection
       select_formula_default(line);
   }
   ```

5. **Add analogy to self-questioning flow**:
   ```rust
   // In app_chat_core.rs, after self_question_instruction
   if result.method == "SHELL" {
       // Check if analogous task suggests specific approach
       if let Some(analogy) = find_analogy_for_shell_task(line).await? {
           trace(&args, &format!("analogy_found similarity={:.2}", analogy.similarity_score));
           // Use analogy-suggested approach
       }
   }
   ```

6. **Implement fallback chain with analogy**:
   ```rust
   pub enum ProblemSolvingStrategy {
       DirectMatch,      // Exact memory match
       Analogy,          // Similar past task
       FirstPrinciples,  // Reason from scratch
       AskUser,          // Request clarification
   }

   pub async fn solve_with_fallback_chain(
       task: &str,
       context: &ExecutionContext,
   ) -> Result<Program> {
       // Try direct match
       if let Ok(program) = try_direct_memory_match(task).await {
           return Ok(program);
       }
       
       // Try analogy
       if let Ok(program) = try_analogy_based_solution(task).await {
           return Ok(program);
       }
       
       // Try first principles reasoning
       if let Ok(program) = try_first_principles(task).await {
           return Ok(program);
       }
       
       // Ask user for clarification
       ask_user_for_clarification(task);
   }
   ```

7. **Log analogy usage for learning**:
   ```rust
   pub struct AnalogyLog {
       pub timestamp: u64,
       pub current_task: String,
       pub analogous_task: String,
       pub similarity: f32,
       pub adaptation: String,
       pub success: bool,
   }

   // Save to sessions/{session}/analogy_log.jsonl
   ```

## Acceptance Criteria
- [ ] Analogy module created with similarity detection
- [ ] Similar past tasks are identified for new tasks
- [ ] Solutions are adapted from analogous tasks
- [ ] Analogy is integrated into formula selection
- [ ] Fallback chain: Direct→Analogy→FirstPrinciples→Ask
- [ ] Analogy usage is logged for learning
- [ ] Similarity threshold configurable (default 0.7)

## Files to Create
- `src/analogy.rs` - Analogy reasoning module
- `config/{model}/analogy_prompt.toml` - Analogy prompt template

## Files to Modify
- `src/intel.rs` - Add analogy functions
- `src/app_chat_core.rs` - Integrate analogy in flow
- `src/formula_selector.rs` - Use analogy for formula selection
- `src/main.rs` - Add analogy module

## Priority
HIGH - Core to Elma philosophy of improvisation

## Dependencies
- Formula memory system (existing)
- Task 036 (Long-Term Tactical Memory) - complementary

## Philosophy Alignment
- **"Flexibility to improvise solutions that rigid rule-based systems would miss"**
- **"Adaptive reasoning and improvisation rather than deterministic rules"**
- **"Dynamically leverages available knowledge"**

## Example Scenarios

### Scenario 1: Direct Analogy
```
Past task: "count Python files in src/"
Past solution: find src -name '*.py' | wc -l

Current task: "count Rust files in src/"
Analogy: 95% similar (same pattern, different extension)
Adapted: find src -name '*.rs' | wc -l
```

### Scenario 2: Cross-Domain Analogy
```
Past task: "find all TODO comments"
Past solution: rg -n 'TODO' .

Current task: "find all FIXME comments"
Analogy: 90% similar (same tool, different search term)
Adapted: rg -n 'FIXME' .
```

### Scenario 3: Structural Analogy
```
Past task: "backup config files before editing"
Past solution: cp config.toml config.toml.bak && edit...

Current task: "update dependencies safely"
Analogy: 70% similar (preserve state before change)
Adapted: cargo build && cargo update && cargo test
```
