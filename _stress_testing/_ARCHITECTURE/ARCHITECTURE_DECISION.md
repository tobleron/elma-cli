# Architecture Decision Record: Core Operation Model

## Date
2026-03-31

## Status
**DECIDED** - Hybrid approach (existing + selective enhancements)

## Context
Elma-cli currently has a sophisticated orchestration architecture with:
- Step types: Shell, Select, Plan, MasterPlan, Decide, Summarize, Edit, Reply
- Routing: Speech Act × Workflow × Mode → Route → Formula
- Verification: Sufficiency + Critics (Logical/Efficiency/Risk) + Refinement
- Reflection: Pre-execution self-assessment

The audit proposed a 9 core operation model:
- INFER, OBSERVE, READ, SEARCH, TRANSFORM, WRITE, EXECUTE, FETCH, REPORT

## Comparison Analysis

### Existing Architecture Strengths
1. **Step types are already operation-like** - Shell=EXECUTE, Edit=WRITE, Select=SEARCH, Reply=INFER+REPORT
2. **Formula system already composes operations** - `inspect_summarize_reply` = OBSERVE→TRANSFORM→REPORT
3. **Verification loop is sophisticated** - Critics catch issues post-execution
4. **Reflection exists** - Pre-execution self-assessment (but skipped for DIRECT)
5. **Tool discovery exists** - `tool_discovery.rs` for workspace-specific tools
6. **Clean separation** - Planning vs. Execution vs. Verification

### Existing Architecture Weaknesses
1. **Terminology mismatch** - Step types don't map cleanly to operations
2. **Conflated operations** - Edit = TRANSFORM+WRITE, Summarize = READ+TRANSFORM
3. **Missing operations** - No explicit FETCH (internet), no explicit OBSERVE
4. **Reflection skipped for DIRECT** - Simple tasks don't get self-assessment
5. **Speech act conflates intent with operation** - CHAT vs INFO vs INSTRUCTION

### Proposed Architecture Strengths
1. **Clear operation semantics** - Each core type has single responsibility
2. **Better composability** - Formulas explicitly chain operations
3. **Explicit FETCH** - Internet access as first-class operation
4. **Explicit TRANSFORM** - Compute change without applying
5. **Better verification** - Preconditions/postconditions per operation

### Proposed Architecture Weaknesses
1. **Requires step type changes** - Breaking change for existing sessions
2. **More step types** - Increased complexity for model to select
3. **FETCH security risk** - Internet access needs careful gating
4. **Migration overhead** - Need backward compatibility shims

## Decision

**HYBRID APPROACH: Keep existing architecture, add missing operations selectively**

### What to KEEP (Existing)
| Component | Reason |
|-----------|--------|
| **Step enum structure** | Already clean, well-typed |
| **Formula system** | Already composes operations correctly |
| **Verification loop** | Sophisticated critic system works well |
| **Reflection module** | Already implemented, just needs to run always |
| **Tool discovery** | Already exists, just needs integration |
| **Routing (3-stage)** | Speech×Workflow×Mode works well |

### What to ADD (New)
| Component | Reason | Priority |
|-----------|--------|----------|
| **Step::Read** | Explicit read-only file access | HIGH |
| **Step::Search** | Explicit search with query semantics | HIGH |
| **Step::Observe** | Explicit metadata-only inspection | MEDIUM |
| **Step::Fetch** | Internet access (DISABLED with warning) | LOW |
| **Preconditions/Postconditions** | Better verification | MEDIUM |

### What to DEFER (Not Now)
| Component | Reason |
|-----------|--------|
| **Split Edit→Transform+Write** | High complexity, low immediate benefit |
| **Split Summarize→Read+Transform** | Works well enough as-is |
| **Split Reply→Infer+Report** | Conflation is acceptable for chat |
| **FETCH enabled** | Security risk, defer until gated properly |

### What to FIX (Immediate)
| Issue | Fix | Priority |
|-------|-----|----------|
| **Reflection skipped for DIRECT** | Run reflection for all tasks | CRITICAL |
| **Speech act misclassification** | Fix prompts (principles, not rules) | CRITICAL |
| **Critics hallucinate failures** | Ground in actual output | CRITICAL |
| **No INVESTIGATE triggering** | Fix complexity prompts | HIGH |
| **Workspace context verbose** | Add tree view (Task 044) | HIGH |

## Terminology Mapping

### Code Terminology (Articulate & Precise)
| Current Term | New Term | Reason |
|-------------|----------|--------|
| `Step::Shell` (read-only) | `Step::Observe` or `Step::Read` | Explicit about operation type |
| `Step::Shell` (command) | `Step::Execute` | Clear it runs commands |
| `Step::Select` | `Step::Search` | Search is more accurate |
| `Step::Edit` | Keep as `Step::Edit` | Transform+Write split deferred |
| `Step::Reply` | Keep as `Step::Reply` | Infer+Report split deferred |
| `speech_act` | `intent` | More accurate |
| `formula` | `pattern` | Less ambiguous |

### Prompt Terminology (Principle-Based)
| Concept | Prompt Wording |
|---------|---------------|
| **INVESTIGATE** | "Use when YOU cannot determine what to do without first exploring or clarifying" |
| **OBSERVE** | "Inspect metadata without consuming content" |
| **READ** | "Consume content of a specific target" |
| **SEARCH** | "Retrieve candidates by query or predicate" |
| **EXECUTE** | "Run programs/tools/processes" |
| **WRITE** | "Materialize workspace changes" |
| **FETCH** | "Access information outside workspace boundary (DISABLED)" |
| **REPORT** | "Return user-facing output" |

## Security Constraints

### FETCH Operation (Internet Access)
```rust
// MUST be disabled with warning
#[deprecated(note = "FETCH operation is disabled for security. \
    Internet access requires explicit user consent and sandboxing. \
    See security audit #FETCH-001 for requirements.")]
pub(crate) async fn execute_fetch(...) {
    // Implementation exists but never called
    panic!("FETCH operation is disabled");
}
```

### Why Disabled
1. No sandboxing for external requests
2. No authentication/credential handling
3. No rate limiting or abuse prevention
4. No content validation for downloaded data
5. Requires security audit before enabling

## Migration Sequence

### Phase 1: Critical Fixes (Week 1-2)
1. Enable reflection for all tasks (not just non-DIRECT)
2. Fix speech act classification prompts (principles only)
3. Ground critics in actual output (stop hallucination)
4. Fix complexity assessment for INVESTIGATE

### Phase 2: Add Missing Step Types (Week 3-4)
1. Add `Step::Read` (read-only file access)
2. Add `Step::Search` (query-based retrieval)
3. Add `Step::Observe` (metadata inspection)
4. Add `Step::Fetch` (DISABLED with warning)

### Phase 3: Workspace Context (Week 5)
1. Implement Task 044 (tree view with `ignore` crate)
2. Update workspace brief format
3. Add file importance heuristics

### Phase 4: Verification Enhancements (Week 6-7)
1. Add preconditions to step types
2. Add postconditions to step types
3. Add file state validation before edits
4. Add test-after for edits

### Phase 5: Formula Updates (Week 8-9)
1. Update formula definitions to use new step types
2. Add new formulas for new operations
3. Deprecate old formulas (with shims)

### Phase 6: Rollout (Week 10-12)
1. Enable new step types for 10% of requests
2. Monitor metrics
3. Enable for 50% if stable
4. Enable for 100% if metrics improve
5. Deprecate old step types after 4 weeks

## Success Metrics

| Metric | Baseline | Target | Measurement |
|--------|----------|--------|-------------|
| **Routing accuracy** | ~75% | >90% | Scenario tests |
| **Retry rate** | ~25% | <10% | Session metrics |
| **Critic hallucination** | ~15% | <5% | Trace analysis |
| **Token usage** | Baseline | -20% | Session metrics |
| **Execution time** | Baseline | -15% | Session metrics |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Breaking existing sessions** | MEDIUM | HIGH | Backward-compatible step conversion |
| **FETCH security vulnerability** | HIGH | CRITICAL | Keep disabled, add compile-time warning |
| **Model confusion with new types** | MEDIUM | MEDIUM | Gradual rollout, metrics monitoring |
| **Prompt drift** | MEDIUM | MEDIUM | Principle-based prompts, no hardcoded rules |
| **Performance regression** | LOW | MEDIUM | Metrics monitoring, rollback capability |

## References
- Architectural Audit Report (this session)
- AGENTS.md - Elma philosophy
- QWEN.md - Agent guidelines
- Task 044 - Workspace context optimization
- ../PLANNING_AND_TASKS/TASKS.md
5 - Tool discovery (already implemented)
