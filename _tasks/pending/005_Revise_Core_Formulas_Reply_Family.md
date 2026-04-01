# Task 049: Revise Core Formulas (Reply Family)

## Priority
**P1 - HIGH** (Most frequently used formulas, highest impact)

## Context
This task updates Task 001 with specific focus on the **Reply Family** of formulas:
- `reply_only` - Direct conversational response
- `capability_reply` - Respond to capability questions
- `execute_reply` - Execute action then respond
- `inspect_reply` - Inspect then respond
- `inspect_summarize_reply` - Inspect, summarize, then respond
- `inspect_decide_reply` - Inspect, decide, then respond

These are the MOST USED formulas - improvements here have maximum impact.

## Objective
Improve reliability and accuracy of reply formulas through principle-based prompt refinement.

## Work Items

### 1. Document Current Formulas
For each formula, define:
- **Principle:** When to use (not hardcoded rules)
- **Expected evidence pattern:** What workspace data is needed
- **Expected reply pattern:** What the response looks like
- **Common failure modes:** What goes wrong

Example:
```
Formula: inspect_summarize_reply
Principle: Use when user asks about something that requires inspecting workspace content and synthesizing an answer
Expected evidence: File contents, directory structure, or code patterns
Expected reply: Summary that answers user's question using evidence
Failure modes: Over-inspection, summarizing wrong files, missing key content
```

### 2. Create Validation Scenarios
For each formula, create 3-5 test scenarios in `scenarios/formula_validation/`:
- `reply_only_01_greeting.md`
- `reply_only_02_farewell.md`
- `capability_reply_01_can_you.md`
- `execute_reply_01_list_files.md`
- etc.

### 3. Run Baseline Evaluation
Run each scenario and record:
- Formula selected (correct/incorrect)
- Steps generated (appropriate/inappropriate)
- Final reply quality (1-5 scale)
- Retry count

### 4. Refine Prompts (Principle-Based)
Update formula prompts in `src/defaults_evidence.rs`:

**BEFORE (hardcoded):**
```toml
system_prompt = """
Use inspect_summarize_reply when:
- User asks "what is in this file"
- User asks "summarize the project"
- User asks about code structure
"""
```

**AFTER (principle-based):**
```toml
system_prompt = """
Use inspect_summarize_reply when:
- User asks about content YOU don't already know
- Answer requires inspecting workspace files
- Response should synthesize findings into coherent summary

Principle: INSPECT to gather evidence, SUMMARIZE to synthesize, REPLY to answer
"""
```

### 5. Re-Validate
Run same scenarios after refinements. Compare:
- Accuracy improvement
- Retry rate reduction
- Reply quality improvement

## Acceptance Criteria
- [ ] All 6 reply formulas documented with principles
- [ ] 18+ validation scenarios created (3 per formula)
- [ ] Baseline metrics recorded
- [ ] Prompts updated to principle-based (no hardcoded examples)
- [ ] Post-refinement metrics show improvement
- [ ] At least one before/after example per formula

## Expected Impact
- **+20% formula selection accuracy** (right formula for task)
- **-30% retry rate** (fewer wrong-formula failures)
- **+15% reply quality** (better evidence synthesis)

## Dependencies
- Task 046 (speech act classification) - affects formula triggering
- Task 047 (Read/Search step types) - formulas may use new types

## Verification
- `cargo build`
- `cargo test`
- Run validation scenarios
- Compare before/after metrics

## Architecture Alignment
- ✅ Principle-based prompts (AGENTS.md/QWEN.md compliance)
- ✅ Articulate terminology (formulas clearly defined)
- ✅ Enables autonomous reasoning (model applies principles)
