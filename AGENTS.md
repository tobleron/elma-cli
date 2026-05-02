This file provides universal guidance for agents working in this repository.

## Elma CLI Philosophy

Elma is a local-first autonomous CLI agent designed to deliver the highest reliability and practical usefulness possible on constrained local models.

Core philosophy:
- Reliability before speed.
- Adaptive reasoning before deterministic rule playback.
- Small-model-friendly decomposition before asking one model call to do too much.
- Truth-grounded answers before polished but weakly supported answers.
- Offline-first behavior by default, with network use only when truly necessary.
- Prompt principles over long brittle examples.
- Classification signals are soft guidance, not hard law.

Elma should feel premium, careful, and capable even on low-end hardware. The system must maximize quality per token, quality per unit of reasoning, and quality per unit of context window.

## Critical Behavioral Rules

These rules are enforced in code and must never be violated by new contributions.

### 1. Do Not Turn Elma Into A Keyword Matcher

Routing, classification, and behavior selection must never use hardcoded word triggers. If you find yourself writing `if input.contains("word")`, you are violating Elma's philosophy. Use model confidence, entropy, evidence availability, and bounded fallback principles instead.

### 2. Keep The System Principle-First, Not Example-Driven

Prompts must describe reasoning principles, not list examples. A prompt that is mostly examples or exceptions is wrong. Rewrite it from first principles: what the unit does, why it exists, what contract it fulfills.

### 3. Maintain Semantic Continuity From User Intent To Final Answer

The meaning of the user's request must survive every transformation: intent annotation → classification → routing → complexity assessment → formula selection → work graph decomposition (Goal → SubGoal → Plan → Instruction) → approach execution → step execution → final answer. If the user asks for X and the answer solves Y, that is a semantic continuity failure. Inspect continuity by comparing the raw prompt, intent annotation, chosen route, graph nodes, executed steps, and final answer.

### 4. If A Model Is Too Weak For A Step, Decompose — Don't Bloat

When a small model struggles:
- First tighten the narrative/context
- Then reuse an existing intel unit if it fits
- Then add a new focused intermediary intel unit if truly needed

Never respond to small-model weakness by stuffing more examples, overfitting rules, forcing giant prompts, or merging cognitive jobs into one prompt. Preferred pattern: one intel unit, one role, one narrow decision.

### 4a. Complexity Is The Main Gate

Complexity assessment determines the maximum depth of the work graph before any work begins:

| Complexity | Max Depth | Graph Layers |
|------------|-----------|--------------|
| DIRECT | 0 | Skip graph, go straight to instruction |
| INVESTIGATE | 2 | Goal → Instruction |
| MULTISTEP | 3 | Goal → SubGoal → Plan → Instruction |
| OPEN_ENDED | 4+ | Full pyramid, parallel approaches |

Never bypass complexity assessment. If a task exceeds its assessed depth, re-assess before adding layers.

### 4b. Approaches Are Sibling Branches

When an approach fails, the system forks a new sibling approach from the same objective — it does not continue down a failing branch. Each approach is a separate branch in the work graph with its own Goal → SubGoal → Plan → Instruction chain. The approach engine tracks failures, prunes exhausted branches, and spawns alternatives. Tasks are scoped to individual approaches.

### 5. Keep The Bottom Status Bar Limited To Core Runtime Metrics Only

The footer/status bar must show only: model name, token count, elapsed time. Execution mode, queue notices, operational notifications, and routing decisions belong in the chat transcript, not the footer.

### 6. Prefer Transcript-Native Operational Visibility

Budgeting, routing/formula choice, compaction, stop reasons, and hidden processes must surface as collapsible transcript rows. Do not bury these in trace-only state, debug logs, or hidden metadata. The transcript is the single source of truth for what happened during a session.

**This rule is critically important and is not currently applied properly.** Current behavior hides compaction triggers, routing decisions, and stop reasons behind trace-only logging. These must be surfaced as visible, collapsible transcript rows.

### 7. Never Blame The Model — Improve The System

Small-model weakness is an expected constraint, not a defect. When a model produces poor output, hallucinates, enters stagnation, or fails to follow instructions:

- **Do not** add comments like "model is too weak" or "this fails because the model is small"
- **Do not** respond by stuffing more examples into prompts, overfitting rules, or bloating context
- **Do** decompose: add a focused intermediary intel unit, tighten narrative context, or reduce cognitive load per step
- **Do** increase iteration budget, add retry with temperature variation, or use clean-context reset for finalization
- **Do** add real timeout mechanisms for blocking I/O instead of relying on model self-correction
- **Do** sanitize inputs (shell prompts, ANSI sequences, truncation artifacts) before they reach the model

The model is a given. The system must adapt to it. Every failure is a signal to improve decomposition, context hygiene, or execution robustness — never an excuse to blame the model.

### 8. Protect The Core System Prompt

`src/prompt_core.rs` contains the canonical system prompt sent to the model on every tool-calling turn. It is the result of extensive iteration and performance tuning.

**You must NEVER modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` without explicit user approval.**

If you believe the prompt needs adjustment:
1. Document the proposed change and its rationale
2. Run scenario tests to verify behavior
3. Present the change to the user for explicit approval
4. Only then update the prompt and its build-time hash

This rule applies to all agents, including Elma itself, external coding assistants, and automated refactoring tools.

### 9. Tasks Must Be Persisted, Not Memory-Only

Todo tasks created during a session must persist to disk:
- Per-session: `sessions/<id>/runtime_tasks/tasks.json` for session resume
- Per-workspace: `_elma-tasks/NNN_{auto|user}_{slug}_{uid}.md` for cross-session traceability

Two task types exist:
- **auto**: Generated automatically when a work graph Instruction node resolves to a Step
- **user**: Initiated by direct user request

Task status flows: `pending → in_progress → completed|failed`. Never store tasks only in UI memory.

## Theme

The Elma terminal UI uses a tokenized color system:

| Token | Color | Purpose |
|-------|-------|---------|
| Background | Dark grey | Terminal canvas background |
| Primary | Pink | Primary accent, prompt affordances, active selection, attention states |
| Secondary | Cyan | Complementary accent, tools, file mentions, progress, informational contrast |
| Shell output | Black | Sub-terminal shell command output panels |
| Elma output (final) | White | Assistant responses after thinking completes |
| Elma output (thinking) | Grey | Assistant responses while thinking is in progress |
| Metadata | Grey | Separators, disabled text, inactive hints, timestamps |

Theme implementation must be tokenized — all color references go through the canonical theme module. Future themes must be able to swap Pink for another primary color without rewriting renderers. Do not hardcode UI colors outside the theme module.

## Success Standard

A change is aligned with Elma only if it improves at least one of:
- Truthfulness
- Reliability
- Bounded autonomy
- Small-model effectiveness
- Context efficiency

without materially harming the others.

## Full Documentation

AGENTS.md is the quick-reference behavioral contract. Full guidelines live in `docs/`:

| Document | Covers |
|----------|--------|
| [ARCHITECTURAL_RULES.md](docs/ARCHITECTURAL_RULES.md) | All 12 non-negotiable architecture rules, narrative context rules, runtime priorities |
| [DEVELOPMENT_GUIDELINES.md](docs/DEVELOPMENT_GUIDELINES.md) | Commit rules, config safety, verification, de-bloating, snapshots |
| [DEVELOPMENT.md](docs/DEVELOPMENT.md) | Build, test, project structure, testing philosophy |
| [SOUL.md](docs/SOUL.md) | Elma's character, identity, tone, autonomy boundary |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | End-to-end workflow, module map, all systems |
| [SKILL_SYSTEM.md](docs/SKILL_SYSTEM.md) | Skills, formulas, playbook rules, context-budget awareness |

Task management procedures are in [`_tasks/TASKS.md`](_tasks/TASKS.md).
