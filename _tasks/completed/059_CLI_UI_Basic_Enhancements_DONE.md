# Task 059: CLI UI Basic Enhancements

## Priority
**P2 - UX POLISH**

## Objective
Refine Elma's CLI interface to feel cleaner, more coherent, and easier to read during development, without reducing the current level of runtime visibility.

This task is intentionally basic and should avoid advanced UI redesign, animation, terminal framework migration, or large-scale refactors. The goal is a neater presentation layer on top of the existing verbose development output.

## Requested Changes

### 1. Lighten Elma's Main Output Color
- Adjust Elma's final-answer color away from the current stronger orange tone toward a lighter pink.
- Preserve readability on common dark terminal backgrounds.
- Keep the change localized to Elma's primary response rendering rather than broad recoloring of the entire interface.

### 2. Improve Visibility of Important Runtime Steps
- Make classification output slightly brighter and easier to notice during development.
- Review other high-signal runtime categories and improve visibility where helpful.
- Favor a small, consistent visual hierarchy instead of many unrelated highlight colors.
- Likely candidates include:
  - classification
  - planning
  - reflection
  - retry outcomes
  - shell execution / command lines

### 3. Remove Emoticons And Refine Interface Coherence
- Remove decorative emoticons/symbols that make the CLI feel noisy or inconsistent.
- Replace them with neater text-first or minimal-symbol alternatives.
- Keep the interface verbose and development-friendly, but make it feel more intentional and coherent.
- Maintain scanability of:
  - user prompt line
  - process trace lines
  - final Elma output
  - retry / status summaries

## Constraints
- Do not reduce the amount of useful development-time visibility.
- Do not introduce heavy theming complexity or excessive configurability in this task.
- Prefer surgical updates in existing UI rendering helpers such as:
  - `src/ui_trace.rs`
  - `src/ui_colors.rs`
  - nearby CLI output helpers if needed
- Keep the overall terminal UX plain, readable, and stable.

## Suggested Acceptance Criteria
- Elma's final answer appears in a lighter pink tone.
- Classification and other key process stages are easier to notice at a glance.
- Decorative emoticons are removed from the CLI interface.
- The interface still feels verbose, but visually cleaner and more coherent.
- `cargo build` and `cargo test` pass after the UI adjustments.

## Notes
- This is a presentation-layer polish task, not a workflow or orchestration task.
- Changes should be validated in real CLI usage, not only by static inspection.
