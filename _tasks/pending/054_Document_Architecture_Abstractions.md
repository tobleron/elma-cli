# Task 054: Document Architecture Abstractions

## Objective
Create a comprehensive architectural guide that explains the project's high-level abstractions, specifically "Formulas," "Programs," and "Intel Units."

## Context
The codebase is technically excellent but the domain-specific terminology (e.g., "FormulaSelection", "ScopePlan", "ExecutionLadder") has a steep learning curve. Clear documentation will improve maintainability and ease of contribution.

## Technical Details
- **File**: `docs/ARCHITECTURE_CONCEPTS.md`
- **Content**:
    - **Intel Units**: Explain the Pre-flight/Execute/Post-flight/Fallback lifecycle.
    - **Formulas**: Define what a formula is (a strategy template) and how they are selected based on route/complexity.
    - **Programs & Steps**: Describe how a formula decomposes into a `Program` of discrete `Step`s.
    - **Execution Ladder**: Explain the tiered orchestration approach for minimum-sufficient reasoning.
    - **The _dev-system**: Summarize the "Drag" formula and de-bloating philosophy.

## Verification
- Review the document for clarity and technical accuracy.
- Ensure it aligns with the implementation in `src/orchestration_*.rs` and `src/intel_trait.rs`.
