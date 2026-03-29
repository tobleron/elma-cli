# 023_Implement_Complexity_Trigg ered_Hierarchical_Decomposition

## Problem
Elma currently measures complexity (DIRECT, INVESTIGATE, MULTISTEP, OPEN_ENDED) but does NOT act on it appropriately.

From granite session `s_1774826560_84116000`:
```
User: "Read all Rust source files and provide comprehensive summary"
  ↓
Complexity: OPEN_ENDED, risk: MEDIUM
  ↓
Elma: Tries to execute directly with single plan
  ↓
Result: Massive command (find | xargs cat), no decomposition, session crash
```

The complexity assessment exists but is ignored - Elma doesn't decompose large tasks.

## Objective
Implement hierarchical task decomposition triggered by complexity assessment.

When complexity is HIGH or OPEN_ENDED, Elma must:
1. Generate a MASTERPLAN first (strategic overview)
2. Decompose into SUBGOALS (milestones)
3. Decompose each subgoal into TASKS
4. Generate METHODS for each task
5. Execute ACTIONS one at a time
6. Aggregate results upward

## Technical Tasks

### Phase 1: Data Structures
- [ ] **Add new unit types to `src/program.rs`**
  ```rust
  pub enum UnitType {
      // Existing
      Shell, Select, Edit, Plan, MasterPlan, Decide, Summarize, Reply,
      // NEW hierarchy
      Goal,      // Level 1: Final state
      Subgoal,   // Level 2: Intermediate milestone
      Task,      // Level 3: Work unit
      Method,    // Level 4: Decomposition strategy
      Action,    // Level 5: Primitive executable
  }
  ```

- [ ] **Add parent-child tracking**
  ```rust
  pub struct StepCommon {
      pub id: String,
      pub parent_id: Option<String>,  // NEW: links to parent unit
      pub unit_type: UnitType,        // NEW: what level is this?
      pub depth: u8,                  // NEW: hierarchy depth (1-5)
      // ... existing fields
  }
  ```

- [ ] **Add decomposition metadata**
  ```rust
  pub struct Decomposition {
      pub goal_id: String,
      pub subgoals: Vec<Subgoal>,
      pub tasks: Vec<Task>,
      pub methods: Vec<Method>,
      pub actions: Vec<Action>,
  }
  ```

### Phase 2: Complexity-to-Hierarchy Mapping
- [ ] **Define decomposition rules**
  ```rust
  fn get_required_depth(complexity: &str, risk: &str) -> u8 {
      match (complexity, risk) {
          ("DIRECT", _) => 1,           // Action only
          ("INVESTIGATE", "LOW") => 2,  // Task → Action
          ("INVESTIGATE", "MEDIUM") => 3, // Subgoal → Task → Action
          ("MULTISTEP", _) => 3,        // Subgoal → Task → Action
          ("OPEN_ENDED", _) => 5,       // Full hierarchy
          (_, "HIGH") => 4,             // At least Method level
          _ => 2,
      }
  }
  ```

- [ ] **Add decomposition trigger in orchestration**
  ```rust
  // In src/orchestration.rs, after complexity assessment
  let required_depth = get_required_depth(&complexity.complexity, &complexity.risk);
  
  if required_depth >= 4 {
      // FORCE hierarchical decomposition
      let masterplan = generate_masterplan(...).await?;
      let subgoals = decompose_to_subgoals(&masterplan, ...).await?;
      // ... continue decomposition
  } else {
      // Direct execution (current behavior)
      execute_directly(...).await?;
  }
  ```

### Phase 3: Masterplan Generation
- [ ] **Create `src/decomposition.rs` module**
  - `generate_masterplan()` - strategic overview
  - `decompose_to_subgoals()` - milestone breakdown
  - `decompose_to_tasks()` - operational units
  - `generate_methods()` - how-to specifications
  - `aggregate_results()` - bottom-up result propagation

- [ ] **Add masterplan-specific system prompt**
  ```
  You are Elma's strategic planner.
  
  For OPEN_ENDED or HIGH risk tasks, generate a MASTERPLAN:
  - Identify 3-5 major phases
  - Define success criteria for each phase
  - Estimate dependencies between phases
  - Do NOT generate executable steps yet
  
  Output format:
  {
    "goal": "ultimate objective",
    "phases": [
      {"name": "Discovery", "objective": "...", "success": "..."},
      ...
    ]
  }
  ```

### Phase 4: Execution with Hierarchy Awareness
- [ ] **Modify `run_autonomous_loop()` to handle hierarchy**
  - Track current position in hierarchy
  - Execute actions at leaf level only
  - Aggregate results to parent units
  - Mark subgoals complete when all child actions succeed

- [ ] **Add progress tracking**
  ```rust
  pub struct HierarchyProgress {
      pub goal_complete: bool,
      pub subgoals_complete: Vec<String>,
      pub current_task: Option<String>,
      pub actions_executed: u32,
      pub actions_total: u32,
  }
  ```

### Phase 5: Validation & Testing
- [ ] **Add hierarchy validation**
  - Every Action must have parent Method
  - Every Method must have parent Task
  - Every Task must have parent Subgoal
  - Every Subgoal must have parent Goal
  - No orphan units allowed

- [ ] **Create test scenarios**
  - `scenarios/hierarchy/hier_001_simple_greeting.md` - DIRECT → 1 level
  - `scenarios/hierarchy/hier_002_file_inspection.md` - INVESTIGATE → 2 levels
  - `scenarios/hierarchy/hier_003_multi_step_task.md` - MULTISTEP → 3 levels
  - `scenarios/hierarchy/hier_004_codebase_analysis.md` - OPEN_ENDED → 5 levels

## Acceptance Criteria
- [ ] OPEN_ENDED tasks automatically generate masterplan first
- [ ] Hierarchy depth matches complexity assessment
- [ ] Parent-child relationships are validated
- [ ] Results aggregate from actions → methods → tasks → subgoals → goal
- [ ] Progress is trackable at each level
- [ ] granite session failure case now succeeds with decomposition

## Verification
1. Test with "Analyze entire codebase" request
2. Confirm masterplan is generated first (3-5 phases)
3. Confirm decomposition into subgoals before any execution
4. Confirm actions execute one at a time
5. Confirm final report aggregates all subgoal results
6. Verify no massive single-step commands (like `find | xargs cat`)

## Related
- Session: `s_1774826560_84116000` (granite failure on codebase analysis)
- Session: `s_1774825976_127357000` (3B model identity confusion)
- Files: `src/orchestration.rs`, `src/program.rs`, `src/types.rs`
- T018-T021: Error handling improvements (complementary)

## Notes
- This is a MAJOR architectural change - implement incrementally
- Keep backward compatibility for simple tasks (DIRECT, INVESTIGATE)
- Test each phase before proceeding to next
- Consider feature flag for gradual rollout
