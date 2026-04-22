# 144 Typeahead And Auto-Complete System

## Summary
Implement intelligent typeahead with multiple suggestion types.

## Reference
- Claude Code: `_stress_testing/_claude_code_src/hooks/useTypeahead.tsx` (1152+ lines)

## Implementation

### 1. Trigger System
File: `src/ui/typeahead.rs` (new)
- Detect prefixes: `/` (commands), `@` (files), `!` (shell), `?` (questions)
- Trigger positions: start of input, after whitespace

### 2. Suggestion Types
| Trigger | Source | Example |
|---------|--------|---------|
| `/` | Command registry | `/mode`, `/resume` |
| `@` | File index | `@src/main.rs` |
| `!` | Shell history | `!git status` |
| `?` | FAQ/knowledge | `?how to config` |

### 3. Fuzzy Matching
File: `src/ui/fuzzy.rs` (new)
- Fuzzy match against suggestion sources
- Rank by: match score, recency, frequency
- Return top 5 suggestions

### 4. UI Rendering
File: `src/ui/typeahead_dropdown.rs` (new)
- Dropdown with arrow key navigation
- Ghost text preview
- Tab/Enter to accept

### 5. Integration
File: `src/ui/composer.rs`
- Add typeahead to input handling
- Show suggestions on trigger

## Verification
- [ ] `cargo build` passes
- [ ] Trigger detection works
- [ ] Fuzzy matching returns correct results