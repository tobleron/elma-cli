This file provides universal guidance for agents working in this repository.

## Elma CLI Philosophy

Elma is a local-first autonomous CLI agent designed to deliver correct, grounded answers on any model size. The model is a given. The system adapts.

Core philosophy:
- **Accuracy over speed.** Elma should take as many model calls and as much wall-clock time as necessary to produce a correct answer. Speed is secondary.
- **Small-model-first.** The system targets 3B-class models. If a 3B model cannot perform a step reliably, the step is too complex. Decompose it. Larger models (thinking models, tool-calling-native models) must still be supported — their capabilities should be utilized when available, never stripped.
- **Reliability before speed.** Wait for correctness. Never shortcut for latency.
- **Adaptive reasoning before deterministic rule playback.** Use model confidence, entropy, and evidence — never hardcoded keyword triggers.
- **Truth-grounded answers before polished but weakly supported answers.** Every factual claim must trace back to collected evidence.
- **Offline-first behavior by default.** Network use only when truly necessary.
- **Prompt principles over long brittle examples.** Explain what to do and why, not what not to do with five counterexamples.
- **Classification signals are soft guidance, not hard law.** Routing is probabilistic, not boolean.

If a model fails to produce correct output, the failure belongs to the system — the prompt, the decomposition, the cognitive load per call. Never blame the model. Never suggest switching to a larger model. The correct response is always: split the job into smaller, single-purpose intel units that the model can handle.

Elma must feel premium, careful, and capable on 3B-class hardware. The system maximizes quality per token, quality per unit of reasoning, and quality per unit of context window.

## Critical Behavioral Rules

These rules are enforced in code and must never be violated by new contributions.

### 1. Do Not Turn Elma Into A Keyword Matcher

Routing, classification, and behavior selection must never use hardcoded word triggers. If you find yourself writing `if input.contains("word")`, you are violating Elma's philosophy. Use model confidence, entropy, evidence availability, and bounded fallback principles instead.

### 2. Keep The System Principle-First, Not Example-Driven

Prompts must describe reasoning principles, not list examples. A prompt that is mostly examples or exceptions is wrong. Rewrite it from first principles: what the unit does, why it exists, what contract it fulfills.

### 3. Maintain Semantic Continuity From User Intent To Final Answer

The meaning of the user's request must survive every transformation: intent annotation → routing → formula selection → execution → final answer. If the user asks for X and the answer solves Y, that is a semantic continuity failure. Inspect continuity by comparing the raw prompt, intent annotation, chosen route, executed steps, and final answer.

### 4. If A Model Is Too Weak For A Step, Decompose — Don't Bloat

When a small model struggles:
- First tighten the narrative/context
- Then reuse an existing intel unit if it fits
- Then add a new focused intermediary intel unit if truly needed

Never respond to small-model weakness by stuffing more examples, overfitting rules, forcing giant prompts, or merging cognitive jobs into one prompt. Preferred pattern: one intel unit, one role, one narrow decision.

### 5. Model-Produced Structured Output Must Use Compact DSL

Every new or changed intel unit that asks a model for structured output must use a compact Rust-native DSL contract, not JSON, YAML, TOML, XML, Markdown tables, or prose-shaped pseudo-structure.

The DSL must be designed for constrained local models:
- Use the smallest grammar that represents the job: one token for simple verdicts, one key/value line for bounded records, repeated prefixed lines for short lists, and block bodies only when multiline text is truly required.
- Keep the grammar boring: uppercase tags or commands, explicit field names, quoted strings only where needed, clear block terminators, and no hidden syntax.
- Do not add nesting, batches, variables, conditionals, loops, or arbitrary sublanguages.
- Keep the cognitive unit narrow: one intel unit, one role, one decision shape.
- Prefer compact repair feedback over examples-heavy prompts.

The LLM output is always untrusted text. For every structured intel output:
- Parse with a strict Rust parser.
- Reject empty output, prose before or after the DSL, Markdown fences, JSON-looking output, duplicate fields, malformed quotes, and missing block terminators.
- Do not extract the "best looking" fragment from messy output.
- Do not silently fix quotes, missing fields, or missing terminators.
- Validate semantics after parsing before the value affects routing, execution, memory, or finalization.
- Return a short deterministic repair observation when parsing or validation fails.

Grammar-constrained decoding such as GBNF is useful but optional defense-in-depth. Grammar may reduce garbage; the parser proves syntax, validators prove meaning, and execution code proves safety.

User-facing configuration should be TOML-first. JSON may remain only for provider wire formats, third-party contracts, or documented local-state boundaries. It must not be used as the model-output format for intel units.

### 6. Keep The Bottom Status Bar Limited To Core Runtime Metrics Only

The footer/status bar must show only: model name, token count, elapsed time. Execution mode, queue notices, operational notifications, and routing decisions belong in the chat transcript, not the footer.

### 7. Prefer Transcript-Native Operational Visibility

Budgeting, routing/formula choice, compaction, stop reasons, and hidden processes must surface as collapsible transcript rows. Do not bury these in trace-only state, debug logs, or hidden metadata. The transcript is the single source of truth for what happened during a session.

**This rule is critically important and is not currently applied properly.** Current behavior hides compaction triggers, routing decisions, and stop reasons behind trace-only logging. These must be surfaced as visible, collapsible transcript rows.

### 8. Never Blame The Model — Improve The System

Small-model weakness is an expected constraint, not a defect. When a model produces poor output, hallucinates, enters stagnation, or fails to follow instructions:

- **Do not** add comments like "model is too weak" or "this fails because the model is small"
- **Do not** respond by stuffing more examples into prompts, overfitting rules, or bloating context
- **Do not** suggest switching to a larger model as a solution. The model is a given. The system adapts. Larger models must still be supported as a capability tier — their thinking/reasoning and tool-calling features should be utilized when available.
- **Do** decompose: add a focused intermediary intel unit, tighten narrative context, or reduce cognitive load per step
- **Do** increase iteration budget, add retry with temperature variation, or use clean-context reset for finalization
- **Do** add real timeout mechanisms for blocking I/O instead of relying on model self-correction
- **Do** sanitize inputs (shell prompts, ANSI sequences, truncation artifacts) before they reach the model

The model is a given. The system must adapt to it. Every failure is a signal to improve decomposition, context hygiene, or execution robustness — never an excuse to blame the model.

### 9. Protect The Core System Prompt

`src/prompt_core.rs` contains the canonical system prompt sent to the model on every tool-calling turn. It is the result of extensive iteration and performance tuning.

**You must NEVER modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` without explicit user approval.**

If you believe the prompt needs adjustment:
1. Document the proposed change and its rationale
2. Run scenario tests to verify behavior
3. Present the change to the user for explicit approval
4. Only then update the prompt and its build-time hash

This rule applies to all agents, including Elma itself, external coding assistants, and automated refactoring tools.

### 10. Compact DSL Is Permanent Architecture

Compact DSL is the canonical model-output format for all intel units. JSON model output is permanently deprecated. This is an architectural decision, not a temporary experiment.

- **Never** propose returning to JSON, YAML, TOML, XML, or any other format for model-produced structured output.
- **Never** suggest that JSON would be "easier for the model" — the model is the one that must adapt to the DSL, not the other way around.
- **Never** introduce a new intel unit that produces JSON output.
- **Do** tighten prompts, grammar, and repair feedback when a model struggles with DSL.
- **Do** decompose multi-field DSL units into single-field units when the model fails to produce all fields correctly.
- **Do** add a dedicated DSL format specialist intel unit whose only job is to generate exact DSL syntax for a given decision.

JSON may only exist at provider wire boundaries (OpenAI-compatible chat API), third-party contracts, and documented local-state storage. It must never appear in a model-output prompt contract.

### 11. One Intel Unit = One Decision

When a 3B model fails to produce correct multi-field DSL from a single prompt, the prompt is asking too much. The fix is not a bigger model — it is a narrower intel unit.

Every intel unit must make exactly one meaningful decision and emit exactly one compact DSL command:
- A classifier returns one label from a bounded set.
- An assessor returns one boolean verdict.
- A selector picks one item from a list.
- A summarizer composes one narrative from pre-filled facts.

Multi-field output (e.g., `needs_evidence` AND `needs_tools` in one response) is a decomposition failure. Split it into two intel units each returning one field. DSL generation (formatting a decision into exact syntax) is a separate cognitive job from making the decision. Split them.

When in doubt, prefer more narrow intel calls over fewer complex ones. A 3B model making 5 single-field correct decisions is better than making 1 multi-field wrong one.

### 12. Accuracy Over Speed — No Shortcuts

Elma must produce a correct, grounded answer regardless of how many model calls or how much wall-clock time it takes.

- Never shortcut a safety check, verification step, or evidence requirement for latency.
- Never collapse two classification stages into one to save an LLM call.
- Never omit repair feedback because "the model might fix it on its own."
- Never skip evidence assessment because the route looks confident.
- Each intel unit call, each repair retry, each verification pass is justified if it increases correctness.

The user can wait. A wrong answer costs more than a slow one.

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
