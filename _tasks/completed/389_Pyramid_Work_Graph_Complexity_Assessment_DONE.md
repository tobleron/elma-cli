# Task 389: Pyramid Work Graph Complexity Assessment

**Status:** Pending
**Priority:** HIGHEST
**Estimated effort:** 3-4 days
**Dependencies:** Task 378, Task 380
**References:** user objective for Objectives -> Goals -> Sub-Goals -> Plans -> Instructions

## Objective

Represent complex requests as a bounded work graph:

```text
Objective -> Goals -> Sub-Goals -> Plans -> Instructions
```

Each thread in the graph belongs to a named approach. The graph exists in Rust state; model calls only fill one small field set at a time.

## Problem

Small models fail when a single prompt asks them to assess complexity, choose a route, plan steps, preserve intent, and produce JSON all at once. Elma needs an explicit hierarchy that reduces cognitive load per model call.

## Implementation Plan

1. Add work graph types, likely in a new module:
   - `ObjectiveNode`
   - `GoalNode`
   - `SubGoalNode`
   - `PlanNode`
   - `InstructionNode`
   - `ApproachId`
2. Add a complexity assessment unit that only decides:
   - `complexity`
   - `needs_graph`
   - `reason`
3. Add graph builders that decompose one layer at a time.
4. Preserve semantic continuity by carrying the original objective into every node.
5. Store graph state in the session so retries and summaries can explain what happened.
6. Emit graph creation and stage transitions as transcript rows.

## JSON Constraint

Each intel unit must produce at most three required fields and no object nesting. If a graph layer needs more data, split it into another unit.

## Verification

```bash
cargo test work_graph
cargo test complexity
cargo test continuity
cargo test orchestration
cargo build
```

Manual probes:

- A simple factual request should avoid the graph.
- A multi-step coding request should create goals and instruction nodes.
- A failed instruction should not discard the original objective.

## Done Criteria

- Complex work can be decomposed into graph nodes without prompt bloat.
- The graph preserves the raw user objective and current approach.
- Transcript rows expose graph decisions.
- All model JSON stays within Task 378 limits.

