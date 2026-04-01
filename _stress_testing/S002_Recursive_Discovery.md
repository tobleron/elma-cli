# Stress Test S002: External Recursive Discovery

## 1. The Test (Prompt)
"Perform a recursive scan of `_stress_testing/_opencode_for_testing/`. Map the directory structure and identify the top 3 largest files by line count. Analyze their imports to determine if they are cohesive modules."

## 2. Debugging Result Understanding
- **Success Criteria**: Agent uses `ls -R` or `find`. It identifies the large files and reads their headers to check imports.
- **Common Failure Modes**:
    - "Losing" context of which folder it is scanning.
    - Plan collapse: Mapping the folder but forgetting the import analysis.

## 3. Bottleneck Detection
- **Context Fog**: High file count in the test directory causing reasoning errors.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
