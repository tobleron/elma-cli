# Task 794: Polishing Slash Command Modal Dialogs

## Status
- **Priority:** High
- **Assignee:** Unassigned
- **Status:** Pending

## Objective
Convert flat/text-only slash command outputs into interactive modal dialogs with selectable options to improve UX and prevent accidental command triggers.

## Requirements
- Replace `println!` or flat messages with `tui.set_modal(...)` or interactive lists where applicable.
- Ensure 100% functional parity with current behavior but in a more "alive" UI.
- Use the existing modal infrastructure from `src/ui/ui_state.rs` and `src/ui/ui_modal.rs`.

## Commands to Polish (Set A: Configuration)
- `/models`: Switch between available models/providers (needs a list picker).
- `/approve`: Select approval policy (Off/Ask/On) via a choice list instead of blind cycling.
- `/tune`: Select model performance/cost profiles.
- `/provider`: Interactive endpoint configuration (IP/Port/Base URL).

## Commands to Polish (Set B: Session & Tools)
- `/sessions`: Improved session picker (already exists but may need styling parity).
- `/tools`: List discovered tools with detailed descriptions in a modal.
- `/goals`: Display active goals and milestones in a structured list.
- `/usage`: Visual breakdown of token costs and limits.

## Commands to Polish (Set C: Reset & Safety)
- `/reset`: Add a "Confirm Reset" dialog box to prevent accidental history loss.
- `/clear`: Confirm before clearing the transcript.
- `/snapshot`: List and manage manual snapshots.
