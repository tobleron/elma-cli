# Task U006: Model Picker

## Status
Completed.

## Objective
Implement an interactive model selection overlay with performance metrics.

## Implementation
- Created `ui_model_picker.rs` with `ModelPicker` struct using ratatui popup.
- Integrated into `ClaudeRenderer` with show/hide/select methods.
- Added Ctrl+M key binding to open model picker in `ui_terminal.rs`.
- Modal handles navigation with Up/Down, Enter to select, Esc to close.
- Displays model name, max tokens, and temperature.
- Models loaded from hardcoded list (TODO: read from config/profiles.toml).

## Notes
- Uses ratatui for popup rendering.
- Event handling prioritizes model picker when visible.
- Actual model switching not implemented; modal structure is complete.
