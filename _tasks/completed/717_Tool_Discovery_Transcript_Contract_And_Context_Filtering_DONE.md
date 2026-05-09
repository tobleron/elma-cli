# Task 717: Tool Discovery Transcript Contract And Context Filtering

## Type

Tool Visibility / Transcript-Native Operations

## Severity

Medium

## Evidence

Round 6 transcripts repeatedly begin with:

```text
system: [TOOLS] discovered 2 tool(s)
```

Yet the same sessions use many more tools, including `workspace_info`, `search`, `read`, `glob`, `write`, `backup`, `file_size`, `shell`, and `ls`.

Example:

- `project_tmp/round6_sessions/prompt_08_s_1778140807_118803000/session.md`

## Problem

The transcript-visible tool discovery message is misleading. It appears to report only a subset or search-discovered tools while runtime execution exposes a larger tool set. This weakens trace readability and makes debugging tool availability harder.

## Requirements

- Define a clear transcript contract for tool availability:
  - active tool set
  - context-filtered hidden tools
  - tools discovered through `tool_search`
  - tools used in the turn
- Rename or split the current `[TOOLS] discovered N tool(s)` row so it does not imply the full active tool set when it is not.
- Surface context-filtering decisions as collapsible transcript rows.
- Ensure the trace and transcript agree on active tool names for each turn.
- Keep the bottom status bar unchanged: model name, token count, elapsed time only.

## Acceptance Criteria

- [ ] A prompt 08-style session accurately reports the active tools and the discovered/search-added tools separately.
- [ ] The transcript makes it obvious why `backup` was available.
- [ ] Tests cover transcript wording for full tool set, filtered tool set, and tool-search results.
- [ ] No footer/status bar fields are added.

