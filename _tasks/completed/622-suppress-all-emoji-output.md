# Task 622: Suppress all emoji/emoticon output from Elma CLI

## Type

Refactor

## Severity

Medium

## Scope

System-wide

## Session Evidence

The thought summary on the right panel displayed a `📝` emoji prefix. Emoji rendering on terminals is unreliable:
- Some terminals don't render emoji at all (showing tofu/missing glyph)
- Different terminals render different widths (breaking alignment)
- Emoji steal attention from the actual content
- Terminal multiplexers (tmux/screen) often break emoji rendering

Emojis currently used in the codebase (partial list):
- `📝` — thought summary prefix (`claude_render.rs`)
- `🚫` — error state (`ui_modal.rs`)
- `🦀` — Crab constant (`ui_theme.rs`)
- `⚡` — warning icon (`ui_theme.rs`, `ui_modal.rs`)
- `🔒` — lock icon (`ui_theme.rs`)
- `⚠️` — warning (many files)
- Emoji picker entries (`ui_autocomplete.rs` — `:smile:`, `:tada:`, etc.)

## Problem

Emoji output causes visual inconsistency across terminals and violates the CLI aesthetic. Elma is a terminal agent — output should be plain text, using Unicode box-drawing and punctuation symbols only. Status indicators should use simple glyphs: `✓`, `✗`, `⚠`, `⚡`, `🦀` should become text equivalents or simple Unicode.

## Proposed Solution

1. **Replace all emoji with Unicode symbols or plain text**:
   - `📝` → `--` or `summary`
   - `🚫` → `BLOCKED`
   - `🦀` → remove (unused)
   - `⚡` → `!`
   - `🔒` → remove/unused
   - `⚠️` → `!` or keep as `⚠`

2. **Remove emoji autocomplete entries** (`ui_autocomplete.rs` lines 132-170+)

3. **Audit and replace**: document_adapter.rs emoji, tool_loop.rs `⚠️` → `!`

Files to change:
- `src/claude_ui/claude_render.rs` — `📝` → text prefix
- `src/ui/ui_autocomplete.rs` — remove emoji map entries
- `src/ui/ui_theme.rs` — replace `🦀` with nothing or text
- `src/ui/ui_modal.rs` — `🚫` → text
- `src/document_adapter.rs` — `📄`, `✅`, `❌`, `🔍` → text
- `src/tool_loop.rs` — `⚠️` → `! `
- `src/stop_policy.rs` — `⚠️` → `! `
- `src/hook_system.rs` — `⚠️` → `! `

## Acceptance Criteria

- [ ] No emoji codepoints in any Elma output (session.md, terminal transcript, TUI display)
- [ ] Status indicators use simple Unicode: `✓`, `✗`, `⚠`
- [ ] Autocomplete no longer suggests emoji
- [ ] `grep -rP '[\x{1F300}-\x{1F9FF}\x{2600}-\x{27BF}]' src/` returns no results (except Unicode symbols like `✓` `✗`)

## Verification Plan

- Build and run: verify no tofu/garbled characters in terminal
- Check session.md output: verify no emoji in transcript
- Visual inspection: right panel thought summary shows `summary: text` not `📝 text`

## Dependencies

None.

## Notes

`✓` (U+2713), `✗` (U+2717), `⚠` (U+26A0), `⚡` (U+26A1) are NOT emoji — they're Unicode symbols that render reliably. Keep those.
