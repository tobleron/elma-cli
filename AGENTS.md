# AGENTS.md

This file provides universal guidelines for agents working with code in this repository.

## 🤖 Elma CLI Agent Philosophy

**Elma is a highly autonomous CLI agent engineered to deliver intelligent, reliable assistance through adaptive reasoning and improvisation rather than deterministic rules. Designed for efficiency on local AI models with constrained hardware resources, Elma achieves sophisticated outcomes through minimal reasoning units (intel units), composable planning formulas, and aggressive compression techniques—including evidence summarization and compact knowledge representation—that maximize intelligence per token while minimizing computational overhead. Built as an orchestration layer over high-resolution specialized components, Elma dynamically leverages available knowledge, tools, environment context, and platform capabilities to determine the most effective course of action. The architecture prioritizes accuracy and reliability over speed, empowering Elma to autonomously assess each situation, reason about available options, and execute workflows that genuinely address user objectives—treating classification signals as soft guidance rather than hard constraints, and maintaining the flexibility to improvise solutions that rigid rule-based systems would miss.**

---

## 🧠 Core Protocols

**Context-First Approach:**
1. **ALWAYS READ FIRST**: Start every task by reading `_tasks/TASKS.md` for context.
2. **Architecture Check**: Check `_dev-tasks/` for current de-bloating and structural priorities.
3. **Root-Relative Paths**: All file references must be relative to repository root.

**Commitment Constraint:**
- NEVER commit changes unless explicitly asked to "save", "checkpoint", or "commit".
- Only commit when the user explicitly provides a message or instruction.

**Task Protocol:**
- Follow the exact procedures: Move to `_tasks/active/` → Implement → Verify build (`cargo build`) → Archive.
- **Troubleshoot (T###)**: If a bug is detected, start a T-prefixed task immediately.

## 🛠️ Workflow Automation

### Phase 0: Troubleshooting
- Create `_tasks/active/T###_troubleshoot_[context].md`.
- Document hypothesis, experiment log, and results.
- **Rollback Check**: Ensure any failed experiments are reverted before moving to implementation.

### Phase 1: Implementation & Verification
- Run `cargo build` after significant edits.
- Run `cargo test` and scenario probes (`./run_intention_scenarios.sh`, etc.) to verify behavioral correctness.
- Maintain **Zero Warnings** in all Rust modules.

## 🚨 Coding Vitals (PRIORITY 0)

1. **Rust Orchestration**: Follow idiomatic Rust patterns.
2. **De-bloating Target**: `src/main.rs` is an oversized orchestrator (6.5k LOC). Use `_dev-system` guidance to extract logic into cohesive domain modules.
3. **Configurations**: Model and system configurations live in `config/` as TOML files.
4. **Scenario Integrity**: Verification MUST include running relevant scenarios in `scenarios/`.

## 🧠 System Prompt Design Principles (CRITICAL)

**NEVER hardcode examples or rules in system prompts.** This turns Elma into a rule-matching robot instead of a reasoning agent.

### Wrong Approach (Deterministic Rules):
```
Use INVESTIGATE when:
- "pending tasks" is mentioned
- "recent files" is mentioned
- Multiple sources exist
```

### Right Approach (Principles):
```
Use INVESTIGATE when the model cannot determine what to do without first exploring or clarifying.
```

### Why This Matters:
- **Examples limit reasoning** - Model matches patterns instead of understanding
- **Principles enable reasoning** - Model assesses each situation autonomously
- **Elma's philosophy** is "adaptive reasoning and improvisation rather than deterministic rules"

### Rule of Thumb:
If your prompt says "if X then Y" with specific examples, you're doing it wrong. Instead, explain the **principle** behind when to use Y.

---

## 🚫 CRITICAL: Never Use Word-Based Pattern Matching

**NEVER implement routing/classification using hardcoded word patterns.**

### Wrong Approach (Word-Based Pattern Matching):
```rust
// ❌ WRONG: Hardcoded word patterns
fn is_obvious_chat(input: &str) -> bool {
    if input.starts_with("hello") || input.contains("who are you") {
        return true;  // Forces CHAT route
    }
}
```

**Why this violates Elma's philosophy:**
- Turns Elma into a keyword-matching robot
- Breaks on variations ("Hey there" vs "Hello")
- Cannot handle novel phrasings
- Violates "adaptive reasoning over deterministic rules"

### Right Approach (Confidence-Based Fallback):
```rust
// ✅ RIGHT: Use model's own uncertainty
if route_decision.entropy > 0.8 || route_decision.margin < 0.15 {
    // Model is uncertain → use safe default
    return CHAT;  // Principle: when uncertain, under-execute
}
```

**Why this aligns with Elma's philosophy:**
- Uses model's own confidence metrics
- Principle-based: "when uncertain, be conservative"
- Works on ANY input the model is uncertain about
- Allows model to reason, not match keywords

### Rule of Thumb:
**If you're checking `input.contains("word")` to make routing decisions, you're doing it wrong.**

Instead, use:
- Model confidence (entropy, margin)
- Classification distributions
- Principle-based thresholds

---

## Essential Commands

### Development
```bash
cargo build
cargo run -- [args]
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
```

### Architecture Analysis
```bash
# Run the de-bloating analyzer
cd _dev-system/analyzer && cargo run
```

### Formatting
```bash
cargo fmt
```
