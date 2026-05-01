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
1. Understands intent (one intel call, one sentence)
2. Classifies the speech act (one intel call, one label)
3. Decides the route (one intel call)
4. Assesses evidence needs (one intel call per field)
5. Executes tools or answers directly
6. Produces a truth-grounded answer backed by collected evidence

No step ever fails because "the model is too small." If a step fails, it is split into smaller steps, or the approach is adjusted.

## Current Focus

**Make the basic workflow reliable on 3B models:** intent -> classify -> route -> assess -> execute -> answer.

## How We Get There

1. **Split multi-field intel units** (e.g., `evidence_need_assessor` into `needs_evidence` + `needs_tools`)
2. **Clean-context finalization** -- final answers never leak internal state, stop reasons, or error messages
3. **Transcript visibility** -- every routing decision, stop reason, and hidden process visible in transcript rows
4. **Re-apply non-DSL improvements** (routing collapse fix, hard max iterations, stagnation detection, dedup, CHAT bypass)

## Non-Goals

- Using a larger model as a solution
- Adding keyword-based routing
- Optimizing for speed over correctness

---

*This document defines the end goal. See `_masterplan.md` for the prioritized task roadmap and `_tasks/pending/` for individual task files with implementation details.*
