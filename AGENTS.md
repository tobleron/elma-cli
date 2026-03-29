# AGENTS.md

This file provides universal guidelines for agents working with code in this repository.

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
