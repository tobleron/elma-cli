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

**Do not build system prompts around long deterministic rule lists or brittle pattern examples.**  
Prompts must remain principle-first. Minimal examples are allowed only to clarify a boundary when the principle could otherwise be misread.

### Required Prompt Pattern
1. State the governing principle first.
2. Add a short example block only if it sharpens the decision boundary.
3. Use a **4:1 ratio** of representative positive examples to negative edge cases.
4. Keep examples short, canonical, and high-signal.
5. Examples must anchor judgment, not replace reasoning.

### Wrong Approach (Deterministic Rule Dump)
```text
Use INVESTIGATE when:
- "pending tasks" is mentioned
- "recent files" is mentioned
- Multiple sources exist
```

### Right Approach (Principle First, Minimal Boundary Examples)
```text
Use INVESTIGATE when the model cannot determine what to do responsibly without first exploring the workspace, validating evidence, or clarifying missing context.

Examples:
- Need to inspect files before answering a repo-specific question -> INVESTIGATE
- Need to search symbols before making a code claim -> INVESTIGATE
- Need to examine task state before proposing next implementation step -> INVESTIGATE
- Need to gather workspace evidence before confirming a result -> INVESTIGATE

Edge case:
- User asks a self-contained conceptual question that can be answered without workspace inspection -> do not force INVESTIGATE
```

### Critic Prompt Example (Preferred Style)
```text
Principle:
Return retry when a repo-specific claim is not supported by actual workspace evidence in the step results. Return ok when the evidence clearly supports the claim.

Examples:
- The answer claims a file was inspected, but no read/search output exists -> retry
- The answer claims a symbol exists, but no grep/read evidence exists -> retry
- The answer claims a file was edited, but no edit or verification evidence exists -> retry
- The answer claims a test passed, but no test output exists -> retry

Edge case:
- The step results clearly show the relevant file contents or command output and the answer is grounded in that evidence -> ok
```

### Why This Matters
- **Long example lists limit reasoning** - the model starts pattern-matching instead of assessing the situation
- **Principles preserve flexibility** - the model can generalize to novel cases
- **Minimal examples improve calibration** - they clarify the boundary without turning the prompt into a script
- **Elma's philosophy** is adaptive reasoning and improvisation, not rigid rule playback

### Rule of Thumb
If the prompt is mostly examples, exception lists, or "if X then Y" rules, rewrite it.
If the principle is clear but the boundary is fuzzy, add a small example block with a 4:1 positive-to-edge-case ratio.

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
