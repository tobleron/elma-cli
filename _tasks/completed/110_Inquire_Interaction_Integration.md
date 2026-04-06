# Task 110: Deploy Inquire System for Interactive Selection Menus

## Objective
Replace standard text prompts with `inquire`, providing a polished, keyboard-driven interface for selecting models, profiles, and configuration options.

## Technical Implementation Plan

### 1. Selection Wrapper
- Create an `InteractiveUI` module in `src/ui.rs` wrapping `inquire` components.
- Implement a generic `select_from_list<T>(title: &str, options: Vec<T>) -> Option<T>` helper.

### 2. Custom Rendering & Theming
- Configure `inquire`'s `RenderConfig` to use Elma's color palette from `src/ui_colors.rs`.
- Use custom prefixes for prompts (e.g., `?` in soft gold).
- Enable "Vim mode" navigation (j/k) globally.

### 3. Feature-Specific Menus
- **Model Selection**: Implement a fuzzy-searchable list of available models from `src/models_api.rs`.
- **Profile Switching**: An interactive list of profiles from `profiles.toml`.
- **Tool Confirmation**: A "Confirm/Reject" prompt for sensitive shell commands.

### 4. Integration with CLI Flow
- Hook into `src/app_bootstrap_modes.rs` to use interactive selection if a required argument (like `--profile`) is missing but the terminal is TTY.

## Verification Strategy
1. **Usability**: Confirm that typing filters the list in real-time.
2. **Keybindings**: Verify `Up/Down`, `j/k`, `Enter`, and `Esc` work as expected.
3. **TTY Check**: Ensure the system falls back to standard text input if not running in a terminal (e.g., in a pipe).
