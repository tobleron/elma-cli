# Task 307: Tokenized Theme Enforcement

**Status:** completed
**References:** Directive 008

## Objective

Replace all hardcoded color constants outside the theme module with canonical theme token calls. No `Color::Red`, `Color::Green`, `RED`, `GREEN`, `BLUE`, `YELLOW` should appear outside `src/ui/ui_theme.rs`.

## Scope

1. **`src/ui/ui_render_legacy.rs`**: Replace all hardcoded color constants (RED, GREEN, YELLOW, BLUE, SYSTEM_YELLOW) with `current_theme()` tokens
2. **`src/ui/ui_markdown.rs`**: Replace hardcoded `BLUE` with theme token
3. **`src/ui/ui_modal.rs`**: Replace hardcoded YELLOW, BLUE, GREEN, RED with theme tokens
4. **`src/ui/ui_terminal.rs`**: Replace all `Color::Yellow`, `Color::Cyan`, `Color::Green`, etc. with theme tokens
5. **`src/ui/ui_diff.rs`**: Replace `Color::Red` and `Color::Green` with theme tokens
6. Verify with grep that `Color::[A-Z]` and hardcoded color constants don't appear outside theme module

## Verification

```bash
cargo build
cargo test
# grep for hardcoded colors - should return zero outside theme module
rg 'Color::(Red|Green|Blue|Yellow|Cyan|Magenta|White|Black)' src/ --type rust
```
