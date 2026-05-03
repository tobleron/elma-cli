# Task 500: Workspace as Discoverable Info Tool — Remove Static System Prompt Injection

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 2-3 days
**Primary surfaces:** `elma-tools/src/tools/workspace_info.rs` (new), `src/tool_calling.rs`, `src/prompt_core.rs`, `src/orchestration_core.rs`, `elma-tools/src/tools/mod.rs`
**Depends on:** None (can be done in parallel with Task 499)
**Related tasks:** Task 193 (project guidance loader), Task 200 (branded splash and compact header), Task 313 (deterministic tool filtering and stable ordering)

## Objective

Remove workspace path, directory tree, and project guidance from the static system prompt injection. Replace them with a `workspace_info` tool that the model calls on demand when it needs to understand its environment.

Currently, `build_tool_calling_system_prompt()` at `src/orchestration_core.rs:24` pumps the full workspace path, directory tree, project guidance, and file tree into every single system prompt. For a 4B model with 8K context window, this is 500-2000 tokens wasted on every turn — tokens the model could use for actual reasoning and tool output.

After this change, the system prompt is ~60 tokens (the core prompt) plus conversation history. The model discovers its workspace through a tool, exactly like it discovers extra capabilities through `tool_search`.

## Current State

### System prompt assembly

`src/orchestration_core.rs:24-68` — `build_tool_calling_system_prompt()`:
- Line 25: `let workspace_facts = runtime.ws.trim();` — full workspace path
- Line 27: `let workspace_brief = runtime.ws_brief.trim();` — directory tree listing
- Line 59: `let project_guidance = runtime.guidance.render_for_system_prompt();` — AGENTS.md, TASKS.md, active master task (up to 4000 chars, trimmed to 1600/1200 per section)
- Line 61: calls `assemble_system_prompt(workspace_facts, workspace_brief, &conversation, &skill_context, &project_guidance)`

`src/prompt_core.rs:70-124` — `assemble_system_prompt()`:
- Line 78-79: injects `## Workspace\n{workspace_facts}` into SILENT_METADATA
- Line 81-82: injects `## File tree\n{workspace_brief}` into SILENT_METADATA
- Line 91-95: injects `## Project guidance\n{project_guidance}` into SILENT_METADATA
- Line 104-123: wraps everything in SILENT_METADATA tags

### Who provides workspace info

`src/app.rs:59` — `AppRuntime.ctx_max`, `AppRuntime.ws`, `AppRuntime.ws_brief`:
- `ws` comes from `app_bootstrap_core.rs` — full path of the working directory
- `ws_brief` comes from `workspace_tree.rs` — directory tree listing

Project guidance from `src/project_guidance.rs:44` — `load_project_guidance()`:
- Reads `AGENTS.md` (trimmed to 1600 chars)
- Reads `_tasks/TASKS.md` (trimmed to 1200 chars)
- Finds active master task file in `_tasks/active/` (trimmed to 1200 chars)

### Tool registration pattern

Tools are registered in `elma-tools/src/tools/mod.rs:32` via `register_all()`:
```rust
pub(crate) fn register_all(builder: &mut crate::registry::RegistryBuilder) {
    ls::register(builder);
    read::register(builder);
    // ... 30+ more
}
```

Each tool file follows this pattern (from `elma-tools/src/tools/ls.rs`):
```rust
pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new("tool_name", "description", json_schema, keywords)
            .not_deferred()
            .with_implementation(ImplementationKind::RustNative)
            .with_risks(vec![ToolRisk::ReadOnly])
            .with_executor_state(ExecutorState::PureRust)
            .concurrency_safe(true),
    );
}
```

Tool execution lives in `src/tool_calling.rs:76` — a `match tool_name.as_str()` block dispatches to executor functions.

## Implementation Plan

### Step 1: Register `workspace_info` tool in elma-tools

**File:** `elma-tools/src/tools/workspace_info.rs` (NEW)

```rust
use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "workspace_info",
            "Get information about the current workspace: root path, directory structure, project type (Cargo.toml, package.json, etc.), git status, and any active project guidance documents (AGENTS.md, _tasks/TASKS.md). Use this to understand where you are and what kind of project you're working in. Call this early when you need to read files, run commands, or understand the project structure.",
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            vec![
                "workspace",
                "project info",
                "what directory am I in",
                "project structure",
                "repo info",
                "working directory",
                "current project",
                "project type",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
```

**File:** `elma-tools/src/tools/mod.rs`

1. Add `mod workspace_info;` to the module declarations (after `write` at line 30)
2. Add `workspace_info::register(builder);` to `register_all()` (after `write::register(builder)` at line 62)

### Step 2: Implement executor in tool_calling.rs

**File:** `src/tool_calling.rs`

Add match arm at line 76 (in the `match tool_name.as_str()` block, after `"observe"`):

```rust
"workspace_info" => exec_workspace_info(workdir, runtime, &call_id, tui),
```

Note: This executor needs access to `AppRuntime` to read `ws`, `ws_brief`, and `guidance`. The current `execute_tool_call` signature at line 46 does not take a runtime parameter. We need to either:

**Option A**: Add an `&AppRuntime` parameter to `execute_tool_call()` (cleanest)
**Option B**: Pass workspace info as additional parameters (noisy)
**Option C**: Use a global/thread-local for workspace state (tight coupling)

**Recommend Option A.** The function already takes `workdir` as `&PathBuf`. Change signature:

```rust
pub(crate) async fn execute_tool_call(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    workdir: &PathBuf,
    session: &SessionPaths,
    tool_call: &ToolCall,
    intent: &str,
    tui: Option<&mut crate::ui_terminal::TerminalUI>,
    // NEW: workspace state for workspace_info tool
    workspace_state: Option<&crate::app::AppRuntime>,
) -> ToolExecutionResult {
```

Actually, let's keep it simpler. The executor just needs three strings: workspace path, directory tree, and project guidance. We can pass those directly without coupling to AppRuntime:

Add a new struct:

```rust
/// Snapshot of workspace state needed by workspace_info tool.
pub(crate) struct WorkspaceState {
    pub root_path: String,
    pub directory_tree: String,
    pub project_guidance: String,
}
```

Pass `Option<&WorkspaceState>` to `execute_tool_call`. This is cleaner — the tool_calling module doesn't need to know about AppRuntime.

But wait — this breaks the existing callers of `execute_tool_call`. Let me check all call sites... The function is called from `tool_loop.rs:783` and `tool_loop.rs:987`. Adding a parameter requires updating these call sites.

Actually, let me think about this differently. The executor can read workspace info from `workdir` (which it already has) plus a few helpers. It doesn't need AppRuntime. The workspace path is `workdir.display()`. The directory tree can be generated on-the-fly by reading the filesystem. The project guidance can be read from disk on-demand. This is fully self-contained:

```rust
fn exec_workspace_info(
    workdir: &PathBuf,
    call_id: &str,
    _tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let mut info = String::new();

    // 1. Workspace root path
    info.push_str(&format!("## Workspace Root\n{}\n\n", workdir.display()));

    // 2. Directory structure (top-level + 1 level deep, max 100 entries)
    info.push_str("## Directory Structure\n");
    if let Ok(entries) = std::fs::read_dir(workdir) {
        let mut items: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // Skip hidden/common noise
            if name.starts_with('.') || name == "target" || name == "node_modules"
                || name == "dist" || name == "build"
            {
                continue;
            }
            let marker = if path.is_dir() { "/" } else { "" };
            // For directories, show one level deep
            if path.is_dir() {
                let mut sub_items = String::new();
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    let mut subs: Vec<String> = sub_entries
                        .flatten()
                        .filter_map(|e| {
                            let sp = e.path();
                            let sn = sp.file_name()?.to_string_lossy().to_string();
                            if sn.starts_with('.') { return None; }
                            let sm = if sp.is_dir() { "/" } else { "" };
                            Some(format!("    {}{}", sn, sm))
                        })
                        .take(20) // max 20 sub-items per directory
                        .collect();
                    subs.sort();
                    if !subs.is_empty() {
                        sub_items = format!("\n{}", subs.join("\n"));
                    }
                }
                items.push(format!("  {}{}{}", name, marker, sub_items));
            } else {
                items.push(format!("  {}{}", name, marker));
            }
            if items.len() >= 100 { break; }
        }
        items.sort();
        info.push_str(&items.join("\n"));
    }
    info.push_str("\n\n");

    // 3. Project type detection
    info.push_str("## Project Type\n");
    let checks: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust"),
        ("package.json", "Node.js/JavaScript/TypeScript"),
        ("pyproject.toml", "Python"),
        ("setup.py", "Python"),
        ("go.mod", "Go"),
        ("Makefile", "Make-based project"),
        ("CMakeLists.txt", "CMake/C++"),
        ("Gemfile", "Ruby"),
        ("composer.json", "PHP"),
        ("pom.xml", "Java/Maven"),
        ("build.gradle", "Java/Gradle"),
        ("requirements.txt", "Python"),
        ("Dockerfile", "Docker container"),
        ("docker-compose.yml", "Docker Compose"),
        (".github/workflows", "GitHub Actions CI"),
    ];
    let mut found = false;
    for (file, label) in checks {
        if workdir.join(file).exists() {
            info.push_str(&format!("- {} ({})\n", label, file));
            found = true;
        }
    }
    if !found {
        info.push_str("- Generic (no recognized project file)\n");
    }

    // 4. Git status summary
    if workdir.join(".git").exists() {
        info.push_str("\n## Git Status\n");
        let branch = std::process::Command::new("git")
            .args(["-C", &workdir.display().to_string(), "branch", "--show-current"])
            .output();
        if let Ok(out) = branch {
            let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !b.is_empty() {
                info.push_str(&format!("Branch: {}\n", b));
            }
        }
        let status = std::process::Command::new("git")
            .args(["-C", &workdir.display().to_string(), "status", "--short"])
            .output();
        if let Ok(out) = status {
            let text = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if lines.is_empty() {
                info.push_str("Working tree clean\n");
            } else {
                let modified = lines.iter().filter(|l| l.starts_with(' M') || l.starts_with("M ")).count();
                let untracked = lines.iter().filter(|l| l.starts_with("??")).count();
                let staged = lines.iter().filter(|l| l.starts_with("M ") || l.starts_with("A ")).count();
                info.push_str(&format!(
                    "{} staged, {} modified, {} untracked files\n",
                    staged, modified, untracked
                ));
                // Show first 20 changes
                info.push_str("Recent changes:\n");
                for line in lines.iter().take(20) {
                    info.push_str(&format!("  {}\n", line));
                }
                if lines.len() > 20 {
                    info.push_str(&format!("  ... and {} more\n", lines.len() - 20));
                }
            }
        }
    }

    // 5. Project guidance documents (if they exist)
    let guidance_files = [
        ("AGENTS.md", 1600usize),
        ("_tasks/TASKS.md", 1200),
    ];
    let mut guidance_section = String::new();
    for (rel_path, max_chars) in &guidance_files {
        let full_path = workdir.join(rel_path);
        if full_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let trimmed: String = content.chars().take(*max_chars).collect();
                guidance_section.push_str(&format!(
                    "\n### {}\n```\n{}\n```\n",
                    rel_path, trimmed
                ));
                if content.chars().count() > *max_chars {
                    guidance_section.push_str("...(truncated)\n");
                }
            }
        }
    }
    // Check for active master task
    let active_dir = workdir.join("_tasks").join("active");
    if active_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&active_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let trimmed: String = content.chars().take(800).collect();
                        let name = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        guidance_section.push_str(&format!(
                            "\n### Active task: {}\n```\n{}\n```\n",
                            name, trimmed
                        ));
                    }
                    break; // only show first active task
                }
            }
        }
    }
    if !guidance_section.is_empty() {
        info.push_str("\n## Project Guidance\n");
        info.push_str(&guidance_section);
    }

    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "workspace_info".to_string(),
        content: info,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}
```

Key design decisions:
- Self-contained: reads everything from disk using `workdir` (already available in executor)
- No new parameters needed on `execute_tool_call()` — works with existing signature
- Directory listing is limited to 100 top-level + 20 sub-items per directory to keep output reasonable
- Project guidance trimmed to same limits as current static injection (1600/1200 chars)
- Git status shows branch + summary counts (not full diff) to keep output small
- Total output typically 500-2000 tokens — similar to what was in the system prompt, but only sent when the model asks for it

### Step 3: Remove workspace from system prompt assembly

**File:** `src/orchestration_core.rs`

Modify `build_tool_calling_system_prompt()` at line 24:

1. **Remove** lines 25-27 (workspace_facts and workspace_brief extraction)
2. **Remove** line 59 (project_guidance)
3. Change the `assemble_system_prompt()` call at line 61 from:
   ```rust
   crate::prompt_core::assemble_system_prompt(
       workspace_facts, workspace_brief, &conversation, &skill_context, &project_guidance,
   )
   ```
   to:
   ```rust
   crate::prompt_core::assemble_system_prompt(
       &conversation, &skill_context,
   )
   ```

**File:** `src/prompt_core.rs`

Modify `assemble_system_prompt()` at line 70:

1. **Change signature** from:
   ```rust
   pub fn assemble_system_prompt(
       workspace_facts: &str,
       workspace_brief: &str,
       conversation: &str,
       skill_context: &str,
       project_guidance: &str,
   ) -> String {
   ```
   to:
   ```rust
   pub fn assemble_system_prompt(
       conversation: &str,
       skill_context: &str,
   ) -> String {
   ```

2. **Remove** lines 78-83 (the `## Workspace` and `## File tree` blocks):
   ```rust
   // REMOVE THESE LINES:
   if !workspace_facts.is_empty() {
       metadata.push_str(&format!("\n## Workspace\n{}\n", workspace_facts));
   }
   if !workspace_brief.is_empty() {
       metadata.push_str(&format!("\n## File tree\n{}\n", workspace_brief));
   }
   ```

3. **Remove** lines 91-95 (the `## Project guidance` block):
   ```rust
   // REMOVE THESE LINES:
   if !project_guidance.is_empty() {
       metadata.push_str(&format!(
           "\n## Project guidance\n{}\n",
           project_guidance
       ));
   }
   ```

4. **Update doc comment** on line 63:
   ```
   /// Assemble the system prompt from the core prompt plus conversation context
   /// and skill context. Workspace info and project guidance are available via
   /// the `workspace_info` tool — the model discovers them on demand rather than
   /// having them statically injected.
   ```

5. **Update callers**: Find all callers of `assemble_system_prompt()` and update to the new 2-parameter signature:
   - `src/orchestration_core.rs:61` — already updated above
   - Run `rg "assemble_system_prompt" src/` to find any other callers

### Step 4: Update TOOL_CALLING_SYSTEM_PROMPT to reference workspace_info

**File:** `src/prompt_core.rs`

Update `TOOL_CALLING_SYSTEM_PROMPT` constant at line 43. The current prompt is on lines 43-56. Replace with:

```rust
pub const TOOL_CALLING_SYSTEM_PROMPT: &str = "\
You are Elma, a local-first terminal agent.

Understand the user's request and take action. Deliver direct answers for conversational queries. Use tools to gather evidence for factual requests.

Tool workflow:
1. Call workspace_info to discover where you are and what project you're working in
2. Discover extra capabilities with tool_search
3. Execute commands: shell (terminal), read (view files), search (ripgrep), glob (file patterns), ls (directory tree), fetch (web), write (create), edit (modify), patch (multi-file), update_todo_list (tasks)
4. Use respond for interim status updates (loops)
5. Use summary when you have enough evidence that the user request, inquiry, or task is resolved and accomplished

Prefer `rg` for text search and file listing — it respects .gitignore and skips hidden files automatically.

Begin with the most direct source of truth. Collect evidence until you have sufficient information. Ground all answers in tool output.";
```

Changes from current prompt:
- Line 1 (was line 49): Added `Call workspace_info...` as step 1 before tool_search
- Renumbered subsequent steps
- Removed mention of `observe` (not actually a listed tool in the workflow)
- Total token count: approximately 100 tokens (up from ~60), still very lean

### Step 5: Update build-time prompt hash

**File:** `src/prompt_core.rs`, test `test_prompt_unchanged` at line 144

1. Run the test to get the failure message with the new expected hash:
   ```bash
   cargo test test_prompt_unchanged
   ```
2. Copy the new hash from the failure message
3. Update the assertion in the test

### Step 6: Verify SILENT_METADATA section handling

The `assemble_system_prompt()` function at line 104 checks `if metadata.is_empty()` — if we remove workspace and project guidance but `conversation` or `skill_context` is still present, the SILENT_METADATA wrapper is still generated. This is correct behavior.

If both `conversation` and `skill_context` are empty (first turn of a fresh session), no SILENT_METADATA is generated. The prompt is just `TOOL_CALLING_SYSTEM_PROMPT` + mode instructions. This is also correct — the simplest possible prompt for the simplest case.

## Acceptance Criteria

1. `cargo build` compiles with no errors (both `elma-cli` and `elma-tools`)
2. `cargo test -p elma-tools` passes
3. `cargo test test_prompt_unchanged` passes with updated hash
4. `workspace_info` appears in `build_current_tools()` output
5. System prompt no longer contains workspace path, file tree, or project guidance in SILENT_METADATA
6. `workspace_info` executor returns valid structured output when called
7. Manual test: start Elma in a project directory, ask "what project is this?" — model should call `workspace_info` and correctly identify the project type
8. Manual test: ask a question that requires reading files — model should call `workspace_info` before or during file operations to discover project structure

## Risk Assessment

- **Model might not call workspace_info**: Some models may not intuitively know to call a discovery tool. Mitigation: the system prompt explicitly lists it as step 1. If a model skips it and tries to read a non-existent file, the error message should guide it back.
- **First-turn latency**: The model now needs one extra tool call on first turn. This is a tradeoff: one extra round-trip for ~500-2000 tokens saved on every subsequent turn. For multi-turn sessions, this is a net win.
- **No workspace context in first assistant message**: If the model doesn't call workspace_info before responding, its first response won't have workspace context. This is acceptable — the model will call it when needed, same as `tool_search`.
- **Regression in file path resolution**: The model was previously told the full workspace path directly in the system prompt. Now it discovers it via tool output. File reads should still work because `workdir` is the base for all path resolution in the tool executors, not the model's prompt.
