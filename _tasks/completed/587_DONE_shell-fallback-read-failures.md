# Task 587: Shell Fallback When `read` Fails N Consecutive Times

## Session Evidence
Session `s_1777822834_658323000`: After the first `read` failure, the model tried `read` 7 more times in the same cycle (8 total), all with the same `filePath: missing` error. The duplicate gate caught each attempt (`"duplicate detected (previous failure) signal=read:"`) but stagnation accumulated. The model never switched to using `shell cat docs/ARCHITECTURE.md` as a fallback strategy.

## Problem
When a tool fails repeatedly, the model enters a stagnation loop: it retries the exact same broken call → gets the same error → stagnation counter increases → eventually forced to finalize with zero useful evidence. The model lacks the strategic flexibility to switch to an alternative approach (e.g., shell with `cat`/`head` for file reading).

## Solution
After N consecutive `read` failures (suggested: 3), inject a system message that explicitly proposes a fallback strategy:

```
System message: "The 'read' tool has failed 3+ times. Use 'shell cat docs/FILENAME.md' as a fallback to read files."
```

Additionally, when inject this, include a concrete example:
```
System message (following the above): "For example: shell command='cat docs/ARCHITECTURE.md'"
```

Implementation location: `src/tool_loop.rs`, in the tool failure processing section (around line 1540-1560), after detecting consecutive failures for the same tool.
