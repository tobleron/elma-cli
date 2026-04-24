# 211: Confirmations and Selections via `dialoguer`

## Status
`pending`

## Crate
`dialoguer` — Prompts, confirmations, selections, and password input.

## Rationale
Elma uses `inquire` for rich interactive prompts, but `dialoguer` fills a gap: simple yes/no confirmations, multi-select lists, and password inputs — all with a cleaner, more compact API for common cases that `inquire` handles verbosely. They are complementary, not mutually exclusive. `dialoguer` also has better Ratatui integration hooks.

## Implementation Boundary
- Add `dialoguer = "0.11"` to `Cargo.toml`.
- Audit existing `inquire::Confirm`, `inquire::Select`, or `inquire::Password` usage.
- Replace low-complexity confirmations with `dialoguer::Confirm`:

  ```rust
  use dialoguer::Confirm;

  if Confirm::new()
      .with_prompt("Delete session data?")
      .default(false)
      .interact_opt()?
  ```

- Replace simple single-select prompts with `dialoguer::Select` (when `inquire` is too heavy).
- Keep `inquire` for complex multi-step prompts, autocomplete inputs, or editor-based prompts.
- Do NOT replace all `inquire` usage — only where `dialoguer` is strictly better (confirmations, simple selections).
- Do NOT change the TTY/integration contract — ensure prompts still work in the Ratatui context.

## Verification
- `cargo build` passes.
- At least one existing confirmation path uses `dialoguer`.
- No regressions in interactive prompt behavior.