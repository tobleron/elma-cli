# Task 086: Stress Test Sloppy Human Prompts And Intent Helper A/B

## Priority
**P3 - LOW PRIORITY / LATE-STAGE VALIDATION**
**Created:** 2026-04-03

## Objective
Add a dedicated late-stage stress-testing layer that evaluates how well Elma handles sloppy, ambiguous, typo-heavy, and casually written human prompts, with explicit attention to whether `intent_helper` materially improves downstream behavior for small local models.

## Why This Matters
Elma is designed for small local LLMs. In real use, users will not always write clean prompts. We need to know:
- whether `intent_helper` successfully clarifies messy user intent
- whether Elma’s behavior improves with `intent_helper` enabled
- how much degradation remains as prompt quality gets worse
- whether routing, planning, and presentation stay grounded under sloppy phrasing

This should be treated as a **late validation layer**, not a current blocker, because baseline reliability on clean prompts still has higher priority.

## Scope
- Add separate sloppy-human variants for existing `_stress_testing/` prompts.
- Keep them sandboxed to `_stress_testing/` targets only.
- Increase sloppiness incrementally rather than jumping straight to extreme noise.
- Compare Elma behavior with `intent_helper` enabled vs disabled or bypassed in a controlled evaluation path.

## Test Design

### Prompt Families
For selected baseline prompts such as `S000B` through `S000I`, create sloppy variants in tiers:

1. **Tier 1 - Light Informality**
- casual phrasing
- minor punctuation loss
- shortened wording

2. **Tier 2 - Medium Sloppiness**
- typos
- inconsistent casing
- weak grammar
- omitted connective words

3. **Tier 3 - Heavy Sloppiness**
- fragmented phrasing
- shorthand
- imprecise wording
- redundant filler

4. **Tier 4 - Realistic Messy Human Input**
- multiple mistakes at once
- partial intent buried in casual text
- still recoverable by a strong intent clarifier

### Comparison Mode
For each sloppy prompt, evaluate:
- `intent_helper` enabled
- `intent_helper` bypassed or disabled in a controlled test path

Record:
- route
- ladder level
- selected formula
- whether execution remained grounded
- final answer quality
- whether task objective was satisfied

## Acceptance Criteria
- Sloppy prompts remain confined to `_stress_testing/` sandboxes.
- Tests are organized separately from the main clean-prompt ladder.
- Sloppiness increases incrementally and is labeled clearly.
- A/B results show whether `intent_helper` improves:
  - route correctness
  - task completion
  - groundedness
  - answer usefulness
- Failures produce actionable diagnostics rather than only pass/fail labels.

## Non-Goals
- Do not prioritize this ahead of current clean-prompt reliability work.
- Do not turn sloppy-prompt support into word-based heuristics.
- Do not bake typo examples directly into routing prompts as deterministic rules.

## Suggested Implementation Order
1. Add a small number of sloppy variants for one or two primitive stress prompts.
2. Add controlled A/B execution mode for `intent_helper`.
3. Produce a comparison report format.
4. Expand to broader primitive coverage only after the first slice is stable.

## Dependencies
- Current CLI reliability work should be substantially stable first.
- Stress ladder on clean prompts should be meaningfully healthier before this begins.
- `intent_helper` runtime path must remain preserved and testable.

## Exit Criteria
- Elma has a dedicated sloppy-human stress suite.
- Intent-helper A/B comparisons are measurable and reproducible.
- The results clearly show where `intent_helper` helps, where it does not, and what follow-up work is justified.
