# Elma Task Guidelines

## Objective

Elma produces correct, grounded answers on any model size, with stability, regardless of how many intel-unit calls, tool calls, approach retries, or how much wall-clock time it takes.

The system must adapt to the model. The model must never be asked to adapt to a brittle system. Larger thinking and tool-calling-native models are supported, but Elma must remain effective on constrained local models.

## Core Principles

1. The model is a given. If output is wrong, inspect prompt shape, context shape, decomposition, tool contracts, runtime state, and cognitive load per call before blaming capability.
2. One intel unit should perform one narrow decision. If a model fails multi-field reasoning, split the job into smaller single-purpose units.
3. Strict JSON is the standard intel-unit output format. Provider-native tool calling is used for the action loop.
4. Accuracy and stability outrank speed. Elma can spend more calls and more time when that is what completion requires.
5. Semantic continuity must survive the full chain: user intent -> classification -> route -> complexity -> formula -> work graph -> approach -> instruction -> step -> final answer.
6. Tools are part of Elma's intelligence. Elma should know her tool arsenal, use tools when needed, and recover when a tool strategy fails.
7. Failure analysis is holistic. Failures may come from runtime architecture, prompt packets, tool schemas, decomposition, context hygiene, finalization, or model limits.
8. **Relaxed, not rushed**: Elma resists premature finalization. Budgets favor completion over speed. When the model tries to finalize early, the system pushes back with evidence of incomplete work — not pressure to hurry up.
9. **Laser focused, not chaotic**: After planning, Elma commits to sub-goals in topological order. She does not switch approaches or drift until effort has been spent and stagnation is confirmed. Sub-goal commitment is a runtime contract, not a suggestion.

## Task Quality Rules

Every new pending task must satisfy these rules before it is added to `_tasks/pending/`.

1. It must improve at least one of truthfulness, reliability, bounded autonomy, small-model effectiveness, or context efficiency without materially harming the others.
2. It must not violate Elma's objectives or AGENTS.md.
3. It must not introduce deterministic user-input keyword matching for routing, classification, behavior selection, scope decisions, or finalization.
4. It must not repeat circular repairs that have already failed, such as adding more broad prompt text when the actual issue is missing runtime state, weak contracts, stale context, or missing decomposition.
5. It must define observable success criteria and verification steps. A task that cannot be tested should be revised until it can be.
6. It must prefer deleting or replacing dead/unwired logic over adding another parallel path.
7. It must preserve offline-first behavior unless the task explicitly concerns optional online behavior.

## Model Bottleneck Policy

Only report the model as the bottleneck after the system has exhausted structural recovery options that are available locally:

1. Increase or reset iteration budget when the objective is unfinished and progress is still possible.
2. Fork a sibling approach from the same objective instead of continuing down a failed branch.
3. Vary temperature in bounded retries for extreme cases where repeated deterministic attempts fail.
4. Compact, summarize, or atomize context so the model sees a smaller and clearer problem.
5. Expire irrelevant stale tool results and failed attempts from the live narrative while preserving them in session trace.
6. Add a focused intermediary intel unit when one cognitive job is too broad.
7. Repair tool arguments or change tool strategy when the model cannot produce a valid call.

If all recovery options fail, Elma should stop honestly, explain the exact blocker, show the evidence gathered, and state the next actionable path.

## Long-Running Autonomy

Elma is designed to continue until the user request is resolved or a real blocker is reached. Runtime budgets must favor completion:

- High iteration ceilings for open-ended work.
- Stagnation detection that changes strategy before stopping.
- Approach branching after repeated failure.
- Completion contracts that prevent polished partial answers.
- Trace and transcript rows that show routing, budget, compaction, retries, coverage, stop reasons, and finalization decisions.

Stopping is acceptable only when the system can explain why further autonomous work is not currently productive.

## What Success Looks Like

A user types anything. Elma:

1. Assesses complexity as the main work-depth gate.
2. Preserves intent in a compact current-turn objective.
3. Selects the necessary tools and evidence strategy.
4. Builds or updates the work graph according to complexity.
5. Executes steps through recoverable approach branches.
6. Persists tasks and runtime state to disk.
7. Maintains a scoped evidence and coverage contract.
8. Produces a truth-grounded final answer backed by collected evidence.

No step should fail permanently because "the model is too small." If a step fails, the system decomposes, retries with a changed approach, repairs context, or stops with a specific blocker only after structural recovery is exhausted.

## Non-Goals

- Using a larger model as the default solution.
- Adding keyword-based routing or classification.
- Optimizing speed ahead of correctness.
- Adding cosmetic tasks that do not improve reliability, truthfulness, or maintainability.
