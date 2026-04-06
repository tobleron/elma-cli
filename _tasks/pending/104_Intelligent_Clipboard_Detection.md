# Task 104: Intelligent Clipboard Detection

## Objective
Enhance user workflow by detecting images in the clipboard and suggesting their inclusion in prompts.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Clipboard Library**:
    - Use the `arboard` crate to access the system clipboard (cross-platform).
2. **Detection Logic**:
    - Implement a `detect_clipboard_image()` function in `src/ui_state.rs`.
    - Check if the clipboard contains image data (e.g., PNG/JPEG).
3. **UI Notification**:
    - Implement a `draw_clipboard_hint()` in `src/ui.rs`.
    - Show a subtle hint (e.g., `[CLIPBOARD: Image detected, press Tab to attach]`) above the input prompt when an image is found.
4. **Integration**:
    - Check for clipboard changes when the terminal window gains focus.
    - Hook into the prompt input to allow the user to "attach" the clipboard image to the next request.
5. **Security/Privacy**:
    - Only read the clipboard when the app is in the foreground.
    - Provide a configuration setting to disable clipboard monitoring.

### Proposed Rust Dependencies
- `arboard = "3.3"`: Robust cross-platform clipboard access.

### Verification Strategy
1. **UX**:
    - Copy an image from a browser and confirm the hint appears in Elma.
    - Confirm the hint disappears after pasting or clearing the clipboard.
2. **Platform Compatibility**:
    - Verify on macOS, Linux, and Windows if possible.
3. **Safety**:
    - Confirm the app doesn't crash if the clipboard is locked or unavailable.
