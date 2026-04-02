# Task 012: Iterative Program Refinement

## Priority
**P0-4.1 - CRITICAL (PILLAR 4: RELIABILITY TASKS)**
**Blocked by:** P0-1 (JSON Reliability), P0-2 (Context Narrative), P0-3 (Workflow Sequence)

## Status
**PENDING** — Blocked on completion of P0-1, P0-2, P0-3

## Renumbering Note
- **Old Number:** Task 013
- **New Number:** Task 012 (per REPRIORITIZED_ROADMAP.md)
- **Reason:** Reprioritized as P0-4.1, must complete after foundational pillars

---

## Problem
Elma produces one program and executes it without iteration. Real autonomous agents iterate: plan → execute → observe → revise.

## Evidence
From "summarize AGENTS.md" session:
- Model produces program with empty `content` field in edit step
- No mechanism to detect incomplete execution and revise
- Single-shot execution model

## Goal
Implement a refinement loop that lets the model revise its program based on execution feedback.

## Implementation Steps

1. **Create new module** `src/refinement.rs`:
   ```rust
   pub struct RefinementContext {
       pub original_objective: String,
       pub step_results: Vec<StepResult>,
       pub failures: Vec<ExecutionFailure>,
       pub iteration: u32,
   }

   pub async fn refine_program(
       client: &reqwest::Client,
