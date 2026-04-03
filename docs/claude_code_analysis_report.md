# Analysis Report: Claude Code Source Code & Potential Improvements for Elma-CLI

> **See [ARCHITECTURE.md](./ARCHITECTURE.md)** for comprehensive documentation including design philosophy, GBNF grammar integration, JSON reliability pipeline, and full implementation details.
> **See [IMPLEMENTATION_NOTES.md](./IMPLEMENTATION_NOTES.md)** for recent progress, troubleshooting sessions, and current state.

---

## 7 Major Categories

### **Finding: Tool Search & Discovery (`ToolSearchTool`)**
Claude Code uses a `ToolSearchTool` to manage a large ecosystem of tools. Instead of including all tool schemas in the system prompt (which consumes tokens and causes confusion), it only includes "deferred" tool names and search hints. The model can then use `ToolSearchTool` to find and load the necessary tools dynamically.

**Recommendation for Elma-CLI:**
- Implement a similar `ToolSearchTool`.
- Add `searchHint` (3-10 word capability phrases) to `elma-cli` tool definitions.
- Transition from a static tool set to a dynamic, searchable tool registry.

### **Finding: Granular Tool Control Flags**
Claude Code tools include flags such as `isReadOnly`, `isDestructive`, `isConcurrencySafe`, and `interruptBehavior`. These provide the orchestration layer with critical information to handle permissions and execution safety.

**Recommendation for Elma-CLI:**
- Update the `Step` or `Tool` traits in Rust to include these semantic flags.
- Use `isDestructive` to trigger mandatory user confirmation regardless of the current permission mode.

### **Finding: Automatic Large Result Persistence**
Tools like `FileReadTool` in Claude Code have a `maxResultSizeChars` property. If a tool's output exceeds this limit, it is automatically saved to a temporary file, and the model receives a path instead of the full content. This prevents context window flooding.

**Recommendation for Elma-CLI:**
- Implement a result-capping mechanism in the `orchestration_loop.rs`.
- Automatically offload large outputs to `_dev-system/tmp` and provide the model with a `read_file` instruction to access specific parts if needed.

---

## 🛡️ 2. Enhanced Shell Security & Robustness

### **Finding: Deep Shell Command Validation (`BashTool`)**
Claude Code's `BashTool` includes extremely rigorous security checks (e.g., `bashSecurity.ts`, `bashPermissions.ts`). It detects:
- Dangerous Zsh-specific commands (`zmodload`, `emulate`, etc.).
- Malformed token injections and shell metacharacter tricks.
- Obfuscated flags and IFS (Internal Field Separator) injection.
- Unescaped backticks and command substitutions in unexpected places.

**Recommendation for Elma-CLI:**
- Move beyond simple "allowed command" lists.
- Integrate a robust shell parser (like `tree-sitter-bash`) to perform semantic analysis of commands before execution.
- Implement more aggressive pattern matching for shell-level escapes and injection attacks.

---

## 🤖 3. Specialized Sub-Agents & Delegation

### **Finding: High-Isolation Sub-Agents (`AgentTool`)**
Claude Code's `AgentTool` supports spawning sub-agents with "isolation" modes like `worktree`. This creates a temporary git worktree so the sub-agent can work on an isolated copy of the repository without interfering with the main workspace.

**Recommendation for Elma-CLI:**
- Enhance the current sub-agent delegation to use git worktrees for complex refactoring tasks.
- Implement `fork` and `resume` capabilities for sub-agents to allow long-running background tasks.

### **Finding: Background Task Management**
Claude Code has a dedicated system for managing background tasks (`TaskCreateTool`, `TaskListTool`, etc.), allowing the agent to kick off long-running operations (like builds or tests) and continue with other work.

**Recommendation for Elma-CLI:**
- Formally implement a "Background Task" primitive that can be monitored and managed by the main orchestration loop.

---

## 🧠 4. Context Management & Compaction

### **Finding: History Compaction ("Snip")**
Claude Code uses a "Snip" mechanism (`HISTORY_SNIP`) to identify logical boundaries and compact the message history. This prevents the token count from growing indefinitely in long-running sessions.

**Recommendation for Elma-CLI:**
- Implement a similar compaction mechanism in `src/app_chat_core.rs`.
- When the history reaches a certain threshold, summarize previous turns into a single "Context Snapshot" message.

### **Finding: Tiered Memory Layers (`CLAUDE.md`, `CLAUDE.local.md`)**
Claude Code uses `CLAUDE.md` for project conventions and `CLAUDE.local.md` for personal user preferences. This separates concerns and ensures that the agent is both project-aware and user-aware.

**Recommendation for Elma-CLI:**
- Adopt this standard. Check for these files at startup and inject their content into the system prompt.
- Use `auto-memory` for transient learning and promote useful patterns to these persistent files.

---

## 🛠️ 5. Reusable "Skills"

### **Finding: Bundled Skills (`SkillTool`)**
Claude Code allows bundling high-level logic (prompts + instructions) as "Skills" (e.g., `stuck.ts` for diagnosis, `remember.ts` for memory review). These are more than tools—they are specialized mini-agents.

**Recommendation for Elma-CLI:**
- Implement a `SkillTool` that can load and execute reusable "Reasoning Formulas" or "Specialized Prompts" stored in a dedicated `skills/` directory.

---

## 🩺 6. Diagnostic & Self-Healing

### **Finding: Self-Diagnostic Tools (`stuck.ts`)**
Claude Code includes a `stuck` skill that allows the agent to diagnose its own frozen or slow processes using system-level commands like `ps`, `pgrep`, and `sample`.

**Recommendation for Elma-CLI:**
- Add a `diagnose_self` tool that can check for CPU spikes, memory leaks, or hung subprocesses and report back to the user or attempt a restart/recovery.

---

## 🎨 7. UI & Experience (CLI)

### **Finding: Rich Interactive Feedback**
Claude Code uses `Ink` (React for CLI) to provide grouped tool use rendering, real-time progress bars, and beautifully formatted activity descriptions (e.g., "Searching for pattern...").

**Recommendation for Elma-CLI:**
- While maintaining the Rust-based core, improve the terminal output with more interactive components (spinners, progress bars, and collapsed result views) to make the agent's actions more transparent and less noisy.

---

## 📚 Quick Links

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Comprehensive reference documentation
- **[TASKS.md](./TASKS.md)** - Complete task list by pillar
- **[IMPLEMENTATION_NOTES.md](./IMPLEMENTATION_NOTES.md)** - Recent progress & troubleshooting
- **[INTEL_UNIT_STANDARD.md](./INTEL_UNIT_STANDARD.md)** - Intel unit output format standard

---

## 🚀 Essential Commands

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

### Troubleshooting Quick Reference

**Connection Pool Exhaustion:**
- Symptom: Hangs after ~5 HTTP API calls, no timeout errors.
- Root Cause: Creating `reqwest::Client::new()` in hot paths (each intel unit call).
- Solution: Pass shared client through `IntelContext`.

**Shell Command Timeouts:**
- Symptom: 30-minute timeouts for simple tasks.
- Causes: Model hangs in retry loops, shell syntax issues, 30-minute timeout too long.
- Solution: Reduce to 5-minute timeout, fix shell command syntax.

**Terminology Mismatch:**
- Symptom: All requests routed to CHAT with entropy=0.00.
- Root Cause: Model tuned on old terminology (CHAT, SHELL), new terms not recognized.
- Solution: Revert to original terminology or perform full re-tuning.

**Pattern-Matching Routing:**
- Symptom: Over-orchestration, keyword-based decisions.
- Root Cause: Hardcoded word patterns in routing logic.
- Solution: Use confidence-based fallback (entropy > 0.8 → CHAT).
