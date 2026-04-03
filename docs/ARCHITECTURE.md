# Elma CLI Architecture Reference

## Core Design Principles

**Elma is a highly autonomous CLI agent engineered to deliver intelligent, reliable assistance through adaptive reasoning and improvisation rather than deterministic rules.**

### Philosophy
- **Context-First Approach**: Always explore workspace, validate evidence, or clarify missing context before acting
- **Confidence-Based Fallback**: When model uncertainty is high (entropy > 0.8), use safe defaults (CHAT route)
- **Principle-Based Routing**: Use model confidence metrics rather than hardcoded word patterns
- **Compact JSON for Efficiency**: Minimize token generation time with standardized output formats

### Core Protocols
1. **READ FIRST**: Always read `_tasks/TASKS.md` and check `_dev-tasks/` before acting
2. **Root-Relative Paths**: All file references must be relative to repository root
3. **Commitment Constraint**: Never commit unless explicitly asked ("save", "checkpoint", or "commit")
4. **Task Protocol**: Move to `_tasks/active/` → Implement → Verify (`cargo build`) → Archive

---

## Configuration Architecture

### Externalized TOML System

**Loading Order (Fallback Chain):**
```
1. Model-Specific Config: config/<model>/angel_helper.toml
   ↓ (if not found)
2. Global Default: config/defaults/angel_helper.toml
   ↓ (if not found)
3. Error (should never happen - defaults are complete)
```

**Benefits:**
- No recompilation needed - edit TOML, restart Elma
- Model-specific tuning for each model
- User customization without touching code
- Version control friendly
- Fallback safety guaranteed

### Configuration Hierarchy
```
config/
├── defaults/                    ← Global default prompts (63 configs)
│   ├── angel_helper.toml
│   ├── rephrase_intention.toml
│   ├── speech_act.toml
│   ├── orchestrator.toml
│   └── ... (all 63 intel units)
│
├── llama_3.2_3b_instruct_q6_k_l.gguf/  ← Model-specific overrides
│   ├── angel_helper.toml
│   ├── speech_act.toml
│   └── ... (65 configs)
│
├── granite-4.0-h-micro-UD-Q8_K_XL.gguf/  ← Another model
│   └── ... (65 configs)
│
└── ... (other models)
```

---

## Intel Unit Standard

### Output Format (All Units)

All intel units use this unified JSON format:

```json
{"choice": "<NUMBER>", "label": "<LABEL>", "reason": "<ULTRA_CONCISE_JUSTIFICATION>", "entropy": <FLOAT>}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `choice` | string | The number corresponding to the chosen option |
| `label` | string | Human-readable label (e.g., "CHAT", "WORKFLOW") |
| `reason` | string | Ultra-concise justification for the choice |
| `entropy` | float | Confidence level from 0.0 (certain) to 1.0 (uncertain) |

### Output Format Tiers

| Tier | Max Tokens | Use Case | Example Units |
|------|------------|----------|---------------|
| **1** | 64 | Single-choice classifiers | `router.toml`, `speech_act.toml`, `mode_router.toml` |
| **2** | 180 | Structured decisions | `evidence_mode.toml`, `command_repair.toml` |
| **3** | 540 | Multi-step planning | `workflow_planner.toml`, `planner_master.toml` |

### System Prompt Template

```
You are Elma's <ROLE>.

Return the most probable answer based on the context in addition to the confidence level from 0 to 1 (entropy) in json format.

Choice rules:
1 = <LABEL>: <what this choice represents about user intention>
2 = <LABEL>: <what this choice represents about user intention>
3 = <LABEL>: <what this choice represents about user intention>

Output format:
{"choice": "<NUMBER>", "label": "<LABEL>", "reason": "<ULTRA_CONCISE_JUSTIFICATION>", "entropy": <FLOAT>}
```

### Key Principles
1. **Choice rules describe intention, not consequence** - What the user IS doing, not what happens after
2. **Principle-based definitions** - Explain the reasoning principle, not pattern matching rules
3. **No heuristics section** - Trust the model to reason from the definitions
4. **No example section** - The format specification is sufficient
5. **Compact output requirement** - Explicitly state JSON format expectation

### Entropy Interpretation

| Range | Meaning | Action |
|-------|---------|--------|
| 0.0 - 0.3 | Very confident | Proceed without hesitation |
| 0.3 - 0.7 | Moderate confidence | Proceed, consider fallback |
| 0.7 - 1.0 | High uncertainty | Consider retry, alternative route, or user clarification |

---

## GBNF Grammar Enforcement

### Purpose
Force model to produce valid JSON at token generation level, not through prompts.

### Grammar Files Location
`config/{model}/grammars/`

### Current Coverage (11 profiles)
- `router.toml`, `speech_act.toml`, `mode_router.toml`
- `complexity_assessment.toml`, `evidence_needs.toml`, `action_needs.toml`
- `workflow_planner.toml`, `formula_selector.toml`, `scope_builder.toml`
- `critic.toml`, `logical_reviewer.toml`, `efficiency_reviewer.toml`
- `risk_reviewer.toml`, `outcome_verifier.toml`, `self_question.toml`

### How It Works

**Request Flow:**
```
User Input → Intent Annotation → Classification (router, speech_act, mode_router)
    ↓
chat_once_base()
  ├─ inject_grammar_if_configured() ← GRAMMAR INJECTION POINT
  │   ├─ Load grammar_mapping.toml
  │   ├─ Get grammar_path for profile
  │   └─ Inject into ChatCompletionRequest
  └─ Send request with grammar
    ↓
Model (constrained by GBNF grammar) → 100% Valid JSON Output
```

### Integration Points
- **Config root bootstrap**: `set_config_root()` called during app bootstrap
- **Grammar injection hook**: `inject_grammar_if_configured()` in `ui_chat.rs`
- **Intel unit integration**: Grammar injection in individual intel unit execution

---

## JSON Reliability Pipeline

### Multi-Layer Protection

1. **Prevention (Tuning)**: Find temperature that minimizes malformed JSON
2. **Detection (Parsing)**: Catch repetition loops and length violations
3. **Repair (fix_orphaned_keys)**: Fix common structural errors
4. **Fallback (jsonrepair-rs)**: General JSON repair library
5. **Graceful Degradation**: CHAT route fallback when all else fails

### Parsing Pipeline (in order)
1. Direct JSON - Clean JSON parsing
2. Markdown extraction - Extracts from ` ```json ... ``` ` blocks
3. Text extraction - Finds JSON in text with leading/trailing content
4. JSON repair - Uses `jsonrepair-rs` to fix malformed JSON
5. Legacy fallback - Parses raw digits or labels

### Repair Capabilities
The parser automatically repairs:
- Trailing commas: `{"choice": "1",}`
- Single quotes: `{'choice': '1'}`
- Unquoted keys: `{choice: "1"}`
- Missing closing braces (partial repair)
- Other common JSON malformations

### Metrics Target
- **Parse success rate**: >99.9%
- **Latency overhead**: <10% with grammar injection
- **Grammar coverage**: 100% for enabled profiles

---

## Tuning Safety and Reliability

### Current Tuning Surface
Elma tuning is restricted to per-profile numeric inference fields:
- `temperature`
- `top_p`
- `repeat_penalty`
- `max_tokens`

Tuning may also activate a different profile set, but it must not mutate:
- `system_prompt`
- `reasoning_format`
- profile names
- schemas
- slash-command behavior
- deterministic safety controls

### Protected Baselines
Each tune run evaluates three protected anchors when available:
- Active live profile set
- Immutable shipped baseline
- Runtime-default baseline derived from `/props.default_generation_settings`

### Quick Tune vs Full Tune

**Quick Tune (Startup Gate):**
Validates routing quality, workflow entry behavior, execution entry behavior, simple inspection/plan/decide behavior.
Does not guarantee deep multi-step recovery quality, large artifact workflows, broad platform portability, or full reviewer stability.

**Full Tune:**
Evaluates routing, workflow/program quality, execution quality, response quality, efficiency, baseline comparison, and stability penalty on a critical quick subset.

### Activation Policy
Activation prefers the candidate only when it meaningfully beats the preferred protected baseline. If improvement is marginal, Elma keeps the more stable baseline instead. When runtime defaults are close to the best baseline, runtime defaults are preferred.

### Reliability Boundaries
Tuning is intentionally prevented from rewriting Elma's identity. Prompt mutation is disabled by reliability policy. This keeps tuning:
- Reproducible
- Explainable
- Safe to compare across models
- Accountable when a model is simply a poor fit

---

## Workflow Sequence

### Current Sequence
```
1. User input
2. Intent annotation (intent_helper)
3. Classification (speech_act × workflow × mode → route)
4. Planning priors (complexity, evidence_needs, action_needs)
5. Workflow planner
6. Formula selector
7. Scope builder
8. Program building
9. Execution
10. Verification (sufficiency → critics → reviewers)
11. Refinement (if needed)
```

### Optimization Opportunities
- **Conditional workflow planning**: Only run when ladder >= Task (skip for Action level)
- **Parallel intel units**: Run independent units in parallel (complexity + evidence + action can execute concurrently)
- **Context boundary enforcement**: Each unit receives ONLY the context it needs (no over-sharing)

---

## SWOT Analysis Summary

### Strengths
- **Modular Orchestration**: Highly decoupled runtime with specialized components
- **Sophisticated Verification**: Multi-layered critic system provides high-fidelity feedback loops
- **Robust Intel Framework**: Principle-based `IntelTrait` architecture allows composable reasoning units
- **High-Fidelity Testing**: Substantial suite of intention scenarios, reliability probes, and sandboxed stress-testing assets

### Weaknesses
- **Architectural Drift**: Gap between advanced runtime capabilities and stale/misaligned task documentation
- **Cognitive Drag**: Legacy compatibility layers and duplicated orchestration paths increase maintenance complexity
- **Brittle Heuristics**: Residual reliance on keyword-based logic in critical routing/guardrail paths
- **Context Fragility**: Incomplete robustness for long-context scenarios

### Opportunities
- **Local-Model Optimization**: Implement token telemetry and budget-aware orchestration
- **Autonomous Refinement**: Leverage existing reflection/critic loop for self-improvement
- **Hierarchical Reasoning**: Transition from flat step execution to structured decomposition

### Threats
- **Model Hallucination**: Critics/planners may hallucinate if not grounded in actual tool output
- **Security Surface Area**: Enabling `FETCH` without robust sandboxing poses critical vulnerability risk
- **Prompt Drift**: Evolution of underlying LLMs may degrade current principle-based prompts

---

## Essential Commands

### Development
```bash
cargo build
cargo run -- [args]
cargo test
cargo fmt
```

### Testing & Probing
```bash
# Run unit tests
cargo test

# Run behavioral probes
./probe_parsing.sh
./reliability_probe.sh
./run_intention_scenarios.sh
./smoke_llamacpp.sh

# Run stress tests
./run_stress_tests_cli.sh
```

### Architecture Analysis
```bash
# Run the de-bloating analyzer
cd _dev-system/analyzer && cargo run
```

### Configuration Management
```bash
# View current config structure
ls -la config/

# View defaults
ls -la config/defaults/

# Test model-specific override
mv config/<model>/angel_helper.toml /tmp/
cargo run  # Should fall back to defaults
mv /tmp/angel_helper.toml config/<model>/
```

---

## Troubleshooting Quick Reference

### Connection Pool Exhaustion
**Symptom**: Hangs after ~5 HTTP API calls, no timeout errors.
**Root Cause**: Creating `reqwest::Client::new()` in hot paths (each intel unit call).
**Solution**: Pass shared client through `IntelContext`.

### Shell Command Timeouts
**Symptom**: 30-minute timeouts for simple tasks.
**Causes**: 
- Model hangs in retry loops
- Shell syntax issues (process substitution)
- 30-minute timeout too long for practical testing
**Solution**: Reduce to 5-minute timeout, fix shell command syntax

### Terminology Mismatch
**Symptom**: All requests routed to CHAT with entropy=0.00.
**Root Cause**: Model tuned on old terminology (CHAT, SHELL), new terms not recognized.
**Solution**: Revert to original terminology or perform full re-tuning

### Pattern-Matching Routing
**Symptom**: Over-orchestration, keyword-based decisions.
**Root Cause**: Hardcoded word patterns in routing logic.
**Solution**: Use confidence-based fallback (entropy > 0.8 → CHAT)

---

## Future Enhancements

1. **Adaptive Re-tuning**: Re-run JSON tuning if parse errors exceed threshold during normal operation
2. **Per-Difficulty Temperatures**: Use different temperatures for different task complexities
3. **Model-Specific Profiles**: Store and reuse optimal temperatures per model
4. **Continuous Learning**: Update temperature based on runtime JSON success rate
5. **Hierarchical Planning**: Transition from simple step sequences to multi-turn goal persistence
6. **Gated Connectivity**: Maintain `FETCH` in DISABLED state with explicit warnings until sandboxing audit
