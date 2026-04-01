# Stress Test S006: Global Architecture Audit

## 1. The Test (Prompt)
"Perform a principal-level audit of the entire `src/` directory. Score every file based on its 'Drag' (complexity vs utility) using the logic described in `_dev-system/ARCHITECTURE.md`. Generate a comprehensive report and identify the top 3 modules that are most in need of a 'Surgical Refactor'. Do not use the existing analyzer; perform this logic yourself."

## 2. Debugging Result Understanding
- **Success Criteria**: The model must read the `ARCHITECTURE.md`, understand the mathematical formula, and manually apply it to a sample of files.
- **Common Failure Modes**:
    - Math errors in the Drag calculation.
    - Laziness: Only auditing 1-2 files instead of a representative sample.
    - Hallucinating the analyzer's output instead of performing the audit itself.

## 3. Bottleneck Detection
- **Cognitive Load**: The model's ability to maintain the "mathematical state" across multiple file reads.
- **Utility Gap**: If the model's "manual" audit contradicts the automated analyzer without a good reason.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
