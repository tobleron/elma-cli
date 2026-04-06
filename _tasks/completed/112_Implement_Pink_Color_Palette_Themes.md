# Task 112: Implement Pink Color Palette Themes

## Objective
Enhance Elma's visual identity by implementing a selection of modern pink-themed color palettes. This task provides three distinct "looks," with **Rose Pine** as the recommended default for its professional and easy-on-the-eyes aesthetic.

## Palette Options

### Choice 1: Rose Pine (Recommended - "Sophisticated & Moody")
- **Base (Background)**: `#191724` (Deep Charcoal Purple)
- **Rose (Primary Pink)**: `#ebbcba` (Dusty Rose)
- **Love (Accent Pink)**: `#eb6f92` (Strawberry Pink)
- **Gold (Contrast)**: `#f6c177` (Warm Honey)
- **Iris (Subtle)**: `#908caa` (Muted Purple-Grey)

### Choice 2: Catppuccin Mocha ("Vibrant & Modern")
- **Base (Background)**: `#1e1e2e` (Modern Navy-Black)
- **Pink (Primary Pink)**: `#f5c2e7` (Bubblegum Pastel)
- **Flamingo (Soft Pink)**: `#f2cdcd` (Warm Muted Pink)
- **Mauve (Accent)**: `#cba6f7` (Cool Purple)
- **Lavender (Subtle)**: `#b4befe` (Soft Blue-Purple)

### Choice 3: Sakura Night ("High-Contrast Cyber")
- **Void (Background)**: `#0d0d0d` (Pure Black)
- **Neon Sakura (Primary Pink)**: `#ff80bf` (Intense Cherry Blossom)
- **Magenta (Dark Pink)**: `#cc0066` (Deep Magenta)
- **Cyan (Accent)**: `#00ffff` (Electric Cyan)
- **Silver (Text)**: `#e6e6e6` (Bright Silver)

---

## Technical Implementation Plan (Rust)

### 1. Update `src/ui_colors.rs`
Implement TrueColor (24-bit) ANSI helper functions for the chosen palette.
Example for **Rose Pine**:
```rust
pub(crate) fn ansi_rose_pine_rose(s: &str) -> String {
    format!("\x1b[38;2;235;188;186m{s}\x1b[0m")
}

pub(crate) fn ansi_rose_pine_love(s: &str) -> String {
    format!("\x1b[38;2;235;111;146m{s}\x1b[0m")
}

pub(crate) fn ansi_rose_pine_gold(s: &str) -> String {
    format!("\x1b[38;2;246;193;119m{s}\x1b[0m")
}
```

### 2. State-Based Theme Selection
- Update `src/ui_state.rs` to include a `ThemeName` enum: `RosePine`, `Catppuccin`, `SakuraNight`.
- Implement a `get_theme_color(role: ColorRole) -> String` function that returns the appropriate ANSI string based on the active theme in state.

### 3. Integration with `src/ui_chat.rs`
- Replace hardcoded color calls (like `ansi_soft_gold`) with theme-aware calls (like `theme_primary_pink`).
- Ensure the background color is only applied if the terminal supports it, or stick to foreground-only enhancements for maximum compatibility.

### 4. CLI Command to Switch
- Add a temporary internal command or profile setting to switch between the 3 palettes for testing.

## Verification Strategy
1. **Visual Test**: Run Elma and verify the new pink tones are visible and contrast well with the background.
2. **Compatibility**: Verify that on terminals without TrueColor support, the colors degrade gracefully (using standard 16-color fallbacks if necessary).
3. **Consistency**: Ensure all UI elements (Spinners, Status Line, Diffs) respect the selected palette.
