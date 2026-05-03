# Elma CLI End Goal

## Objective

Elma produces **correct, grounded answers on any model size, with stability**, regardless of how many intel unit calls or how much wall-clock time it takes.

The system must adapt to the model — the model must never be asked to adapt to the system. Larger models (thinking, tool-calling-native) are fully supported and their capabilities are utilized when available.

## Principles

1. **The model is a given.** If output is wrong, the prompt, decomposition, or cognitive load per call is wrong. Never blame the model. Never suggest a larger model.
2. **One intel unit = one narrow decision.** When a 3B model fails to produce multi-field output, split into separate single-field units.
3. **JSON is the standard intel-unit output format.** Provider-native tool calling is used for the action loop. Intel units output JSON key=value records.
4. **Accuracy over speed, with stability.** Elma takes as many model calls as needed. Latency and instability are not concerns.
5. **The answer must survive semantic continuity.** User intent -> classification -> route -> execution -> final answer must all solve the same problem.
6. **Tool awareness.** Elma knows her arsenal of tools and uses them when needed to accomplish user requests.
7. **Failure analysis is holistic.** Failures may indicate system, prompt, tool, decomposition, or other issues -- not just model limitations.

## What Success Looks Like

A user types anything. Elma:
1. Assesses complexity (one intel call, DIRECT/INVESTIGATE/MULTISTEP/OPEN_ENDED) — MAIN GATE
2. Understands intent (one intel call, one sentence)
3. Classifies the speech act (one intel call, one label)
4. Decides the route (one intel call)
5. Selects the formula (one intel call, matches complexity + intent)
6. Builds the work graph: Objective → Goal → SubGoal → Plan → Instruction (depth gated by complexity)
7. Creates approach branches (sibling forks for retry, not continuation down failed branches)
8. Generates persisted tasks: `NNN_{auto|user}_{slug}_{uid}.md` in `_elma-tasks/`
9. Executes steps (shell, read, edit, reply, etc.) mapped from Instruction nodes
10. Updates task status from step results (pending → in_progress → completed|failed)
11. Produces a truth-grounded answer backed by collected evidence

No step ever fails because "the model is too small." If a step fails, it is split into smaller steps, or the approach is adjusted. Failed approaches fork new siblings — they never continue down a broken branch.

## Current Focus

**Completed:** The full hierarchy is now reliable on 3B models: complexity → intent → classify → route → formula → graph → approach → instruction → step → answer.

**New Focus Areas:**
1. **Recipe system adoption** — expand formula-to-recipe bridge coverage (Task 451)
2. **Model capability awareness** — leverage token budgeting and feature detection (Task 448, 499)
3. **Advanced code intelligence** — repo map with symbol awareness (Task 463), interpreter tools (Task 461)
4. **Execution safety** — sandboxed profiles (Task 459), multi-file atomic patches (Task 455)
5. **Transcript richness** — event logging (Task 470), continuity tracking (Task 380), effective history (Task 310)

## How We Got Here (Completed)

1. ✅ **Complexity-gated decomposition** — complexity assessment decides maximum graph depth (Task 389)
2. ✅ **Split multi-field intel units** — evidence_needs split, 18+ focused units in `intel_units/`
3. ✅ **Clean-context finalization** — final answers never leak internal state (Task 384)
4. ✅ **Transcript visibility** — routing decisions, graph creation, approach forks, task changes visible (Task 470)
5. ✅ **Approach branching** — failures fork new approaches from objective root (Task 390)
6. ✅ **Task persistence** — tasks survive session close via `runtime_tasks/tasks.json` and `_elma-tasks/` (Task 494)
7. ✅ **Non-DSL improvements** — routing fixes, stagnation detection, CHAT bypass
8. ✅ **Work graph bridge** — graph nodes ↔ tasks ↔ step execution (Task 494)
9. ✅ **Background tasks** — async job execution with start/status/output/stop (Task 268)
10. ✅ **Semantic continuity** — tracking across full pipeline (Task 380)

## Next Objectives

1. **Recipe system expansion** — add recipes for common workflows (code review, refactoring, debugging)
2. **Token efficiency** — optimize context usage with tiktoken-rs integration (Task 499)
3. **Multi-model support** — leverage model capabilities registry for optimal model selection
4. **Enhanced document intelligence** — improve format support and extraction accuracy
5. **Developer experience** — improve debugging tools, transcript analysis, and error reporting

## Non-Goals

- Using a larger model as a solution
- Adding keyword-based routing
- Optimizing for speed over correctness

---

*This document defines the end goal. See `_masterplan.md` for the prioritized task roadmap and `_tasks/pending/` for individual task files with implementation details.*
