# Task 050: Revise Core Formulas (Plan Family)

## Priority
**P1 - HIGH** (Complex tasks depend on these formulas)

## Context
This task covers the **Plan Family** of formulas:
- `plan_reply` - Create step-by-step plan then respond
- `masterplan_reply` - Create high-level strategic plan then respond
- `cleanup_safety_review` - Review before destructive operations
- `code_search_and_quote` - Search codebase and quote findings

These formulas handle COMPLEX tasks that require planning or careful review.

## Objective
Improve reliability of planning formulas through principle-based prompt refinement and hierarchical decomposition integration.

## Work Items

### 1. Document Current Formulas

```
Formula: plan_reply
Principle: Use when user requests a concrete implementation plan with specific steps
Expected evidence: Understanding of current state, desired end state
Expected reply: Numbered steps with clear success conditions
Failure modes: Too vague, missing prerequisites, wrong step order

Formula: masterplan_reply
Principle: Use when user requests strategic overview with phases/milestones
Expected evidence: High-level understanding of project/goal
Expected reply: Phases with goals, not executable steps
Failure modes: Too detailed (should be strategic), missing success criteria

Formula: cleanup_safety_review
Principle: Use when operation might delete/modify important files
Expected evidence: List of files to be affected, their importance
Expected reply: Risk assessment with go/no-go recommendation
Failure modes: Overly cautious, missing critical files

Formula: code_search_and_quote
Principle: Use when user asks where/how something is implemented
Expected evidence: Search results with file paths and line content
Expected reply: Quoted code with explanation
Failure modes: Wrong files, outdated results, missing context
```

### 2. Create Validation Scenarios
Create 3-5 scenarios per formula in `scenarios/formula_validation/`:
- `plan_reply_01_implement_feature.md`
- `plan_reply_02_fix_bug.md`
- `masterplan_01_project_overview.md`
- `masterplan_02_migration_strategy.md`
- `cleanup_safety_01_delete_old.md`
- `code_search_01_find_implementation.md`

### 3. Integrate with Hierarchical Decomposition
For `masterplan_reply`, integrate with Task 023 (hierarchical decomposition):
- OPEN_ENDED → Full 5-level hierarchy
- MULTISTEP → 3-level hierarchy
- Use `generate_masterplan()` from `decomposition.rs`

### 4. Refine Prompts (Principle-Based)

**BEFORE (hardcoded):**
```toml
system_prompt = """
Use plan_reply when:
- User says "create a plan"
- User says "step by step"
- User wants to implement something
"""
```

**AFTER (principle-based):**
```toml
system_prompt = """
Use plan_reply when:
- User requests specific implementation with clear steps
- Steps can be executed sequentially
- Each step has clear success condition

Principle: Break objective into executable steps that build toward goal
"""
```

### 5. Add Planning Quality Checks
Add pre-execution validation:
- Plan has 2-8 steps (not too few, not too many)
- Steps are ordered logically (dependencies respected)
- Each step has clear purpose and success condition
- Plan achieves stated objective

## Acceptance Criteria
- [ ] All 4 plan formulas documented with principles
- [ ] 12+ validation scenarios created
- [ ] Hierarchical decomposition integrated for masterplan
- [ ] Prompts updated to principle-based
- [ ] Planning quality checks added
- [ ] Before/after metrics show improvement

## Expected Impact
- **+30% plan quality** (better step breakdown)
- **+25% masterplan accuracy** (strategic vs tactical clarity)
- **-40% cleanup errors** (better safety review)
- **+35% code search accuracy** (better query formulation)

## Dependencies
- Task 023 (hierarchical decomposition) - for masterplan
- Task 047 (Read/Search step types) - code_search uses Search

## Verification
- `cargo build`
- `cargo test`
- Run validation scenarios
- Compare plan quality before/after

## Architecture Alignment
- ✅ Principle-based prompts (no hardcoded rules)
- ✅ Hierarchical decomposition (OPEN_ENDED → masterplan)
- ✅ Safety first (cleanup_safety_review)
