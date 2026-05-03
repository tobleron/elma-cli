# Task 291: Core Tools Always Available — Remove Deferred Discovery for Shell/Read/Search

## Status: ACTIVE

## Problem

Observed in real Elma session (2026-04-27):

1. User asked `list files in docs` → Elma called `tool_search` with various queries, got no
   matches, and failed to list the directory. It has no shell/read access at that point.
2. User asked `what time is it now?` → Elma **hallucinated "12:34 PM"** without calling any tool.
   Only after being confronted did it eventually call `shell` with `date`.

Root cause: `shell`, `read`, and `search` are registered as **deferred** in `tool_registry.rs`,
meaning they are only added to the tool definitions after the model successfully calls `tool_search`
with a matching query. A small 3B–4B model frequently fails to match its casual intent to the
registered hint phrases, so it stalls or hallucinates instead of gathering evidence.

## Principle Violated

> "If evidence is missing: gather it — or say clearly that evidence is insufficient."
> — AGENTS.md, Grounded Answers Only

The deferred-discovery pattern is structurally incompatible with small-model reliability for
the 5 core tools Elma has. Deferred discovery only adds value for large tool catalogs (50+ tools)
where token budget matters. For 5 tools the benefit is zero and the risk is real.

## Fix

### 1. Mark core tools as not-deferred in `tool_registry.rs`

`shell`, `read`, `search`, and `update_todo_list` should all be `.not_deferred()` so they appear
in every request from the first turn. `tool_search` remains available for future extensibility
(e.g., if specialty tools are added), but its use is no longer a prerequisite for core evidence
gathering.

### 2. Keep `tool_search` but reposition it

`tool_search` is not removed — it stays available for discovering specialty/extension tools added
later. Its description should be updated to reflect that it is for *additional* tools, not the
core set.

### 3. No routing changes

Do NOT add word-based routing to compensate. The model must be free to choose tools based on its
own reasoning. Making core tools always-available lets it do exactly that.

## Files Changed

- `src/tool_registry.rs` — remove `.not_deferred()` calls where missing; add to shell/read/search

## Verification

1. `cargo build`
2. `cargo test`
3. Real CLI: `list files in docs` → model should immediately call `shell` with `ls docs/` or
   `read` on a known path without needing a `tool_search` first.
4. Real CLI: `what time is it?` → model must call `shell date` and return grounded answer.

## Anti-patterns Avoided

- No word matching on user input
- No hardcoded routes for "time" or "list" prompts
- No injected fake tool results
- Model decides which tool to call based purely on tool schema + self-reasoning
