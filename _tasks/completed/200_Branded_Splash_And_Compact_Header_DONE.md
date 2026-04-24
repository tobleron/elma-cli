# Task 200: Branded Splash And Compact Header

## Priority
P1

## Objective
Add a branded Elma startup splash and a compact persistent header that fit the tokenized Pink/Black/White theme.

## Why This Exists
Elma is intentionally moving away from pure Claude parity at the product identity layer. The startup and compact header should communicate a distinct Elma brand without sacrificing terminal safety.

## Required Behavior
- Show a startup splash derived from `logo/Elma_CLI_Logo.png`.
- Keep the splash on screen for about 3 seconds during startup.
- After startup, show a compact header with:
  - minimal logo treatment
  - execution mode / formula label
  - workspace/session/model essentials
- Preserve alternate-screen and raw-mode safety on normal exit, Ctrl-C, Ctrl-D, and panic.

## Rendering Requirements
- Convert the source logo into terminal-friendly ANSI/ASCII art.
- Default palette must use tokenized black/white/grey plus pink accent and optional cyan complement.
- The compact header must remain visually light; do not reintroduce old boxed chrome or heavy rails.

## Technical Requirements
- splash rendering and header rendering must use the canonical theme tokens
- no hardcoded one-off color escapes outside the theme surface
- splash timing must not block input/event cleanup logic
- if startup finishes early, the splash must still exit cleanly without tearing the terminal state

## Acceptance Criteria
- Splash appears reliably without breaking terminal restore.
- Compact header is visibly branded and does not reintroduce old bulky UI chrome.
- Header continues to work with existing message-first layout and task/status surfaces.

## Required Tests
- PTY snapshot of splash and compact header states
- terminal cleanup test on interrupted splash
- theme token audit ensures no rogue hardcoded colors were added for this feature


## Completion Note
Completed during Task 204 verification on 2026-04-23.
Verified with the relevant automated checks available in this repo, including `cargo build`, targeted tests, and UI parity or startup checks where applicable.
