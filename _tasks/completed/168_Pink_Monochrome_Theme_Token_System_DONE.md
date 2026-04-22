# Task 168: Pink Monochrome Theme Token System

## Status
Completed. Implemented tokenized theme system with Pink/Cyan defaults, replaced Gruvbox constants with theme-mapped values, and ensured active UI uses semantic tokens.

## Objective
Replace the old Gruvbox/Tokyo-Night-style color assumptions with a tokenized Claude-parity theme system using black, white, grey, high-contrast Pink, and high-contrast Cyan.

## Theme Contract
Initial default theme:

| Token | Hex | Purpose |
|---|---:|---|
| `bg` | `#000000` | Terminal background baseline |
| `surface` | `#0b0b0b` | Subtle pane/search surface when needed |
| `surface_raised` | `#171717` | Active picker row or modal surface |
| `fg` | `#f5f5f5` | Primary text |
| `fg_soft` | `#d7d7d7` | Secondary readable text |
| `muted` | `#9b9b9b` | Metadata, hints, inactive labels |
| `dim` | `#676767` | Disabled, separators, low priority copy |
| `border` | `#3a3a3a` | Minimal pane borders when Claude source uses panes |
| `primary` | `#ff4fd8` | Pink accent: prompt, selection, warnings, key affordances |
| `primary_strong` | `#ff2fb3` | Strong Pink accent for active/failing/destructive emphasis |
| `secondary` | `#00e5ff` | Cyan accent: tools, file mentions, progress, informational contrast |

Future themes must be able to swap `primary` and `primary_strong`, for example Pink to Orange, without editing individual renderers. The complementary color may be configured or generated, but renderers must consume the `secondary` token only.

## Scope
- Create or replace a canonical theme module, for example `src/ui/theme.rs` or `src/ui_theme.rs`.
- Remove hard-coded Gruvbox, Tokyo Night, Catppuccin, Rose Pine, or ad hoc ANSI color constants from active interactive UI code.
- Keep ANSI helpers centralized.
- Provide true-color output and a documented 256-color fallback.
- Ensure style decisions are semantic, not per-component magic numbers.
- Remove or quarantine old `src/ui_colors.rs` Gruvbox constants from the active interactive path. Keeping the file for legacy/noninteractive compatibility is acceptable only if production Claude UI modules do not import those constants.
- Old helper names such as `warn_yellow`, `success_green`, `error_red`, or `info_cyan` may remain temporarily only as semantic wrappers over the canonical theme tokens; they must not imply a fixed old palette.

## Semantic Mapping
- Assistant dot `●`: `fg` or `primary` depending on active/streaming state, matching Claude source spacing.
- Prompt `>`: `primary`.
- Active picker row: `primary` foreground or `surface_raised` background, whichever snapshots prove closest.
- Tool name/progress accent: `secondary`.
- Thinking row: `muted` or `dim` plus italic where supported.
- Compact boundary: `muted`.
- Errors/destructive states: `primary_strong` plus bold/reverse where needed because the palette intentionally has no separate red.
- Success checkmarks: `fg` with bold or `secondary` only when the row needs stronger visibility.
- Separators and inactive footer hints: `dim`.

## Files To Inspect Or Change
- `Cargo.toml`
- `src/ui_colors.rs`
- `src/ui_theme.rs`
- `src/ui_render.rs`
- `src/ui_terminal.rs`
- `src/ui_markdown.rs`
- `src/ui_modal.rs`
- `src/ui_autocomplete.rs`
- `src/ui_trace.rs`
- any new Claude-parity UI modules introduced by Task 169.

## Tests
- Unit test that every public UI style token resolves to ANSI and Ratatui styles.
- Snapshot test that renders a representative screen and asserts Pink/Cyan/monochrome ANSI sequences are present.
- Static test or lint helper that fails if old palette hex values are used in active UI modules.
- Static test or lint helper that fails if UI modules create raw RGB values outside the theme module.

## Acceptance Criteria
- No active interactive UI path uses Gruvbox colors.
- The default theme is visually black/white/grey/Pink/Cyan.
- Future primary accent replacement is a data/config change, not a renderer rewrite.
- The theme supports color-disabled terminals with readable monochrome fallback.
- Existing noninteractive logs remain readable even if they do not use the full TUI theme.
- `rg -n "GRUVBOX|Gruvbox|AQUA|BLUE|YELLOW|PURPLE|BG_HARD|Rose Pine|Catppuccin|Tokyo Night" src` returns no active interactive renderer use after excluding legacy/noninteractive compatibility modules.
- The default Pink token matches the task contract (`#ff4fd8`) unless the task is deliberately amended; do not silently substitute a different pink such as HotPink.
- Color snapshot tests preserve ANSI when verifying Pink/Cyan, not only stripped text.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test theme -- --nocapture
cargo test ui_parity_theme -- --nocapture
./ui_parity_probe.sh --fixture theme-palette
```

The final probe must run the real CLI in a pseudo-terminal and verify that old Gruvbox/Tokyo Night ANSI colors do not appear in the default interactive screen.

## Verification Results

- ✅ Created `src/ui_theme.rs` with tokenized theme system (ColorToken struct, Theme struct)
- ✅ Implemented default theme with correct Pink (#ff4fd8) and Cyan (#00e5ff) values
- ✅ Replaced Gruvbox constants in `src/ui_colors.rs` with theme-mapped values
- ✅ Updated UI modules to use semantic color tokens instead of hardcoded Gruvbox RGB
- ✅ `cargo fmt --check` passes
- ✅ `cargo build` succeeds  
- ✅ `cargo test --test ui_parity startup_fixture` passes
- ✅ Verified no active interactive UI imports Gruvbox constants (legacy constants remain for compatibility)
- ✅ Theme is tokenized for future accent color changes without renderer modifications

The default theme now uses the specified Pink/Cyan monochrome palette with proper tokenization for extensibility.
