# Task 205: Transcript-Native Runtime Telemetry And Final-Answer Presentation

## Priority
P1

## Objective
Move operational visibility out of the footer and into the transcript, while improving final-answer presentation so Elma’s results are easier to read and audit in the terminal.

## Why This Exists
The latest verified session showed two UX problems:
- operational notices such as queued-input and mode labels are currently routed through footer/status surfaces, which makes them easy to miss and pollutes the bottom bar;
- long final answers can be technically correct but visually weak, because they do not yet follow a strong presentation contract for terminal markdown.

The user also wants far more visibility into what Elma is doing internally, including budgeting, hidden processes, compaction, and stop behavior.

## Required Behavior
- The footer/status bar must remain limited to core runtime metrics only.
- Execution mode, queue notices, budgeting updates, compaction events, stop reasons, and hidden-process summaries must appear in transcript-native rows.
- These transcript rows may auto-collapse after a short timeout, but they must first be visible in the history.
- Final answers must render with a stronger terminal-friendly markdown structure.

## Footer Rule
Allowed in footer/status bar:
- model
- context usage
- token counts
- timing/effort

Not allowed in footer/status bar:
- selected mode/formula
- queued message notices
- budget warnings
- compaction notices
- stop-reason notices
- other operational notifications

## Transcript Telemetry Requirements
Add transcript-native rows for at least:
- selected execution plan
- main-task creation
- queueing of a follow-up prompt
- budget state changes and budget warnings
- compaction start/finish
- stop-policy outcomes
- document work-plan decisions for document skills

These rows should be:
- concise by default
- collapsible after a short timeout or when the turn completes
- still recoverable in verbose/transcript views

## Final-Answer Presentation Contract
Prefer terminal-friendly markdown with:
- a short top-line summary when the answer is non-trivial
- flat bullets for key findings
- explicit evidence or inspected-source section when relevant
- short next-step section only when useful

Avoid:
- unbroken walls of prose
- hidden operational caveats that never appear in the answer or transcript
- overusing headings for very short answers

## Integration Points
- `app_chat_loop.rs`
- `ui_terminal.rs`
- transcript/message model in `ui_state.rs` and Claude renderer surfaces
- final answer rendering path

## Acceptance Criteria
- Queue and mode notices no longer appear only in the footer.
- Footer remains stable and limited to runtime metrics.
- Users can see budgeting, compaction, and stop information in transcript-native form.
- Long final answers are more readable in terminal markdown than before.

## Required Tests
- PTY snapshot showing queued-input notice in transcript rather than footer
- PTY snapshot showing mode/formula disclosure in transcript or header, not footer
- PTY snapshot showing collapsible process/budget row behavior
- rendering test for a long final answer with structured markdown sections
