# Task 451: Recipe And Subrecipe Workflow System

**Status:** pending
**Source patterns:** Goose recipes/subrecipes, LocalAGI scheduler tasks, OpenHands microagents
**Depends on:** Task 380 (semantic continuity tracking), completed Task 338 (event log)

## Summary

Introduce external, versioned workflow recipes for repeatable task patterns, with bounded subrecipes for focused investigation, implementation, verification, and finalization.

## Why

Elma has built-in formulas and skills, but extending them requires Rust changes. Reference agents support external recipes or microagents that package reusable workflows without bloating the core prompt. This fits Elma's decomposition philosophy if recipes stay principle-first and small-model-friendly.

## Implementation Plan

1. Define a TOML or YAML recipe schema with objective, preconditions, stages, tools, outputs, and verification.
2. Add a loader with schema validation and versioning.
3. Let formulas call recipes without changing `TOOL_CALLING_SYSTEM_PROMPT`.
4. Emit recipe selection and stage transitions as visible events.
5. Add a small built-in recipe only after tests prove the mechanism.

## Success Criteria

- [ ] A recipe can define a multi-stage workflow without Rust code changes.
- [ ] Recipe execution preserves semantic continuity from user request to final answer.
- [ ] Invalid recipes fail with path-aware config errors.
- [ ] Recipe events are visible in the transcript.
- [ ] Tests cover loading, execution, failure, and resume.

## Anti-Patterns To Avoid

- Do not turn recipes into long example prompts.
- Do not create keyword-triggered recipe routing.
- Do not let recipes bypass permission or evidence requirements.
