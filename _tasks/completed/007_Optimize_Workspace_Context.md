# Task 044: Optimize Workspace Context Generation

## Context
Current workspace context uses basic `find` and `ls` commands that produce verbose, unstructured output. This wastes tokens and makes it harder for the model to understand project structure.

## Problem
Current workspace context:
```
top_level: .DS_Store, .cargo, AGENTS.md, Cargo.lock, Cargo.toml, ...
src_files: app.rs, app_bootstrap.rs, app_bootstrap_core.rs, ...
```

Issues:
- No hierarchy/structure shown
- No file sizes or types
- No indication of which files are important
- Wastes tokens on irrelevant files (.DS_Store, etc.)

## Objective
Replace basic workspace context with structured tree view that:
- Shows directory hierarchy (3 levels deep)
- Filters noise (.DS_Store, target/, .git/, etc.)
- Highlights important files (Cargo.toml, package.json, etc.)
- Uses efficient Rust-based tree traversal

## Implementation Options

### Option A: Use `tree` command (if available)
```bash
tree -L 3 -I 'target|.git|node_modules' --dirsfirst
```
**Pros:** Simple, well-tested
**Cons:** External dependency, may not be installed

### Option B: Use Rust crate `ignore` + custom tree
```rust
use ignore::WalkBuilder;

fn build_workspace_tree(root: &Path, max_depth: usize) -> Result<String> {
    let mut walker = WalkBuilder::new(root)
        .max_depth(Some(max_depth))
        .hidden(true)  // Skip hidden files
        .filter_entry(|entry| {
            !is_ignored_dir(entry.path())
        })
        .build();
    // Build tree structure...
}
```
**Pros:** No external dependency, fast, customizable
**Cons:** More code to maintain

### Option C: Use Rust crate `fs_extra` or `walkdir`
Similar to Option B but with different crate.

## Recommended: Option B (ignore crate)

The `ignore` crate is:
- Already used by ripgrep (which Elma uses)
- Fast and memory-efficient
- Respects .gitignore automatically
- Well-maintained

## Implementation Steps

1. **Add dependency** to `Cargo.toml`:
   ```toml
   [dependencies]
   ignore = "0.4"
   ```

2. **Create workspace tree module** `src/workspace_tree.rs`:
   ```rust
   pub struct WorkspaceTree {
       pub root: PathBuf,
       pub max_depth: usize,
       pub ignore_patterns: Vec<String>,
   }

   impl WorkspaceTree {
       pub fn new(root: &Path) -> Self;
       pub fn with_max_depth(self, depth: usize) -> Self;
       pub fn build(&self) -> Result<String>;
   }

   fn is_ignored_dir(path: &Path) -> bool {
       matches!(path.file_name().and_then(|n| n.to_str()), Some(
           "target" | "node_modules" | ".git" | ".cargo" | 
           "dist" | "build" | "__pycache__" | ".venv"
       ))
   }
   ```

3. **Update workspace context generation** in `src/workspace.rs`:
   ```rust
   pub(crate) fn gather_workspace_brief(repo: &Path) -> String {
       let tree = WorkspaceTree::new(repo)
           .with_max_depth(3)
           .build()
           .unwrap_or_else(|_| gather_workspace_brief_fallback(repo));
       tree
   }
   ```

4. **Add fallback** for when tree building fails:
   ```rust
   fn gather_workspace_brief_fallback(repo: &Path) -> String {
       // Current basic implementation
   }
   ```

5. **Format output** for model consumption:
   ```
   Project Structure (3 levels):
   ├── Cargo.toml
   ├── AGENTS.md
   ├── src/
   │   ├── main.rs
   │   ├── app.rs
   │   ├── orchestration/
   │   │   ├── mod.rs
   │   │   └── loop.rs
   │   └── ...
   ├── config/
   │   └── ...
   └── tests/
       └── ...
   ```

## Acceptance Criteria
- [ ] Workspace tree shows 3 levels of hierarchy
- [ ] Ignores common noise directories (target, node_modules, .git, etc.)
- [ ] Uses efficient Rust-based traversal (ignore crate)
- [ ] Falls back to basic listing if tree fails
- [ ] Output is formatted for easy model consumption
- [ ] Token usage reduced by >30% for large projects

## Files to Create
- `src/workspace_tree.rs` - Tree generation module

## Files to Modify
- `src/workspace.rs` - Use tree instead of basic listing
- `Cargo.toml` - Add ignore crate dependency
- `src/main.rs` - Add workspace_tree module

## Priority
HIGH - Improves all downstream reasoning by providing better context

## Dependencies
- None blocking

## Expected Impact
- **Better project understanding** - Model sees structure, not just file list
- **Fewer exploration steps** - Model knows where to look
- **Reduced tokens** - Structured output is more compact
- **Faster execution** - Less guessing about project layout
