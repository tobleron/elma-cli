# Task 481: Benchmark Leaderboard And Eval Dashboard

**Status:** pending
**Source patterns:** Goose benchmark scripts, Qwen-code terminal-bench, Elma tuning history
**Depends on:** Task 473 (provider fault harness), Task 478 (headless event API)

## Summary

Create an offline benchmark runner and report dashboard for comparing models, providers, profiles, and architectural changes on reliability, evidence use, tool efficiency, and final answer quality.

## Why

Elma's philosophy depends on measurable reliability under constrained models. Existing stress and tuning assets are useful, but the project needs a regular aggregate report that shows whether changes improve or degrade behavior.

## Implementation Plan

1. Normalize scenario definitions and expected signals.
2. Run scenarios through the headless event API.
3. Score evidence collection, semantic continuity, tool loops, finalization, and wall-clock/token cost.
4. Generate a local HTML or markdown report with model/profile comparisons.
5. Keep scenario data offline and reproducible.

## Success Criteria

- [ ] Benchmark runner produces a per-model and per-profile summary.
- [ ] Reports include failures with linked event logs/transcripts.
- [ ] Scores include reliability and not only latency.
- [ ] Existing stress scenarios can be migrated incrementally.
- [ ] Tests cover scoring logic.

## Anti-Patterns To Avoid

- Do not optimize only for speed.
- Do not require network providers for the default benchmark.
- Do not hide failed transcripts from the report.
