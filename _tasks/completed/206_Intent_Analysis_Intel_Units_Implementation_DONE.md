# Task 206: Intent Analysis Intel Units Implementation

## Objective
Implement detailed intent analysis intel units beyond basic routing to capture real intent, user expectations, and user effort levels, enhancing orchestration accuracy and user alignment in elma-cli.

## Background
Current elma-cli routing uses basic speech act classification (CHAT, INSTRUCT, INQUIRE) and workflow modes (INSPECT, EXECUTE, PLAN, etc.), but lacks granular intent analysis for:
- Real intent behind surface requests
- User expectations (practical advice vs. theory, brevity vs. depth)
- User effort levels (quick answers vs. deep analysis)

This leads to suboptimal formula selection and response structures. The brainstorming data (section 2: INTENT QUESTIONS) identifies these gaps.

## Requirements
1. **New Intel Units**:
   - `IntentSurfaceUnit`: Analyzes literal request, output type, and format preferences (maps to 2.1 Surface intent)
   - `IntentRealUnit`: Infers underlying problem/goal, decision-making needs, and frustration drivers (maps to 2.2 Real intent)
   - `UserExpectationUnit`: Determines expectations for advice type, depth, certainty, and next steps (maps to 2.3 User expectation)

2. **Integration**:
   - Units run early in orchestration planning (after routing, before complexity assessment)
   - Outputs feed into `ComplexityAssessmentUnit`, `FormulaSelectorUnit`, and `ResultPresenterUnit`
   - Add intent fields to `IntelContext` and `RouteDecision` structs

3. **Output Schema**:
   - Each unit returns JSON with standardized fields (choice, label, reason, entropy per docs/INTEL_UNIT_STANDARD.md)
   - `IntentSurfaceUnit`: {surface_intent: "question|task|advice", output_type: "explanation|list|command|code", format_pref: "paragraph|table|concise"}
   - `IntentRealUnit`: {real_intent: "debug|learn|build|compare|safety", problem_type: "specific|general", decision_needed: boolean}
   - `UserExpectationUnit`: {expectation_type: "practical|theory", depth_level: "quick|deep", certainty_pref: "high|probabilistic", effort_level: "low|high"}

4. **Narrative Building**:
   - Create `intel_narrative_intent.rs` with functions: `build_surface_intent_narrative`, `build_real_intent_narrative`, `build_user_expectation_narrative`
   - Narratives include user message, route decision, workspace context, conversation excerpt

5. **Fallbacks and Validation**:
   - Implement fallback outputs for each unit (e.g., default to neutral intent)
   - Add post-flight validation for required fields
   - Unit timeouts: 30 seconds max

6. **Testing and Verification**:
   - Add unit tests in `src/intel_units/intel_units_intent.rs`
   - Integration tests in orchestration loop verifying intent influences formula selection
   - Verify with real CLI scenarios (e.g., "how do I debug this?" vs. "explain debugging")

## Implementation Steps
1. Define unit structs and impl IntelUnit in new file `src/intel_units/intel_units_intent.rs`
2. Add narrative functions in `src/intel_narrative_intent.rs`
3. Update `IntelContext` and `RouteDecision` to include intent fields
4. Modify `orchestration_planning.rs` to run intent units early in `derive_planning_prior`
5. Update downstream units (complexity, formula selector, result presenter) to consume intent data
6. Add fallbacks and validations
7. Test and verify

## Success Criteria
- Intent units produce consistent, accurate outputs across diverse user messages
- Orchestration selects better formulas based on intent (e.g., deep analysis for learning requests)
- Real CLI testing shows improved response relevance and user satisfaction
- No regressions in existing routing/complexity assessment

## Dependencies
- Relies on existing intel trait and orchestration framework
- Requires updates to `src/app.rs` for new context fields

## Estimated Effort
Medium (2-3 days): Unit implementation (1 day), integration (1 day), testing (0.5 day)

## Notes
- Align with elma-cli philosophy: reliability over speed, grounded answers
- Follow intel unit standard for outputs
- Document in AGENTS.md if new patterns emerge