# Task Priority List - Reorganized

**Last Updated:** 2026-04-01
**Status:** Reorganized by impact/effort ratio

---

## Summary

**Total Pending Tasks:** 19
**Total Postponed Tasks:** 2
**Completed Tasks:** 31 (moved to `_tasks/completed/`)

**Recent Completions:**
- ✅ Task 001 - Enable Reflection For All Tasks (Level-Aware)
- ✅ Task 002 - Fix Speech Act Classification (Superseded by Task 007)
- ✅ Task 003 - Complete JSON Fallback Integration
- ✅ Task 004 - Revise Core Formulas (Reply Family) - Superseded by Task 044
- ✅ Task 005 - Revise Core Formulas (Plan Family) - Superseded by Task 044
- ✅ Task 007 - Decouple Classification From Execution
- ✅ Task 008 - Harden OODA Loop And Critic JSON
- ✅ Task 009 - Align Tuning With Current Runtime Architecture
- ✅ Task 010 - Multi-Strategy Planning With Fallback Chains
- ✅ Task 011 - State-Aware Guardrails
- ✅ Task 012 - Review Intel Unit Atomicity
- ✅ Task 013 - Verify JSON Pipeline For Small Models
- ✅ Task 014 - Confidence-Based Routing with Pattern Fallbacks
- ✅ Task 015 - Entropy Based Flexibility (Superseded - entropy already implemented)
- ✅ Task 019 - Improve JSON Repair (Superseded by Task 003/009)
- ✅ Task 024 - Revise And Perfect Existing Formulas (Superseded by Task 044)
- ✅ Task 034 - Formalize Intel Unit Interfaces
- ✅ Task 044 - Integrate Execution Ladder
- ✅ Task 045 - Migrate Remaining Intel Units
- ✅ Task 052 - Complete Intel Migration (Superseded by Task 045)

---

## P0 - CRITICAL (Do First - This Week)

**✅ ALL P0 TASKS COMPLETE!**

| # | Task | Impact | Effort | Status |
|---|------|--------|--------|--------|
| **001** | Enable Reflection For All Tasks | +30% accuracy | 2-3h | ✅ DONE |
| **002** | Fix Speech Act Classification | +25% routing | 1h | ✅ POSTPONED (Already Done) |
| **003** | Complete JSON Fallback Integration | Zero crashes | 4-6h | ✅ DONE |

---

## P1 - HIGH (Do Second - Next Week)

**High impact, moderate effort. Core capability improvements.**

| # | Task | Impact | Effort | Status |
|---|------|--------|--------|--------|
| **006** | Add FETCH Step Type (DISABLED) | Future capability | 2-3h | ⏸️ POSTPONED (internet-based) |
| **007** | Decouple Classification From Execution | Better modularity | 6-8h | ✅ DONE |
| **008** | Harden OODA Loop And Critic JSON | -50% critic parse errors | 4-6h | ✅ DONE |
| **009** | Align Tuning With Architecture | Better tuning efficiency | 4-6h | ✅ DONE |
| **010** | Multi-Strategy Planning With Fallback Chains | Better reliability | 8-10h | ✅ DONE |
| **011** | State-Aware Guardrails | Better safety | 6-8h | ✅ DONE |
| **012** | Review Intel Unit Atomicity | 3B model compatibility | 8-12h | ✅ DONE |
| **013** | Verify JSON Pipeline For Small Models | 3B model reliability | 6-10h | ✅ DONE |

**All P1 tasks complete!** 🎉

**Next priority:** See P2 - MEDIUM tasks below.

---

## P2 - MEDIUM (Do Third - This Month)

**Moderate impact, variable effort. Quality improvements.**

| # | Task | Impact | Effort |
|---|------|--------|--------|
| **012** | Entropy Based Flexibility | Better uncertainty handling | 4-6h |
| **013** | Iterative Program Refinement | Better program quality | 6-8h |
| **014** | Multi-Turn Goal Persistence | Better long conversations | 6-8h |
| **015** | Limit Summarize Step Output | Prevent massive output | 2-3h |
| **016** | Improve Crash Reporting | Better error handling | 2-3h |
| **017** | Drafting Mode Edits | Safer editing workflow | 4-6h |
| **018** | Specialized FS Intel | Faster config parsing | 4-6h |
| **019** | Hierarchical Evidence Compaction | Handle large outputs | 4-6h |
| **020** | Rolling Conversation Summary | Better long context | 6-8h |
| **021** | Platform Capability Detection | Better platform awareness | 4-6h |

---

## P3 - LOW (Do Fourth - Future)

**Nice-to-have or future capabilities. Defer until P0-P2 complete.**

| # | Task | Impact | Effort | Notes |
|---|------|--------|--------|-------|
| **022** | Angel Helper Transient Context | Better context management | 8-10h | |
| **023** | Revise And Perfect Existing Formulas | Covered by 004/005 | - | May be duplicate |
| **024** | Cross Scenario Correlation | Better learning | 10-12h | |
| **025** | Long Term Tactical Memory | Better learning | 10-12h | |
| **026** | Autonomous Prompt Evolution | Self-improvement | 12-15h | Advanced |
| **027** | Entropy Based Flexibility DUPLICATE | - | - | **DELETE** (duplicate of 012) |
| **028** | Predictive Failure Detection | Better reliability | 8-10h | |
| **029** | Analogy Based Reasoning Engine | Better problem solving | 10-12h | |
| **030** | Constraint Relaxation And Creative Problem Solving | Better flexibility | 10-12h | |
| **031** | Pre-Execution Reflection DEPRECATED | - | - | **DELETE** (superseded by 001) |

---

## Task Dependencies

### Dependency Graph

```
Task 001 (Reflection) ──────────────┐
                                     │
Task 002 (Speech Act) ───────────────┼──→ Task 004/005 (Formulas)
                                     │
Task 003 (JSON Fallback) ────────────┘

Task 007 (Decouple Classification) ──→ Task 010 (Multi-Strategy)

Task 012 (Entropy) ──────────────────→ Task 010 (Multi-Strategy)
```

### Recommended Order

**Week 1:**
1. Task 001 (Enable Reflection)
2. Task 002 (Fix Speech Act)
3. Task 003 (Complete JSON Fallback)

**Week 2:**
4. Task 004 (Revise Reply Formulas)
5. Task 005 (Revise Plan Formulas)
6. Task 006 (Add FETCH - disabled)

**Week 3:**
7. Task 007 (Decouple Classification)
8. Task 008 (Harden OODA Loop)
9. Task 009 (Align Tuning)

**Week 4:**
10. Task 010 (Multi-Strategy Planning)
11. Task 011 (State-Aware Guardrails)

---

## Deleted/Deprecated Tasks

**Remove these files:**

| File | Reason |
|------|--------|
| `027_Entropy_Based_Flexibility_DUPLICATE.md` | Duplicate of Task 012 |
| `031_Pre_Execution_Reflection_DEPRECATED.md` | Superseded by Task 001 |

---

## Metrics Target

After completing P0 tasks (001-003):
- **+30% accuracy** (reflection on all tasks)
- **+25% routing** (correct speech act classification)
- **Zero crashes** (JSON fallback everywhere)

After completing P1 tasks (004-011):
- **+20% formula accuracy** (reply family revised)
- **+30% plan quality** (plan family revised)
- **Better modularity** (classification decoupled)

---

## Notes

- Tasks renumbered sequentially by priority on 2026-04-01
- Old numbers preserved in task file headers for reference
- Completed tasks moved to `_tasks/completed/` with `_DONE` suffix
- Duplicates/deprecated marked for deletion
