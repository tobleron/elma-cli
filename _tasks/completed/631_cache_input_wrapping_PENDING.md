# 631 Cache Input Wrapping

## Status
**PARTIALLY COMPLETE**

## What's Done
- Cache fields exist: `cached_input_key: (String, usize)` and `cached_wrapped_input: Vec<String>` (claude_render.rs:105-106)
- Cache is populated when input content or terminal width changes (claude_render.rs:1119-1124)
- Cache is used for input height calculation (claude_render.rs:1125)

## What's Missing
The actual display path at line 1310 still calls `wrap_input_lines()` fresh every frame instead of using the cached value:

```rust
// Line 1119-1124: cache is populated
let input_key = (self.input_lines.join("\n"), input_display_width);
if input_key.0 != self.cached_input_key.0 || input_key.1 != self.cached_input_key.1 {
    self.cached_wrapped_input = wrap_input_lines(&self.input_lines, input_display_width);
    self.cached_input_key = input_key;
}

// Line 1310: display path bypasses the cache
let display_wrapped = wrap_input_lines(&self.input_lines, text_width.max(10));
```

The cache uses `input_display_width` (main_area.width - 6) but the display call uses `text_width` (input_area.width - 2). These may differ, so the fix needs to verify the widths match or unify the width calculation.

## Fix
At line 1310, replace the fresh `wrap_input_lines` call with `&self.cached_wrapped_input` (after verifying width equivalence).
