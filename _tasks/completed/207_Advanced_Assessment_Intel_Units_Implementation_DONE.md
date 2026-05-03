# Task 207: Advanced Assessment Intel Units Implementation

## Objective
Implement intel units for fine-grained domain difficulty assessments, freshness requirements, assumption tracking, and edge case evaluation to enhance elma-cli's reasoning quality and response accuracy.

## Background
While elma-cli has basic complexity and risk assessment, it lacks specialized units for:
- Domain difficulty beyond general risk levels (e.g., distinguishing medical/legal vs. niche technical knowledge)
- Freshness requirements (e.g., flagging requests needing current data like prices/APIs)
- Assumption tracking (what assumptions are made, their risks, and dependencies)
- Edge case identification (failure modes, rare scenarios, hidden dependencies)

The brainstorming data identifies these in sections 1.3 (domain difficulty), 1.5 (freshness), 6.1 (assumptions), and 6.4 (edge cases) as gaps affecting response quality.

## Requirements
1. **New Intel Units**:
   - `DomainDifficultyUnit`: Classifies domain expertise level and knowledge requirements (maps to 1.3 domain difficulty)
   - `FreshnessRequirementUnit`: Identifies information currency needs and staleness risks (maps to 1.5 freshness requirement)
   - `AssumptionTrackerUnit`: Tracks assumptions made, their validity, and change impacts (maps to 6.1 assumptions)
   - `EdgeCaseEvaluatorUnit`: Identifies potential failure modes, exceptions, and dependencies (maps to 6.4 edge cases)

2. **Integration**:
   - Units run during orchestration planning (after complexity assessment, before scope building)
   - Outputs enhance `RiskClassifierUnit`, `FormulaSelectorUnit`, and reviewer/verification logic
   - Add assessment fields to `ComplexityAssessment` and `ScopePlan` structs

3. **Output Schema**:
   - Each unit returns JSON per docs/INTEL_UNIT_STANDARD.md standard
   - `DomainDifficultyUnit`: {domain_type: "common|specialized|expert|niche", knowledge_level: "basic|intermediate|advanced", sensitive: boolean, expertise_required: "none|general|specific"}
   - `FreshnessRequirementUnit`: {freshness_needed: "stable|moderate|high", staleness_risk: "low|medium|high", update_frequency: "rare|occasional|frequent", sources: ["api","news","docs"]}
   - `AssumptionTrackerUnit`: {assumptions: [{text: "string", risk: "low|medium|high", dependency: "string", change_impact: "minor|major|critical"}], needs_verification: boolean}
   - `EdgeCaseEvaluatorUnit`: {edge_cases: [{scenario: "string", likelihood: "rare|possible|likely", impact: "minor|moderate|severe", mitigation: "string"}], failure_modes: ["mode1","mode2"], hidden_deps: ["dep1","dep2"]}

4. **Narrative Building**:
   - Create `intel_narrative_advanced.rs` with functions: `build_domain_difficulty_narrative`, `build_freshness_requirement_narrative`, `build_assumption_tracker_narrative`, `build_edge_case_evaluator_narrative`
   - Narratives include user message, route decision, complexity assessment, workspace facts, conversation excerpt

5. **Fallbacks and Validation**:
   - Implement fallback outputs (e.g., default to "general" domain, "stable" freshness)
   - Add post-flight validation for required fields and schema compliance
   - Unit timeouts: 45 seconds max (higher for analysis depth)

6. **Testing and Verification**:
   - Add unit tests in `src/intel_units/intel_units_advanced.rs`
   - Integration tests verifying assessments influence risk scoring and scope planning
   - Real CLI testing with diverse queries (e.g., medical advice, API-dependent requests, complex assumptions)

## Implementation Steps
1. Define unit structs and impl IntelUnit in new file `src/intel_units/intel_units_advanced.rs`
2. Add narrative functions in `src/intel_narrative_advanced.rs`
3. Update `ComplexityAssessment` and `ScopePlan` to include advanced assessment fields
4. Modify `orchestration_planning.rs` to run advanced units in planning pipeline
5. Integrate outputs into risk assessment, formula selection, and reviewer logic
6. Add fallbacks, validations, and error handling
7. Test and verify with stress scenarios

## Success Criteria
- Advanced units provide granular, actionable assessments improving decision-making
- Orchestration adapts formulas based on domain difficulty (e.g., conservative for sensitive domains)
- Freshness flagging triggers appropriate evidence gathering for time-sensitive requests
- Assumption tracking enables better verification and caveat inclusion
- Edge case evaluation reduces response failures and improves robustness
- No performance degradation in orchestration pipeline

## Dependencies
- Builds on existing complexity assessment and intel framework
- May require updates to `src/types_core.rs` for new struct fields

## Estimated Effort
High (4-5 days): Unit implementation (2 days), integration and schema work (2 days), testing (1 day)

## Notes
- Prioritize reliability: Units should fail gracefully without blocking orchestration
- Align with elma-cli principles: bounded autonomy, evidence-grounded answers
- Use entropy/margin in outputs for confidence signaling
- Document new assessment patterns in project guidance