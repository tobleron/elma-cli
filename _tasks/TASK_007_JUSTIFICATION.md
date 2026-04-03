# Task 007 Justification — Decouple Classification From Execution

## Current State Analysis

### What Exists (Infrastructure)
- ✅ `ClassificationFeatures` struct exists in `src/types_core.rs`
- ✅ Contains probability distributions for speech_act, workflow, mode, route
- ✅ Contains entropy field for uncertainty measurement
- ✅ `From<&RouteDecision>` conversion implemented
- ✅ Passed to `reflect_on_program()` function

### What's NOT Working (The Gap)

**ClassificationFeatures is CREATED but NOT USED:**

```rust
// In src/app_chat_core.rs line 206:
let features = ClassificationFeatures::from(&route_decision);

// In src/reflection.rs line 82:
fn build_reflection_prompt(
    program: &Program,
    _priors: &ClassificationFeatures,  // ← UNDERSCORE = UNUSED!
    workspace: &str,
    objective: &str,
) -> String {
    // Features are NOT used in the prompt!
    // Only program, workspace, and objective are used
}
```

**Current flow is STILL hard-decision based:**
```
User Input → Classifiers → RouteDecision (hard choice) → Formula → Program
                              ↓
                         features created but ignored
```

**Should be soft-guidance flow:**
```
User Input → Classifiers → ClassificationFeatures (probs) → Orchestrator → Program
                              ↓
                         entropy triggers level escalation
```

---

## Why This Task Matters

### Problem 1: Overconfident Classifications Go Unchallenged

**Current behavior:**
```
route=CHAT p=1.00 margin=1.00 entropy=0.00
→ Hard decision: CHAT
→ No reasoning about alternatives
→ Even if request clearly needs execution
```

**With Task 007:**
```
route_dist=CHAT:0.60 SHELL:0.30 PLAN:0.10
entropy=0.89 (high uncertainty!)
→ Features passed to orchestrator
→ Orchestrator sees: "Model is uncertain, consider alternatives"
→ May choose SHELL despite CHAT being top prediction
```

### Problem 2: Execution Ladder Can't Use Entropy for Escalation

**Task 044 implemented escalation heuristics:**
```rust
// In src/execution_ladder.rs:
if route_decision.entropy > 0.8 {
    if level < ExecutionLevel::Task {
        level = ExecutionLevel::Task;  // Escalate!
    }
}
```

**But this uses `RouteDecision.entropy`, not the full feature vector.**

**With Task 007:**
```rust
// Full feature vector enables better escalation:
if features.entropy > 0.8 || features.route_probs[0].1 < 0.6 {
    // High entropy OR low confidence in top choice
    escalate_level();
}

// Can also check for close calls:
let top_two_margin = features.route_probs[0].1 - features.route_probs[1].1;
if top_two_margin < 0.2 {
    // Close call between top two routes
    escalate_level();
}
```

### Problem 3: Orchestrator Can't Override Bad Classifications

**Current behavior:**
- Classifier says CHAT → Formula = `reply_only`
- Even if user said "run cargo test"
- No mechanism for orchestrator to say "this feels wrong"

**With Task 007:**
```toml
# Orchestrator prompt (updated):
Classification Features (use as EVIDENCE, not rules):
- Speech act: ACTION_REQUEST (55%), INFO_REQUEST (30%), CHAT (15%)
- Route: SHELL (45%), CHAT (40%), PLAN (15%)
- Entropy: 0.92 (HIGH UNCERTAINTY)

These are probabilistic signals. Reason about whether they apply.
If the user message clearly contradicts the top prediction, choose appropriately.
```

**Result:**
- Orchestrator sees uncertainty (entropy=0.92)
- Sees close call (SHELL 45% vs CHAT 40%)
- Can override and choose SHELL for "run cargo test"

---

## What Task 007 Will Actually Do

### Step 1: Use Features in Reflection Prompt

**Current (unused):**
```rust
fn build_reflection_prompt(
    program: &Program,
    _priors: &ClassificationFeatures,  // ← IGNORED
    workspace: &str,
    objective: &str,
) -> String
```

**After (used):**
```rust
fn build_reflection_prompt(
    program: &Program,
    priors: &ClassificationFeatures,  // ← NOW USED
    workspace: &str,
    objective: &str,
) -> String {
    format!(
        r#"Evaluate this program.

Classification Context (soft guidance, not rules):
- Speech act: {} (entropy: {:.2})
- Route distribution: {}
- Margin: {:.2}

If classification is uncertain (high entropy, low margin), mention this in your concerns.
If the program doesn't match the top classification but makes sense, say so.

Program to evaluate:
{:?}
"#,
        priors.speech_act_probs[0].0,
        priors.entropy,
        format_distribution(&priors.route_probs),
        priors.margin,
        program,
    )
}
```

### Step 2: Update Orchestrator Prompt

**Current:**
```toml
system_prompt = """
Generate a program to achieve the objective.
Use the formula: {formula}
"""
```

**After:**
```toml
system_prompt = """
Generate a program to achieve the objective.

Classification Features (EVIDENCE, not rules):
- Speech act: {speech_act_dist}
- Route: {route_dist}
- Entropy: {entropy}
- Margin: {margin}

Principles:
- If entropy is high (>0.8), the classifier is uncertain — use your own judgment
- If margin is low (<0.2), top choices are close — consider alternatives
- If user message clearly contradicts top prediction, override the classification
- Formula suggestions are starting points, not constraints

Formula suggestion: {formula}
Objective: {objective}
"""
```

### Step 3: Add Entropy-Based Level Escalation

**Already in Task 044, but can be enhanced:**

```rust
// Current (uses RouteDecision.entropy):
if route_decision.entropy > 0.8 {
    escalate_level();
}

// Enhanced (uses full ClassificationFeatures):
if features.entropy > 0.8 {
    escalate_level();  // High uncertainty
}

// Check for close calls
let top_route_prob = features.route_probs.get(0).map(|(_, p)| *p).unwrap_or(0.0);
let second_route_prob = features.route_probs.get(1).map(|(_, p)| *p).unwrap_or(0.0);
if top_route_prob - second_route_prob < 0.2 {
    escalate_level();  // Close call
}

// Check for speech act / route mismatch
if features.speech_act_probs[0].0 == "ACTION_REQUEST" && 
   features.route_probs[0].0 == "CHAT" {
    escalate_level();  // Mismatch suggests uncertainty
}
```

---

## Expected Impact

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| **Classification overrides** | 0% | ~15% of uncertain cases | Better handling of edge cases |
| **Entropy-based escalation** | Partial (route only) | Full (all features) | Better level selection |
| **Reflection quality** | Generic | Classification-aware | Catches classification issues |
| **Model autonomy** | Low (follows classification) | High (reasons about features) | Aligns with Elma philosophy |

---

## Why This Task is P1 (High Priority)

### 1. Completes Task 044's Vision

Task 044 (Execution Ladder) implemented level-based orchestration, but:
- Ladder uses `RouteDecision` (hard choice)
- Ladder should use `ClassificationFeatures` (soft features)

**Task 007 completes what Task 044 started.**

### 2. Aligns with Elma Philosophy

From AGENTS.md:
> "treating classification signals as soft guidance rather than hard constraints"

**Current code:** Classification = hard constraint
**After Task 007:** Classification = soft guidance ✅

### 3. Enables Better Error Recovery

When classification is wrong:
- **Before:** System follows wrong classification → failure
- **After:** System sees uncertainty → escalates level → catches error earlier

### 4. Low Risk, High Reward

**Risk:** Low
- Infrastructure already exists (`ClassificationFeatures`)
- Changes are additive (add feature usage, don't remove existing)
- Can be tested incrementally

**Reward:** High
- +15-25% better handling of uncertain classifications
- Better alignment with Elma philosophy
- Completes Task 044's architecture

---

## Effort Estimate

| Step | Hours | Complexity |
|------|-------|------------|
| Update reflection prompt to use features | 1h | Low |
| Update orchestrator prompt | 1h | Low |
| Enhance entropy-based escalation | 2h | Medium |
| Update call sites (app_chat_core, tuning) | 1h | Low |
| Testing and verification | 1h | Low |
| **Total** | **6h** | **Medium** |

---

## Acceptance Criteria

- [ ] `ClassificationFeatures` actually used in reflection prompt (not `_priors`)
- [ ] Orchestrator prompt receives and uses feature distributions
- [ ] Entropy-based escalation uses full feature vector
- [ ] Traces show feature distributions being logged
- [ ] Model can override classification when appropriate
- [ ] Test case: uncertain classification → escalation triggers

---

## Relationship to Other Tasks

| Task | Relationship |
|------|--------------|
| **Task 044** (Execution Ladder) | Task 007 provides features ladder should use |
| **Task 010** (Entropy Flexibility) | Task 007 uses entropy for escalation |
| **Task 001** (Reflection) | Task 007 enhances reflection with classification context |

---

## Bottom Line

**Task 007 is the "last mile" of Task 044.**

Task 044 built the execution ladder infrastructure, but it's still using hard classification decisions (`RouteDecision`) instead of soft features (`ClassificationFeatures`).

**Task 007 connects the dots:**
- Infrastructure exists ✅
- Just not being used ❌
- Task 007 activates it ✅

**6 hours of work to unlock:**
- Better handling of uncertain classifications
- True "soft guidance" as per Elma philosophy
- Completion of Task 044's architectural vision

---

## Recommendation

**DO THIS TASK NEXT** (after Task 006 postponed):

1. ✅ Low risk (infrastructure exists)
2. ✅ High reward (completes Task 044)
3. ✅ Aligns with philosophy (soft guidance over hard rules)
4. ✅ Enables future tasks (010, 042)

**Estimated:** 6 hours
**Impact:** +15-25% better uncertain classification handling
