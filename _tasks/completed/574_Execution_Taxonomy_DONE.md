# 574 — Create Canonical Execution Taxonomy Document

- **Priority**: Medium
- **Category**: Documentation
- **Depends on**: None
- **Blocks**: None

## Problem Statement

The codebase has multiple overlapping classification/execution concepts:
- **Complexity tiers**: DIRECT, INVESTIGATE, MULTISTEP, OPEN_ENDED
- **Execution levels**: Action, Task, Plan, MasterPlan
- **Routes**: CHAT, SHELL, WORKFLOW, SELECT, DECIDE, PLAN
- **Work graph layers**: Goal → SubGoal → Plan → Instruction
- **Skill formulas**: Patterns, scores, formula selection

The relationships between these concepts are not documented in one place. A developer new to the codebase cannot easily understand how a user request flows through classification → routing → level assessment → formula selection → work graph → tool execution.

## Why This Matters for Small Local LLMs

The execution taxonomy directly affects:
- How the model's intent is classified
- How many layers of decomposition occur
- How many iterations the tool loop gets
- What tools are available at each layer

Ambiguity in the taxonomy leads to ambiguity in model behavior.

## Recommended Target Behavior

Create `docs/EXECUTION_TAXONOMY.md` that documents:
1. The complete execution pipeline from user input to final answer
2. Each classification/decision point and what it determines
3. The relationship between complexity, level, route, and formula
4. How each taxonomy layer maps to the work graph
5. Decision flow diagram (ASCII art or mermaid)
6. Examples of each taxonomy level with sample requests

## Source Files That Need Modification

- `docs/EXECUTION_TAXONOMY.md` (new)

## Acceptance Criteria

- Document covers all taxonomy concepts
- Includes decision flow diagram
- Includes examples for each complexity/level combination
- References source files where each concept is implemented
