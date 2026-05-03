# Prompt Audit Inventory - Task 450

## Summary

| File | Prompts | Issues Found |
|------|---------|--------------|
| defaults_router.rs | 20 | 3 multi-field JSON, 2 verbose |
| defaults_core.rs | 15 | 4 multi-field JSON, 1 complex step types |
| defaults_evidence.rs | 18 | 2 multi-field JSON, 1 example-heavy |
| defaults_evidence_core.rs | 9 | 2 multi-field JSON |
| **Total** | **62** | **15 flagged** |

---

## Flagged Prompts

### 1. Multi-field JSON (complex schemas)

These request 3+ fields, risky for weak models:

| Prompt | File | Fields | Risk |
|--------|------|--------|------|
| select_items | defaults_router.rs:135 | items[], reason | MEDIUM |
| jsonify | defaults_router.rs:186 | (schema contract) | MEDIUM |
| extract_final | defaults_router.rs:203 | final | LOW |
| calibrate | defaults_router.rs:220 | 5 fields | MEDIUM |
| complexity | defaults_router.rs:237 | complexity, risk | MEDIUM |
| evidence | defaults_router.rs:254 | needs_evidence, needs_tools | MEDIUM |
| decision | defaults_router.rs:271 | needs_decision, needs_plan | MEDIUM |
| pattern | defaults_router.rs:288 | suggested_pattern | LOW |
| formula | defaults_router.rs:305 | primary, alternatives[], reason | MEDIUM |
| scope | defaults_router.rs:339 | scope fields | MEDIUM |
| program_builder | defaults_core.rs:103 | full program | HIGH |
| evaluator | defaults_core.rs:142 | 4+ fields | HIGH |
| synthesizer | defaults_core.rs:361 | full program | HIGH |
| artifact_filter | defaults_evidence_core.rs:53 | 4 arrays | MEDIUM |
| compactor | defaults_evidence_core.rs:36 | summary, key_facts[], noise[] | MEDIUM |

### 2. Verbose/Example-Heavy

| Prompt | File | Issue |
|--------|-------|-------|
| workflow_gate | defaults_router.rs:19 | Long interpretation guide |
| mode_router | defaults_router.rs:35 | Long interpretation guide |
| tooler | defaults_core.rs:86 | Has rule examples |
| shell_safety | defaults_evidence.rs:159 | Example list |

### 3. Multi-Job Prompts

Prompts that combine classification + reasoning:

| Prompt | File | Jobs |
|-------|------|------|
| program_builder | defaults_core.rs:103 | Step creation + type selection |
| evaluator | defaults_core.rs:142 | Evaluation + risk assessment |

---

## Good Prompts (Principle-First Compliant)

Simple single-output prompts:

| Prompt | Output | Complexity |
|--------|--------|-------------|
| intent_classifier | 1 word | ✅ |
| gate | CHAT/ACTION | ✅ |
| gate_why | 1 sentence | ✅ |
| tooler | 1-line JSON | ✅ |
| route | 1 word | ✅ |
| masterplan | Markdown | ✅ |
| summarize | plain text | ✅ |
| reply | plain text | ✅ |
| planning_reason | 1 sentence | ✅ |

---

## Decomposition Status

### Evaluator (critic) - ALREADY DECOMPOSED ✓

The original multi-job `evaluator` prompt is already split into 3 separate reviewers:

| Config | Function | Role |
|--------|----------|------|
| `logical_reviewer_cfg` | `default_logical_reviewer_config()` | Program logic soundness |
| `efficiency_reviewer_cfg` | `default_efficiency_reviewer_config()` | Minimal steps, no redundancy |
| `risk_reviewer_cfg` | `default_risk_reviewer_config()` | Risky command detection |

All three run in `run_staged_reviewers_once()` in `orchestration_loop_verdicts.rs`.

**Finding: No decomposition needed for evaluator prompts.** They are already properly decomposed.

### Remaining Multi-Job Prompt

| Config | Function | Issue |
|--------|----------|-------|
| `orchestrator_cfg` | `default_orchestrator_config()` | Handles 7 step types in one prompt |

The orchestrator (`program_builder` at defaults_core.rs:103) defines:
- shell, reply, plan, masterplan, select, decide, edit

This is a reasonable design choice - the orchestrator must handle all step types to compose workflows.

---

## Recommendation

Task 450 audit reveals:
1. **Evaluator/critic prompts are already well-decomposed** - uses 3 separate reviewers
2. **No actionable changes needed** for evaluator family
3. **program_builder** handles 7 step types - this is intentional design, not a flaw

**Mark task complete?**