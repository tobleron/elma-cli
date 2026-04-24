# 134 Mode System Type Definitions And Defaults

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Summary
Implement mode system inspired by Roo-Code's 5 built-in modes (Architect, Code, Ask, Debug, Orchestrator). Each mode defines allowed tools and system prompt behavior.

## Reference
- Roo-Code: `~/Roo-Code/packages/types/src/mode.ts`
- Roo-Code: `~/Roo-Code/src/shared/modes.ts`

## Implementation

### 1. Create Mode Type Definitions
File: `src/mode.rs` (new)
- Define `Mode` enum with variants: Architect, Code, Ask, Debug, Orchestrator, Custom(String)
- Define `ModeToolConfig` - which tools each mode allows
- Define `ModeSystemPrompt` - mode-specific system prompt additions

### 2. Define Default Mode Tool Sets
| Mode | Allowed Tools |
|------|-------------|
| Architect | read, search, mcp |
| Code | read, edit, shell, mcp |
| Ask | read, search, mcp |
| Debug | read, edit, shell, search, mcp |
| Orchestrator | (delegates to sub-modes) |

### 3. Create Mode Profiles
File: `config/defaults/mode_*.toml`
- `mode_architect.toml` - planning-focused prompts
- `mode_code.toml` - coding-focused prompts  
- `mode_ask.toml` - Q&A prompts
- `mode_debug.toml` - debugging-focused prompts
- `mode_orchestrator.toml` - delegation prompts

## Verification
- [ ] `cargo build` passes
- [ ] Mode enum variants compile
- [ ] Default profiles load correctly