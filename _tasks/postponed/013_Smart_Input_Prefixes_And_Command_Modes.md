# 146 Smart Input Prefixes And Command Modes

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
Implement smart input prefixes for different execution modes in the chat input.

## Reference
- Claude Code: Multi-mode input system (`/command`, `!bash`, `@file`, `?question`)

## Implementation

### 1. Input Prefix Parser
File: `src/input_parser.rs` (new)
```rust
pub enum InputMode {
    Chat(String),           // Normal chat
    Command(String),        // /command arg
    Shell(String),         // !shell command
    File(String),           // @file path
    Question(String),      // ?search query
}

pub fn parse_input(input: &str) -> InputMode { ... }
```

### 2. Prefix Handlers
| Prefix | Mode | Behavior |
|--------|------|----------|
| (none) | Chat | Normal message |
| `/` | Command | Slash command |
| `!` | Shell | Direct shell execution |
| `@` | File | File context injection |
| `?` | Question | Web search / FAQ |

### 3. Command Registry
File: `src/commands.rs`
- Register all slash commands
- Add help (`/help` lists commands)
- Argument parsing

### 4. Shell Execution Mode
File: `src/execution.rs`
- Execute `!` prefix directly in shell
- Return output as response
- Uses existing shell tool

### 5. File Context Mode
File: `src/context_injection.rs`
- Read `@filename` and inject as context
- Fuzzy file search
- Include file contents in prompt

## Verification
- [ ] `cargo build` passes
- [ ] All prefixes parse correctly
- [ ] Shell execution works