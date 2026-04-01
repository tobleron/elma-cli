# Elma-cli Task Priority List

**Last Updated:** 2026-03-31
**Based on:** Architectural Audit Report

---

## P0 - CRITICAL (Do First)

These tasks have the highest accuracy/reliability gain with minimal changes.

| # | Task | Expected Impact | Effort |
|---|------|-----------------|--------|
| **001** | Enable Reflection For All Tasks | +30% accuracy, -25% retries | LOW |
| **002** | Fix Speech Act Classification | +25% routing accuracy | LOW (prompt only) |
| **008** | 100% JSON Reliability Between Components | **100% JSON reliability**, -90% hallucination | HIGH (5 phases) |
| **009** | JSON Fallback Strategy | **Zero crashes**, graceful degradation | MEDIUM |

**Note:** Task 009 (Fallback Strategy) must be implemented BEFORE Task 008 Phase 1 - every validation layer needs fallback from day one.

---

## P1 - HIGH (Do Second)

These tasks add critical capabilities or fix major gaps.

| # | Task | Expected Impact | Effort |
|---|------|-----------------|--------|
| **003** | Add READ and SEARCH Step Types | +25% search accuracy, +20% read safety | MEDIUM |
| **004** | Add FETCH Step Type (DISABLED) | Future capability, zero risk | LOW |
| **005** | Revise Core Formulas (Reply Family) | +20% formula accuracy, -30% retries | MEDIUM |
| **006** | Revise Core Formulas (Plan Family) | +30% plan quality, -40% cleanup errors | MEDIUM |
| **007** | Optimize Workspace Context | -30% tokens, better project understanding | MEDIUM |

---

## P2 - MEDIUM (Do Third)

These tasks improve quality but are less critical.

| # | Task | Expected Impact | Effort |
|---|------|-----------------|--------|
| **009** | Harden OODA Loop And Critic JSON | -50% critic parse errors | MEDIUM |
| **013** | Decouple Classification From Execution | Better modularity | MEDIUM |
| **015** | Autonomous Tool Discovery | Better workspace-specific tool use | MEDIUM |
| **017** | Align Tuning With Current Runtime Architecture | Better tuning efficiency | MEDIUM |

---

## P3 - LOW (Do Fourth)

These tasks are nice-to-have or future capabilities.

| # | Task | Expected Impact | Effort |
|---|------|-----------------|--------|
| **010** | Entropy Based Flexibility | Better uncertainty handling | HIGH |
| **011** | Iterative Program Refinement | Better program quality | HIGH |
| **014** | Multi-Turn Goal Persistence | Better long conversations | HIGH |
| **020** | Limit Summarize Step Output | Prevent massive output | LOW |
| **021** | Improve Crash Reporting | Better error handling | LOW |
| **027** | Drafting Mode Edits | Safer editing workflow | MEDIUM |
| **028** | State Aware Guardrails | Better safety | HIGH |
| **029** | Specialized FS Intel | Faster config parsing | MEDIUM |
| **030** | Hierarchical Evidence Compaction | Handle large outputs | MEDIUM |
| **031** | Rolling Conversation Summary | Better long context | HIGH |
| **033** | Revise And Perfect Existing Formulas | Covered by 005/006 | - |
| **034** | Formalize Intel Unit Interfaces | Better modularity | MEDIUM |
| **035** | Cross Scenario Correlation | Better learning | HIGH |
| **036** | Long Term Tactical Memory | Better learning | HIGH |
| **037** | Autonomous Prompt Evolution | Self-improvement | HIGH |
| **038** | Entropy Based Flexibility | Duplicate of 010 | - |
| **039** | Predictive Failure Detection | Better reliability | HIGH |
| **040** | Platform Capability Detection | Better platform awareness | MEDIUM |
| **041** | Analogy Based Reasoning Engine | Better problem solving | HIGH |
| **042** | Multi-Strategy Planning With Fallback Chains | Better reliability | HIGH |
| **043** | Constraint Relaxation And Creative Problem Solving | Better flexibility | HIGH |

---

## DEPRECATED / SUPERSEDED

| # | Task | Superseded By | Reason |
|---|------|---------------|--------|
| **012** | Pre-Execution Reflection | Task 001 | Covered with clearer scope |
| **019** | Improve JSON Repair For Malformed Output | Task 009 | GBNF grammar handles this |
| **024** | Decouple Model Profile Mapping | N/A | Postponed |
| **033** | Revise And Perfect Existing Formulas | Task 005/006 | Split into focused tasks |
| **038** | Entropy Based Flexibility | Task 010 | Duplicate |

---

## Task Creation Guidelines

### For New Tasks
1. **Manageable scope** - Completable in single agent session
2. **Clear acceptance criteria** - Measurable outcomes
3. **Principle-based** - No hardcoded rules in prompts
4. **Articulate terminology** - Precise definitions
5. **Security-first** - FETCH/internet access disabled with warning

### For Updating Existing Tasks
1. **Check alignment** - Does this align with hybrid architecture?
2. **Check priority** - Is this P0/P1/P2/P3?
3. **Check dependencies** - What must be done first?
4. **Check for duplicates** - Merge overlapping tasks
5. **Deprecate if superseded** - Mark old tasks clearly

---

## Architecture Principles

All tasks must align with:

1. **Accuracy and Reliability First** - P0 tasks fix critical accuracy issues
2. **Elma Philosophy** - Principle-based prompts, autonomous reasoning
3. **Articulate Terminology** - Precise terms in code AND prompts
4. **Security-First** - Internet access disabled until audited
5. **Minimal Changes for Maximum Gain** - P0/P1 tasks have best effort/impact ratio
6. **Hybrid Architecture** - Keep existing strengths, add missing operations selectively

---

## Next Actions

1. **Complete Task 001** - Enable reflection for all tasks
2. **Complete Task 002** - Fix speech act classification
3. **Complete Task 003** - Add READ and SEARCH step types
4. **Complete Task 004** - Add FETCH (DISABLED) step type
5. **Complete Task 005** - Revise reply formulas
6. **Complete Task 006** - Revise plan formulas
7. **Complete Task 007** - Optimize workspace context

After P0/P1 tasks complete, re-evaluate P2/P3 priorities based on metrics.
