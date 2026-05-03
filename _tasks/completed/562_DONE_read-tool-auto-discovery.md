# Task 562: Read Tool Parameter Auto-Discovery

## Session Evidence
In session `s_1777820401_246730000`, the model failed to call `read` correctly because it couldn't determine the filePath parameter. Meanwhile, there was evidence on disk:
- e_001_raw.txt: full workspace directory listing with all file paths
- e_002_raw.txt: docs/ directory listing with 53 specific filenames

The model needed to read "all docs" but couldn't bridge the gap between "I know the docs exist" and "I can call read with the correct filePath".

## Problem
The model knows documents exist (from ls output) but fails to construct valid `read` tool calls with the discovered paths. The system collects evidence but doesn't feed it into the tool-calling context in a structured way that the model can use.

## Solution
1. After successful directory listing, auto-generate a "discovered files" context block with explicit `read` tool call examples
2. Format: `You can now call: read filePath="docs/ARCHITECTURE.md", read filePath="docs/CONFIGURATION.md", ...` (show first 5-10 files)
3. Implement a `suggest_next_calls()` function that takes tool output and generates concrete next-step tool call templates
4. For the `read` tool specifically, if the model has just listed a directory, prepend the context with a list of "ready to read" file paths
5. Consider a `batch_read` or `read_many` tool that takes a list of file paths and reads them all — this reduces the iteration count for bulk-reading operations
