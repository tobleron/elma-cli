# Concise Task Summary (Completed, Older Tasks)

## JSON Reliability

- **001_Hybrid_JSON** — Implemented 5-layer defense-in-depth JSON reliability (GBNF grammar, few-shot examples, auto-repair, schema validation, fallback values); superseded by masterplan.
- **001_JSON_Reliability_Masterplan** — Built complete JSON reliability pipeline across 7 phases: grammar infrastructure, injection, schema validation, auto-repair, repair intel units, few-shot examples; achieved 99.9% parse success.
- **003_Complete_JSON_Fallback_Integration** — Created unified error handler module with circuit breaker, safe defaults for all components, user-facing error messages, and global fallback tracking.
- **008_Harden_OODA_Loop_And_Critic_JSON** — Planned retry-loop strategy diversification, critic JSON hardening, and reasoning path sanitization; pending architecture updates.
- **008_JSON_Reliability_Pipeline** — Completed 3-phase JSON pipeline: circuit breaker + fallbacks, content grounding for hallucination detection, schema validation + deterministic fix + 4 intel units (text generator, converter, verifier, repair).
- **013_Verify_JSON_Pipeline_For_Small_Models** — Investigated whether intel units use plain-text-first generation vs direct JSON for 3B model reliability; identified gaps and recommended plain-text extraction pipeline.
- **018_Improve_JSON_Repair** — Enhanced multi-stage JSON repair with aggressive extraction, validation, improved json_outputter prompt, and repair metrics; superseded.
- **T044_Eliminate_Critic_JSON** — Replaced critic JSON output with simple text format (`ok: reason` / `retry: reason`) to eliminate parse errors in verification loop.

## Intel Units

- **006_Extend_Narrative_To_All_Intel_Units** — Migrated all intel units from noisy JSON blobs to plain-text narrative input; improved model reasoning consistency across critic, sufficiency, reviewers, and evidence modes.
- **012_Review_Intel_Unit_Atomicity** — Assessed intel units for atomicity and 3B model suitability; identified loaded units needing splitting into single-responsibility atomic units.
- **034_Formalize_Intel_Unit_Interfaces** — Created `IntelUnit` trait with pre-flight/post-flight/fallback interface, `IntelContext` and specialized output types; 6 ladder profiles created; phase 2 deferred.
- **T001_verify_and_delete_obsolete_intel_units** — Verified 16 identified intel units are all in use or tested; none were obsolete; all retained.

## Classification & Routing

- **002_Fix_Speech_Act_Classification** — Updated speech_act prompts to principle-based distinctions (INSTRUCTION changes state, INFO provides answer, CHAT is conversation); superseded by 007.
- **007_Decouple_Classification_From_Execution** — Proposed treating classifier outputs as probabilistic feature vectors rather than hard decisions; enables orchestrator reasoning over rigid rule-following.
- **010_Elma_Helper_Intention_Clarification** — Implemented intention clarifier intel unit that runs before speech act classification, translating ambiguous user input into actionable language (ACTION/INFO/CHAT prefix).
- **012_Entropy_Based_Flexibility** — Added entropy calculation and noise injection to routing outputs to prevent overconfident 100% distributions; superseded.
- **014_Confidence_Based_Routing** — Implemented obvious-chat pattern detection and confidence-based fallback (defaults to safe CHAT route when entropy > 0.8 or margin < 0.15).
- **T001_Terminology_Broke_Classification** — Diagnosed and resolved classification breakage caused by Task 045 terminology overhaul (CHAT->CONVERSATION etc.) that desynchronized router model from code mappings; reverted to original terms.
- **T045_Fix_Info_Instruction_Classification** — Fixed misclassification of implicit action requests (e.g., "what is current date?") as INFO instead of INSTRUCTION; added prompt guidance and post-classification override for shell-command-requiring questions.

## Planning & Formulas

- **001_Revise_And_Perfect_Existing_Formulas** — Transformed formulas from hardcoded command lists into abstract intent patterns with cost/value/risk scoring and runtime efficiency tracking.
- **004_Revise_Core_Formulas_Reply_Family** — Revised 6 reply-family formulas (reply_only, capability_reply, execute_reply, inspect_reply, inspect_summarize_reply, inspect_decide_reply) with principle-based prompts.
- **005_Revise_Core_Formulas_Plan_Family** — Revised 4 plan-family formulas (plan_reply, masterplan_reply, cleanup_safety_review, code_search_and_quote) with principle-based prompts and hierarchical decomposition integration.
- **010_Multi_Strategy_Planning_With_Fallback_Chains** — Implemented strategy chain system (Direct, InspectFirst, PlanThenExecute, SafeMode, Incremental, Delegated) with automatic fallback on failure and strategy effectiveness logging.
- **024_Revise_And_Perfect_Existing_Formulas** — Umbrella task for iterative trial-and-error refinement of all shipped formulas; defined use cases, evidence patterns, failure modes per formula; superseded.

## Execution & Ladder

- **011_State_Aware_Guardrails** — Implemented context drift monitor that compares current Program/StepResult against original Goal at each OODA step, triggering mandatory refinement on divergence.
- **023_Hierarchical_Decomposition** — Implemented complexity-triggered hierarchical decomposition: OPEN_ENDED tasks generate masterplans (3-5 phases), saved to session; prevents massive single-step commands.
- **023_Implement_Complexity_Triggered_Hierarchical_Decomposition** — Original planning document for hierarchical decomposition with 5-level hierarchy (Goal->Subgoal->Task->Method->Action) and parent-child tracking.

## Reflection & Reasoning

- **001_Enable_Reflection_For_All_Tasks** — Removed `should_skip_intel()` check so reflection runs for ALL tasks regardless of complexity, catching hallucination before execution even for simple DIRECT tasks.

## UI & UX

- **007_Optimize_Workspace_Context** — Replaced basic find/ls workspace context with structured tree view (3 levels deep) using Rust `ignore` crate; filters noise, highlights important files, reduces tokens by 30%+.

## Infrastructure

- **009_Align_Tuning_With_Current_Runtime_Architecture** — Established safe tuning policy: captured llama.cpp runtime defaults as protected baseline, added variance penalties, unit-type-specific parameter bands, model behavior integration; prohibited prompt mutation.
- **015_Autonomous_Tool_Discovery** — Implemented CLI tool discovery using `which` crate with caching (7-day TTL, PATH-based invalidation); detects 40+ tools, project-specific tools, and custom scripts.
- **T059_Troubleshoot_Direct_Shell_Path_And_Runtime_Stalls** — Fixed 7 root causes of CLI instability: broken startup config, direct-shell fast path, Unicode truncation panics, chat path stalls, selector contract issues, placeholder handoff bugs, and evidence-free DECIDE hallucination; verified all 12 stress scenarios in real CLI mode.
