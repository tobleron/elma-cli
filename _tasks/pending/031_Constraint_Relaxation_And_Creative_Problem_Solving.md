# ⏸️ POSTPONED

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 043: Constraint Relaxation & Creative Problem Solving

## Context
Elma Philosophy states: *"flexibility to improvise solutions that rigid rule-based systems would miss"* and *"empowering Elma to autonomously assess each situation, reason about available options, and execute workflows that genuinely address user objectives"*

Currently, Elma:
- Follows classification → self-question → execute flow
- Has fallback mechanisms for JSON parsing failures
- Uses retry loops with temperature escalation

But Elma CANNOT:
- Recognize when a task is impossible as stated
- Suggest achievable alternatives when blocked
- Relax constraints creatively ("if X is not possible, what's the closest alternative?")
- Negotiate with user about feasible objectives

## Problem
Without constraint relaxation, Elma:
- Fails completely when requirements can't be met
- Doesn't suggest partial solutions
- Can't adapt objectives to available capabilities
- Misses opportunities for creative workarounds

## Objective
Implement constraint relaxation and creative problem-solving capabilities that help Elma find alternative paths when the direct approach is blocked.

## Implementation Steps

1. **Create constraint analysis module** `src/constraint.rs`:
   ```rust
   pub struct TaskConstraints {
       pub explicit: Vec<String>,      // User-stated requirements
       pub implicit: Vec<String>,      // Assumed constraints
       pub hard: Vec<String>,          // Cannot be relaxed
       pub soft: Vec<String>,          // Can be relaxed
   }

   pub struct ConstraintRelaxation {
       pub original_constraint: String,
       pub relaxed_constraint: String,
       pub impact: String,
       pub confidence: f32,
   }

   pub async fn analyze_constraints(task: &str, context: &ExecutionContext) 
       -> Result<TaskConstraints>;

   pub async fn suggest_relaxations(constraints: &TaskConstraints) 
       -> Vec<ConstraintRelaxation>;
   ```

2. **Implement constraint detection**:
   ```rust
   pub fn extract_constraints_from_task(task: &str) -> Vec<String> {
       // Look for constraint indicators:
       // - "must", "required", "need to" → hard constraints
       // - "should", "prefer", "ideally" → soft constraints
       // - Time limits, resource limits, format requirements
   }

   pub fn classify_constraint_hardness(constraint: &str, context: &ExecutionContext) 
       -> ConstraintHardness 
   {
       // Hard: Legal requirements, data integrity, security
       // Soft: Preferences, conventions, nice-to-haves
   }
   ```

3. **Create relaxation prompt template**:
   ```toml
   # constraint_relaxation.toml
   system_prompt = """
   You are Elma's creative problem-solving assistant.
   
   Original task: {task}
   
   Blocked because: {blocking_reason}
   
   Constraints identified:
   {constraints}
   
   Your goal: Find alternative approaches that:
   1. Address the core user objective
   2. Work around the blocking issue
   3. Minimize compromise on important requirements
   
   Consider:
   - What is the TRUE user objective? (look past stated requirements)
   - Which constraints can be relaxed without losing value?
   - What alternative approaches exist?
   - What partial solutions provide value?
   
   Return JSON:
   {
     "core_objective": "the real user need",
     "alternatives": [
       {
         "approach": "description",
         "relaxed_constraints": ["constraint1", "constraint2"],
         "preserved_value": "what user still gets",
         "tradeoffs": "what user gives up",
         "confidence": 0.0-1.0
       }
     ],
     "recommendation": "which alternative to try first"
   }
   """
   ```

4. **Integrate with orchestration flow**:
   ```rust
   // In orchestration_retry.rs or new orchestration_creative.rs
   pub async fn try_creative_problem_solving(
       task: &str,
       failure_reason: &str,
       context: &ExecutionContext,
   ) -> Result<Program> {
       // Analyze why we're blocked
       let constraints = analyze_constraints(task, context).await?;
       
       // Suggest relaxations
       let relaxations = suggest_relaxations(&constraints);
       
       // Generate alternative approaches
       let alternatives = generate_alternatives(task, &relaxations).await?;
       
       // Present to user or auto-select best
       if alternatives.is_empty() {
           return Err(anyhow::anyhow!("No viable alternatives found"));
       }
       
       // Try the highest-confidence alternative
       let best = alternatives.into_iter()
           .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
           .unwrap();
       
       execute_alternative(&best).await
   }
   ```

5. **Add user negotiation for major relaxations**:
   ```rust
   pub async fn negotiate_with_user(
       original_task: &str,
       blocking_issue: &str,
       alternatives: &[AlternativeApproach],
   ) -> Result<UserDecision> {
       // Present alternatives to user
       // Let user choose or provide new constraints
       // Return updated task or cancellation
   }

   pub enum UserDecision {
       ProceedWith(AlternativeApproach),
       ProvideNewConstraints(String),
       Cancel,
   }
   ```

6. **Create creative solution patterns**:
   ```rust
   pub enum CreativePattern {
       PartialSolution,    // Do part of the task
       EquivalentAlternative, // Different approach, same outcome
       SteppingStone,      // Intermediate step toward goal
       Workaround,         // Avoid the blocking issue
       Decomposition,      // Break into achievable subtasks
       Delegation,         // Use external tool/service
       Approximation,      // Close enough to the requirement
   }

   pub fn apply_creative_pattern(
       task: &str,
       pattern: CreativePattern,
       context: &ExecutionContext,
   ) -> Result<Program>;
   ```

7. **Log creative solutions for learning**:
   ```rust
   pub struct CreativeSolutionLog {
       pub timestamp: u64,
       pub original_task: String,
       pub blocking_issue: String,
       pub pattern_used: String,
       pub relaxation: String,
       pub alternative_approach: String,
       pub user_accepted: bool,
       pub outcome_success: bool,
   }

   // Save to sessions/{session}/creative_log.jsonl
   ```

## Acceptance Criteria
- [ ] Constraint analysis module created
- [ ] Hard vs soft constraints distinguished
- [ ] Relaxation suggestions generated
- [ ] Alternative approaches proposed
- [ ] Creative patterns implemented (6+ patterns)
- [ ] User negotiation for major changes
- [ ] Creative solutions logged for learning

## Files to Create
- `src/constraint.rs` - Constraint analysis module
- `config/{model}/constraint_relaxation.toml` - Relaxation prompt

## Files to Modify
- `src/orchestration_retry.rs` - Add creative problem-solving fallback
- `src/intel.rs` - Constraint analysis functions
- `src/app_chat_core.rs` - Integrate negotiation flow

## Priority
MEDIUM-HIGH - Enhances improvisation capabilities

## Dependencies
- Task 041 (Analogy-Based Reasoning) - complementary
- Task 042 (Multi-Strategy Planning) - complementary

## Philosophy Alignment
- **"Flexibility to improvise solutions that rigid rule-based systems would miss"**
- **"Execute workflows that genuinely address user objectives"** (not just stated requirements)
- **"Adaptive reasoning and improvisation rather than deterministic rules"**

## Example Scenarios

### Scenario 1: Impossible File Operation
```
Task: "delete all files in /system/protected"
Blocked: Permission denied, even with sudo

Constraint Analysis:
- Hard: Data must be removed
- Soft: Must use direct deletion

Relaxation:
- Use archiving instead of deletion
- Move to quarantine folder
- Mark for deletion on next boot

Alternative:
"Instead of deleting (which requires system access), 
I can archive the files and move them to a quarantine 
folder you control. This achieves the goal of removing 
them from active use. Would you like me to do this?"
```

### Scenario 2: Missing Tool
```
Task: "convert all PNGs to WebP"
Blocked: cwebp not installed

Constraint Analysis:
- Hard: Convert images to WebP
- Soft: Use cwebp specifically

Relaxation:
- Install cwebp first
- Use alternative converter (imagemagick)
- Use online conversion service

Alternative:
"cwebp is not installed. I can:
1. Install it: brew install webp (requires sudo)
2. Use ImageMagick: convert *.png *.webp (already available)
3. Use online converter (requires uploading files)

Recommendation: Option 2 - ImageMagick is already 
available and can do the conversion."
```

### Scenario 3: Time Constraint
```
Task: "analyze all 10,000 files and report findings"
Blocked: Would take 2+ hours, exceeds practical limits

Constraint Analysis:
- Hard: Understand file characteristics
- Soft: Analyze ALL files

Relaxation:
- Statistical sampling (analyze 100 representative files)
- Incremental analysis (start with most important)
- Summary-level analysis (metadata only)

Alternative:
"Analyzing all 10,000 files would take 2+ hours. 
Instead, I can:
1. Analyze a representative sample of 100 files (5 min)
2. Start with the most recently modified files
3. Do a metadata-only summary of all files

Recommendation: Option 1 - sampling gives accurate 
insights quickly. We can analyze more if needed."
```
