# Intel Unit Output Standard

This document defines the standardized output format for all Elma intel units that generate decisions or structured output.

## Design Philosophy

**Compact JSON for efficiency:**
- Minimizes token generation time
- Reduces latency on local models
- Provides high intelligence-per-token ratio
- Enables reliable parsing without complex extraction

---

## Standard Output Format

All intel units use this unified JSON format:

```json
{"choice": "<NUMBER>", "label": "<LABEL>", "reason": "<ULTRA_CONCISE_JUSTIFICATION>", "entropy": <FLOAT>}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `choice` | string | The number corresponding to the chosen option (e.g., "1", "2") |
| `label` | string | The human-readable label (e.g., "CHAT", "WORKFLOW") |
| `reason` | string | Ultra-concise justification for the choice |
| `entropy` | float | Confidence level from 0.0 (certain) to 1.0 (uncertain) |

---

## Output Format Tiers

### Tier 1: Single-Choice Classifiers (64 tokens)

**Use for:** Router, speech act, mode router, and other single-label classifiers

**Intel Units:**
- `router.toml` - Route classification (CHAT vs WORKFLOW)
- `speech_act.toml` - Speech act classification (CHAT vs INSTRUCT vs INQUIRE)
- `mode_router.toml` - Mode classification (INSPECT vs EXECUTE vs PLAN vs MASTERPLAN vs DECIDE)
- `risk_classifier.toml` - Risk level classification
- `complexity_classifier.toml` - Complexity classification

---

### Tier 2: Structured Decisions (180 tokens)

**Use for:** Evidence presentation, command repair, decisions with short rationale

**Intel Units:**
- `evidence_mode.toml` - Evidence gathering strategy
- `command_repair.toml` - Repaired command with fix explanation
- `command_preflight.toml` - Preflight safety check with reason
- `execution_mode_setter.toml` - Execution mode with rationale

---

### Tier 3: Planning & Decomposition (540 tokens)

**Use for:** Multi-step plans, workflow decomposition, strategic planning

**Intel Units:**
- `workflow_planner.toml` - Workflow complexity classification
- `planner.toml` - General planning
- `planner_master.toml` - Master planning
- `decomposition.toml` - Task decomposition

---

## System Prompt Template

All intel unit system prompts should follow this structure:

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

---

## Token Budget Guidelines

| Tier | Max Tokens | Typical Usage | Safety Margin |
|------|------------|---------------|---------------|
| 1 | 64 | ~15-25 actual tokens | 2.5x overhead |
| 2 | 180 | ~40-80 actual tokens | 2.25x overhead |
| 3 | 540 | ~150-300 actual tokens | 1.8x overhead |

**Notes:**
- Models stop naturally when JSON is complete
- `max_tokens` is a ceiling, not a target
- Higher tiers get proportionally less margin (complex output is more predictable)

---

## Entropy Interpretation

| Entropy Range | Meaning | Action |
|---------------|---------|--------|
| 0.0 - 0.3 | Very confident | Proceed without hesitation |
| 0.3 - 0.7 | Moderate confidence | Proceed, consider fallback |
| 0.7 - 1.0 | High uncertainty | Consider retry, alternative route, or user clarification |

---

## Example Intel Unit Configurations

### router.toml (Tier 1)
```toml
system_prompt = """
You are Elma's workflow gate estimator.

Return the most probable answer based on the context in addition to the confidence level from 0 to 1 (entropy) in json format.

Choice rules:
1 = CHAT: the user is greeting, inquiring, or engaging in general conversation
2 = WORKFLOW: the user is instructing Elma to perform a task, action, or investigation

Output format:
{"choice": "<NUMBER>", "label": "<LABEL>", "reason": "<ULTRA_CONCISE_JUSTIFICATION>", "entropy": <FLOAT>}
"""
```

### workflow_planner.toml (Tier 2)
```toml
system_prompt = """
You are Elma's workflow complexity classifier.

Return the most probable answer based on the context in addition to the confidence level from 0 to 1 (entropy) in json format.

Choice rules:
1 = DIRECT: the user request is a simple conversational turn or single action
2 = INVESTIGATE: the user request requires inspecting workspace evidence before proceeding
3 = MULTISTEP: the user request requires multiple sequential actions to complete
4 = OPEN_ENDED: the user request requires strategic planning across multiple phases

Output format:
{"choice": "<NUMBER>", "label": "<LABEL>", "reason": "<ULTRA_CONCISE_JUSTIFICATION>", "entropy": <FLOAT>}
"""
```

### command_repair.toml (Tier 2)
```toml
system_prompt = """
You are Elma's command repair specialist.

Return the most probable answer based on the context in addition to the confidence level from 0 to 1 (entropy) in json format.

Choice rules:
1 = REPAIRED: the command was successfully repaired and is safe to execute
2 = UNREPAIRABLE: the command cannot be safely repaired without changing the task intent

Repair guidelines:
- Fix quoting, globbing, regex, filename casing, or command-shape issues
- Preserve the same task semantics and operation type
- Prefer rg over grep
- Do not introduce network, remote, destructive, or privileged commands

Output format:
{"choice": "<NUMBER>", "label": "<LABEL>", "cmd": "<REPAIRED_COMMAND_OR_NULL>", "reason": "<FIX_EXPLANATION>", "entropy": <FLOAT>}
"""
```

---

## Parsing Implementation

The Rust code in `src/json_parser.rs` provides robust parsing with automatic repair:

### Parsing Pipeline (in order)
1. **Direct JSON** - Clean JSON parsing
2. **Markdown extraction** - Extracts from ` ```json ... ``` ` blocks
3. **Text extraction** - Finds JSON in text with leading/trailing content
4. **JSON repair** - Uses `jsonrepair-rs` to fix malformed JSON
5. **Legacy fallback** - Parses raw digits or labels

### Key Functions
- `parse_intel_output()` - Main parsing function with `IntelParseResult`
- `extract_label()` - Extracts label from JSON (with legacy fallback)
- `extract_entropy()` - Extracts entropy value from JSON
- `extract_reason()` - Extracts reason string from JSON

### Repair Capabilities
The parser automatically repairs:
- Trailing commas: `{"choice": "1",}`
- Single quotes: `{'choice': '1'}`
- Unquoted keys: `{choice: "1"}`
- Missing closing braces (partial repair)
- Other common JSON malformations

All functions gracefully fall back to legacy formats for backward compatibility.

---

## Migration Notes

**From single-digit output to JSON:**
- Old: `1` or `2`
- New: `{"choice": "1", "label": "CHAT", "reason": "...", "entropy": 0.XX}`

**Benefits:**
- Includes confidence measure (entropy) from model directly
- Includes justification (reason) for debugging and transparency
- Structured for reliable parsing
- Consistent across all intel units
- Enables better observability and tuning
