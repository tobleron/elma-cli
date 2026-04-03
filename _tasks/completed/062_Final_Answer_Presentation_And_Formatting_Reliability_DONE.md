# Task 062: Final Answer Presentation And Formatting Reliability

## Priority
**P0 - USER-FACING RELIABILITY**

## Objective
Make Elma’s last-mile answer generation consistently plain, grounded, concise, and terminal-appropriate, without allowing formatter or presentation stages to mutate correct evidence-grounded answers into decorative or irrelevant output.

## Why This Exists
Recent real CLI debugging showed that:
- evidence gathering and decision making can be correct
- presenter output can be correct
- a later formatter pass can still corrupt the final answer

This is especially dangerous on small local models because “formatting” prompts can over-generalize and turn a good answer into a presentation, tutorial, or boilerplate artifact.

## Problems To Solve
- Final-answer formatting is still vulnerable to over-transformation.
- The boundary between `expert_responder`, `result_presenter`, `formatter`, and chat finalization is not yet fully hardened.
- Some answers become too ceremonial or too broad compared with the actual user request.
- Terminal plain-text expectations are not enforced strongly enough for non-Markdown requests.

## Scope
- Audit the full final-answer path:
  - evidence mode
  - expert responder
  - result presenter
  - claim checker
  - formatter
- Define a strict contract for each role.
- Make “formatting” deterministic unless the user explicitly asks for richer formatting.
- Prevent expansion into:
  - slide decks
  - tutorials
  - marketing prose
  - generalized explanations not asked for
- Add regression tests for known answer-style failures.

## Deliverables
- Clean role boundaries for final-answer generation.
- Deterministic terminal-safe formatting behavior by default.
- New regression tests covering over-formatting and over-expansion failures.
- Updated profile docs/prompts if role boundaries need clarification.

## Acceptance Criteria
- Grounded answers stay grounded through the final output phase.
- Non-Markdown requests produce plain terminal text.
- Formatter cannot broaden the task into a new artifact type on its own.
- `cargo build`, `cargo test`, and real CLI probes for presentation-sensitive cases pass.
