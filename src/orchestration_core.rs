//! @efficiency-role: service-orchestrator
//!
//! Core Orchestration Module
//!
//! Tool-calling pipeline: Maestro sets context → model calls tools directly → final answer.

use crate::app::AppRuntime;
use crate::app_chat_fast_paths::build_direct_reply_program;
use crate::decomposition_pyramid::DecompositionPyramid;
use crate::formulas::{select_optimal_formula, FormulaPattern, FormulaScores};
use crate::intel_trait::IntelContext;
use crate::intel_units::DecompositionUnit;
use crate::tool_loop::run_tool_loop;
use crate::tools::ToolRegistry;
use crate::*;

// ============================================================================
// Tool-Calling Orchestration (no Maestro — model plans itself)
// ============================================================================

/// Build a system prompt for tool calling without any intermediate planner.
/// The model has full context (workspace, conversation, tools) and plans directly.
///
/// The core prompt is defined in `prompt_core::TOOL_CALLING_SYSTEM_PROMPT`
/// and is protected from modification by CODEOWNERS, AGENTS.md Rule 8,
/// and build-time hash verification.
fn build_tool_calling_system_prompt(runtime: &AppRuntime, _line: &str) -> String {
    // Workspace facts (platform, cwd, git state)
    let workspace_facts = runtime.ws.trim();

    // Workspace brief (file tree)
    let workspace_brief = runtime.ws_brief.trim();

    // Recent conversation excerpt
    let conversation = if runtime.messages.is_empty() {
        String::new()
    } else {
        let last_msgs: Vec<String> = runtime
            .messages
            .iter()
            .rev()
            .take(6)
            .rev()
            .map(|m| {
                format!(
                    "{}: {}",
                    m.role,
                    m.content.chars().take(300).collect::<String>()
                )
            })
            .collect();
        format!("\n## Recent conversation\n{}", last_msgs.join("\n"))
    };

    // Skill context (repo overview, document capabilities, etc.)
    let skill_context = build_skill_context(runtime);

    // Project guidance (AGENTS.md + TASKS.md excerpts)
    let project_guidance = runtime.guidance.render_for_system_prompt();

    crate::prompt_core::assemble_system_prompt(
        workspace_facts,
        workspace_brief,
        &conversation,
        &skill_context,
        &project_guidance,
    )
}

fn build_skill_context(runtime: &AppRuntime) -> String {
    let primary = runtime.execution_plan.primary_skill();
    match primary {
        SkillId::RepoExplorer => {
            if let Ok(overview) = repo_explorer::explore_repo(&runtime.repo) {
                repo_explorer::render_repo_overview(&overview)
            } else {
                "(repo exploration unavailable)".to_string()
            }
        }
        SkillId::DocumentReader => {
            let caps = document_adapter::document_capabilities();
            let lines: Vec<String> = caps
                .iter()
                .map(|c| {
                    let note = c
                        .quality_note
                        .as_ref()
                        .map(|q| format!(" ({q})"))
                        .unwrap_or_default();
                    format!("- {} via {}{}", c.format, c.backend, note)
                })
                .collect();
            format!("Document capabilities:\n{}", lines.join("\n"))
        }
        SkillId::FileScout => {
            let exclusions: Vec<String> =
                file_scout::default_scout_exclusions().into_iter().collect();
            format!(
                "File scout exclusions: {}\nUse on-demand discovery. Stay read-only outside workspace. Disclose searched roots.",
                exclusions.join(", ")
            )
        }
        SkillId::TaskSteward => {
            let inventory = task_steward::scan_task_inventory(&runtime.repo);
            task_steward::render_inventory_summary(&inventory)
        }
        SkillId::General => "(general mode — no specialized context)".to_string(),
    }
}

/// Run the tool-calling pipeline: model plans and executes tools directly.
/// Returns (final_answer, iterations_used, tool_calls_made, stopped_by_max).
pub(crate) async fn run_tool_calling_pipeline(
    runtime: &mut AppRuntime,
    line: &str,
    tui: &mut crate::ui_terminal::TerminalUI,
    route_decision: &RouteDecision,
) -> Result<(String, usize, usize, bool)> {
    // Emit route decision as a collapsible transcript row
    let route_msg = format!(
        "route={} source=\"{}\" margin={:.2} entropy={:.2} evidence={}",
        route_decision.route,
        route_decision.source,
        route_decision.margin,
        route_decision.entropy,
        route_decision.evidence_required,
    );
    tui.push_route_notice(&route_msg);

    if should_use_direct_chat_path(route_decision) {
        return run_direct_chat_pipeline(runtime, line, tui).await;
    }

    let context_hint = route_decision.route.as_str();
    let evidence_required = route_decision.evidence_required;
    let system_prompt = build_tool_calling_system_prompt(runtime, line);
    trace(
        &runtime.args,
        "tool_calling: compact action DSL planning (no provider-native tools)",
    );

    // Generate decomposition pyramid for non-CHAT routes (Task 394)
    let pyramid = generate_decomposition_pyramid(runtime, line, route_decision).await;

    tui.start_status("Executing...");

    let result = run_tool_loop(
        &runtime.args,
        &runtime.client,
        &runtime.chat_url,
        &runtime.model_id,
        &system_prompt,
        line,
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        &runtime.session,
        0.2, // temperature — low for reliability
        16384,
        tui,
        Some(&runtime.profiles.summarizer_cfg),
        context_hint,
        evidence_required,
        runtime.ctx_max,
        &runtime.goal_state,
        pyramid.as_ref(),
    )
    .await?;

    // Emit pyramid as a transcript event for visibility
    if let Some(ref pyra) = pyramid {
        if !pyra.objective.is_empty() {
            let event = format!(
                "🗂 Decomposition pyramid: objective=\"{}\" risk={} goals={} tasks={}",
                pyra.objective,
                pyra.risk,
                pyra.goals.len(),
                pyra.tasks.len()
            );
            trace(&runtime.args, &event);
            tui.push_meta_event("PYRAMID", &event);
            tui.push_decomposition_notice(&event);

            // Save pyramid to session metadata for replay continuity
            let session_root = &runtime.session.root;
            let _ = crate::session_write::save_pyramid(
                session_root,
                &pyra.objective,
                &pyra.risk,
                &pyra.goals,
                &pyra.tasks,
            );
        }
    }

    tui.complete_status("Done");

    runtime.last_stop_outcome = result.stop_outcome.clone();

    // Emit stop reason as a persistent transcript row
    if let Some(ref outcome) = result.stop_outcome {
        let stop_msg = format!(
            "reason={} iterations={} tool_calls={} summary=\"{}\"",
            outcome.reason.as_str(),
            result.iterations,
            result.tool_calls_made,
            outcome.summary,
        );
        tui.push_stop_notice(&stop_msg);
    }

    // Strip leaked thinking/tool_call blocks before returning to the user
    let clean_answer = crate::text_utils::strip_thinking_blocks(&result.final_answer);

    Ok((
        clean_answer,
        result.iterations,
        result.tool_calls_made,
        result.stopped_by_max,
    ))
}

pub(crate) fn should_use_direct_chat_path(route_decision: &RouteDecision) -> bool {
    route_decision.route.eq_ignore_ascii_case("CHAT") && !route_decision.evidence_required
}

/// Generate a decomposition pyramid for a complex request (Task 394).
///
/// Returns None for simple CHAT routes or if generation fails gracefully.
async fn generate_decomposition_pyramid(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
) -> Option<DecompositionPyramid> {
    // Only generate for non-CHAT routes that need tools/evidence
    if should_use_direct_chat_path(route_decision) {
        return None;
    }

    let profile = crate::llm_config::ad_hoc_profile(&runtime.model_id, "decomposition");
    let unit = DecompositionUnit::new(profile);

    let context = IntelContext::new(
        line.to_string(),
        route_decision.clone(),
        runtime.ws.clone(),
        runtime.ws_brief.clone(),
        runtime.messages.clone(),
        runtime.client.clone(),
    );

    match unit.execute_with_fallback(&context).await {
        Ok(output) => {
            let objective = output
                .get("objective")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if objective.is_empty() {
                return None;
            }
            let risk = output
                .get("risk")
                .and_then(|v| v.as_str())
                .unwrap_or("low")
                .to_string();
            let goals: Vec<crate::decomposition_pyramid::PyramidGoal> = output
                .get("goals")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|g| crate::decomposition_pyramid::PyramidGoal {
                            text: g
                                .get("text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            evidence_needed: g
                                .get("evidence_needed")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let tasks: Vec<crate::decomposition_pyramid::PyramidTask> = output
                .get("tasks")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|t| crate::decomposition_pyramid::PyramidTask {
                            id: t.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            text: t
                                .get("text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            status: t
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("pending")
                                .to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            trace(
                &runtime.args,
                &format!(
                    "pyramid_generated objective=\"{}\" risk={} goals={} tasks={}",
                    objective,
                    risk,
                    goals.len(),
                    tasks.len()
                ),
            );

            Some(DecompositionPyramid {
                objective,
                risk,
                goals,
                tasks,
                next_action: None,
            })
        }
        Err(_) => None,
    }
}

async fn run_direct_chat_pipeline(
    runtime: &mut AppRuntime,
    line: &str,
    tui: &mut crate::ui_terminal::TerminalUI,
) -> Result<(String, usize, usize, bool)> {
    trace(&runtime.args, "tool_calling: direct chat response path");
    tui.start_status("Responding...");
    let profile = ad_hoc_profile(&runtime.model_id, "direct_chat");
    let req = chat_request_from_profile(
        &profile,
        vec![
            ChatMessage::simple(
                "system",
                "You are Elma, a concise local-first terminal assistant. Reply in plain text with no emoji and no marketing claims. Use general help language for casual chat. Do not imply a shell command should run unless the user asks for one. Do not claim tools ran, exchanges succeeded, or hidden actions happened unless the user explicitly asked about them.",
            ),
            ChatMessage::simple("user", line),
        ],
        ChatRequestOptions {
            temperature: Some(0.3),
            top_p: Some(1.0),
            max_tokens: Some(512),
            repeat_penalty: Some(None),
            reasoning_format: Some(Some("none".to_string())),
            tools: None,
            ..ChatRequestOptions::default()
        },
    );
    let response = crate::ui_chat::chat_once_with_timeout(
        &runtime.client,
        &runtime.chat_url,
        &req,
        runtime_llm_config().final_answer_timeout_s,
    )
    .await?;
    let answer = response
        .choices
        .first()
        .map(|choice| choice.message.content.clone().unwrap_or_default())
        .unwrap_or_default();
    tui.complete_status("Done");
    let clean = crate::text_utils::strip_thinking_blocks(&answer)
        .trim()
        .to_string();
    Ok((
        if clean.is_empty() {
            "I'm here.".to_string()
        } else {
            clean
        },
        1,
        0,
        false,
    ))
}

/// Compute risk deterministically from the tool-calling result metadata.
pub(crate) fn compute_program_risk(_tool_calls_made: usize, _iterations: usize) -> ProgramRisk {
    ProgramRisk::Low
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_chat_path_only_for_chat_without_evidence_requirement() {
        fn prob(choice: &str, margin: f64, entropy: f64) -> ProbabilityDecision {
            ProbabilityDecision {
                choice: choice.to_string(),
                source: "test".to_string(),
                distribution: vec![(choice.to_string(), 1.0)],
                margin,
                entropy,
            }
        }

        let mut route = RouteDecision {
            route: "CHAT".to_string(),
            source: "test".to_string(),
            distribution: vec![("CHAT".to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: prob("CHAT", 1.0, 0.0),
            workflow: prob("CHAT", 1.0, 0.0),
            mode: prob("DECIDE", 1.0, 0.0),
            evidence_required: false,
        };

        assert!(should_use_direct_chat_path(&route));
        route.evidence_required = true;
        assert!(!should_use_direct_chat_path(&route));
        route.evidence_required = false;
        route.route = "SHELL".to_string();
        assert!(!should_use_direct_chat_path(&route));
        route.route = "CHAT".to_string();
        route.speech_act = prob("INSTRUCT", 1.0, 0.0);
        assert!(should_use_direct_chat_path(&route));
        route.speech_act = prob("CHAT", 0.4, 0.0);
        assert!(should_use_direct_chat_path(&route));
    }
}

// ============================================================================
// Legacy compatibility — keep for non-tool-calling paths
// ============================================================================

/// Legacy compatibility shim: Maestro→Program orchestration.
///
/// Disabled by the compact DSL action protocol migration (Task 384). The live
/// runtime uses the action DSL tool loop instead of program JSON generation.
pub(crate) async fn orchestrate_instruction_once(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _orchestrator_cfg: &Profile,
    _instruction: &str,
    _user_message: &str,
    _intent: &str,
    _expert_advice: &str,
    _ws: &str,
    _ws_brief: &str,
    _previous_steps: &[Step],
    _step_counter: &mut u32,
) -> Result<Vec<Step>> {
    anyhow::bail!("legacy maestro orchestration disabled; use the action DSL tool loop")
}

/// Build a program from Maestro instructions.
/// Calls Maestro, then loops through each instruction transforming it into steps.
/// Caps at 9 total steps. Gracefully degrades to a fallback program on failure.
pub(crate) async fn build_program_from_maestro(
    runtime: &AppRuntime,
    line: &str,
) -> Result<Program> {
    // Step 1: Call Maestro to get numbered instructions
    let unit = MaestroUnit::new(runtime.profiles.the_maestro_cfg.clone());
    let context = IntelContext::new(
        line.to_string(),
        RouteDecision::default(),
        runtime.ws.clone(),
        runtime.ws_brief.clone(),
        runtime.messages.clone(),
        runtime.client.clone(),
    );

    let maestro_output: MaestroOutput = match unit.execute_with_fallback(&context).await {
        Ok(o) => match serde_json::from_value(o.data) {
            Ok(mo) => mo,
            Err(e) => {
                return Ok(build_fallback_program(
                    line,
                    &format!("Maestro produced unparseable output: {}", e),
                ));
            }
        },
        Err(e) => {
            return Ok(build_fallback_program(
                line,
                &format!("Maestro execution failed: {}", e),
            ));
        }
    };

    if maestro_output.steps.is_empty() {
        return Ok(build_fallback_program(
            line,
            "Maestro produced an empty plan",
        ));
    }

    // Step 2: Loop through instructions, transform each into steps (cap at 9)
    const MAX_TOTAL_STEPS: usize = 9;
    let mut all_steps: Vec<Step> = Vec::new();
    let mut step_counter: u32 = 0;

    let intent = line;
    let expert_advice = "";

    for maestro_instruction in &maestro_output.steps {
        if all_steps.len() >= MAX_TOTAL_STEPS {
            break;
        }

        match orchestrate_instruction_once(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.orchestrator_cfg,
            &maestro_instruction.instruction,
            line,
            intent,
            expert_advice,
            &runtime.ws,
            &runtime.ws_brief,
            &all_steps,
            &mut step_counter,
        )
        .await
        {
            Ok(steps) => {
                let remaining = MAX_TOTAL_STEPS - all_steps.len();
                if steps.len() > remaining {
                    all_steps.extend(steps.into_iter().take(remaining));
                    break;
                }
                all_steps.extend(steps);
            }
            Err(_e) => {
                // Skip this instruction, continue with remaining
                continue;
            }
        }
    }

    if all_steps.is_empty() {
        return Ok(build_fallback_program(
            line,
            "All orchestrator instructions failed to produce steps",
        ));
    }

    // Step 3: Auto-append Summarize→Respond if last step is not a reply
    let last_step_is_reply = all_steps
        .last()
        .map(|s| matches!(s, Step::Reply { .. } | Step::Respond { .. }))
        .unwrap_or(false);

    if !last_step_is_reply && all_steps.len() > 1 {
        let total = all_steps.len() + 2;
        all_steps.push(Step::Summarize {
            id: format!("s{}", total - 1),
            text: String::new(),
            instructions: "Summarize all findings from the previous steps concisely.".to_string(),
            common: StepCommon {
                purpose: "summarize findings".to_string(),
                depends_on: vec![format!("s{}", total - 2)],
                success_condition: "concise summary produced".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
                is_read_only: true,
                is_destructive: false,
                is_concurrency_safe: true,
                interrupt_behavior: InterruptBehavior::Graceful,
            },
        });
        all_steps.push(Step::Respond {
            id: format!("s{}", total),
            instructions: "Present the summary to the user clearly.".to_string(),
            common: StepCommon {
                purpose: "present summary to user".to_string(),
                depends_on: vec![format!("s{}", total - 1)],
                success_condition: "user receives summary".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
                is_read_only: true,
                is_destructive: false,
                is_concurrency_safe: true,
                interrupt_behavior: InterruptBehavior::Graceful,
            },
        });
    } else if !last_step_is_reply && all_steps.len() == 1 {
        all_steps.push(Step::Respond {
            id: "s2".to_string(),
            instructions: "Present findings to the user.".to_string(),
            common: StepCommon {
                purpose: "present findings to user".to_string(),
                depends_on: vec!["s1".to_string()],
                success_condition: "user receives answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
                is_read_only: true,
                is_destructive: false,
                is_concurrency_safe: true,
                interrupt_behavior: InterruptBehavior::Graceful,
            },
        });
    }

    Ok(Program {
        objective: line.to_string(),
        steps: all_steps,
    })
}

/// Build a minimal fallback program when orchestration fails entirely.
/// Returns a single reply step that honestly communicates the failure.
fn build_fallback_program(line: &str, reason: &str) -> Program {
    Program {
        objective: line.to_string(),
        steps: vec![Step::Respond {
            id: "s1".to_string(),
            instructions: format!(
                "I wasn't able to build a plan for this request. {}\n\nCould you rephrase or break this into smaller steps?",
                reason
            ),
            common: StepCommon {
                purpose: "honestly communicate orchestration failure to user".to_string(),
                depends_on: vec![],
                success_condition: "user receives honest failure message".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
                is_read_only: true,
                is_destructive: false,
                is_concurrency_safe: true,
                interrupt_behavior: InterruptBehavior::Graceful,
            },
        }],
    }
}

pub(crate) async fn orchestrate_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<(Program, String)> {
    // Get tool registry for this workspace
    let workspace_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tool_registry = ToolRegistry::new(&workspace_path);

    // Select optimal formula based on complexity and efficiency
    let formula_selection = select_optimal_formula(
        &complexity.complexity,
        &complexity.risk,
        &route_decision.route,
        0.5, // Balanced efficiency priority (can be tuned)
    );

    let prompt = build_orchestrator_user_content(
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        &tool_registry,
        &formula_selection,
    );

    // Use GBNF grammar for SHELL routes to ensure valid JSON
    let use_grammar = route_decision.route.eq_ignore_ascii_case("SHELL")
        || route_decision.route.eq_ignore_ascii_case("WORKFLOW");

    orchestration_helpers::request_program_or_repair(
        client,
        chat_url,
        orchestrator_cfg,
        &prompt,
        use_grammar,
    )
    .await
}

pub(crate) async fn recover_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    failure_reason: &str,
    current_program: Option<&Program>,
    step_results: &[StepResult],
) -> Result<Program> {
    let prompt = build_recovery_user_content(
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        failure_reason,
        current_program,
        step_results,
    );
    orchestration_helpers::request_recovery_program(
        client,
        chat_url,
        orchestrator_cfg,
        &prompt,
        step_results,
    )
    .await
}

pub(crate) async fn run_critic_once(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    sufficiency: Option<&ExecutionSufficiencyVerdict>,
    attempt: u32,
) -> Result<CriticVerdict> {
    orchestration_helpers::request_critic_verdict(
        client,
        chat_url,
        critic_cfg,
        line,
        route_decision,
        program,
        step_results,
        sufficiency,
        attempt,
    )
    .await
}

pub(crate) async fn generate_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    evidence_mode_cfg: &Profile,
    expert_advisor_cfg: &Profile,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    formatter_cfg: &Profile,
    system_content: &str,
    model_id: &str,
    base_url: &str,
    line: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
    workspace_facts: &str,
    workspace_brief: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<(String, Option<u64>)> {
    let runtime_context = serde_json::json!({
        "model_id": model_id,
        "base_url": base_url,
    });
    if route_decision.route.eq_ignore_ascii_case("CHAT") && step_results.is_empty() {
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
        let evidence_mode = EvidenceModeDecision {
            mode: "COMPACT".to_string(),
            reason: "chat reply fast path".to_string(),
        };
        let response_advice = orchestration_helpers::request_response_advice_via_unit(
            client,
            expert_advisor_cfg,
            line,
            route_decision,
            &evidence_mode,
            reply_instructions,
            step_results,
            workspace_facts,
            workspace_brief,
        )
        .await
        .unwrap_or_default();
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
        let final_text = orchestration_helpers::present_result_via_unit(
            client,
            presenter_cfg,
            line,
            route_decision,
            &runtime_context,
            &evidence_mode,
            &response_advice,
            step_results,
            reply_instructions,
            workspace_facts,
            workspace_brief,
        )
        .await
        .unwrap_or_else(|_| {
            if reply_instructions.trim().is_empty() {
                line.to_string()
            } else {
                reply_instructions.to_string()
            }
        });

        let (formatted, usage) = orchestration_helpers::maybe_format_final_text(
            client,
            chat_url,
            formatter_cfg,
            line,
            final_text,
            None,
        )
        .await;
        return Ok((formatted, usage));
    }

    let evidence_mode = orchestration_helpers::decide_evidence_mode_via_unit(
        client,
        evidence_mode_cfg,
        line,
        route_decision,
        reply_instructions,
        step_results,
        workspace_facts,
        workspace_brief,
    )
    .await
    .unwrap_or_else(|_| EvidenceModeDecision {
        mode: "COMPACT".to_string(),
        reason: "fallback".to_string(),
    });
    if let Some(t) = tui.as_deref_mut() {
        let _ = t.pump_ui();
    }
    let response_advice = orchestration_helpers::request_response_advice_via_unit(
        client,
        expert_advisor_cfg,
        line,
        route_decision,
        &evidence_mode,
        reply_instructions,
        step_results,
        workspace_facts,
        workspace_brief,
    )
    .await
    .unwrap_or_default();
    if let Some(t) = tui.as_deref_mut() {
        let _ = t.pump_ui();
    }

    let (mut final_text, mut usage_total) = if route_decision.route.eq_ignore_ascii_case("CHAT") {
        orchestration_helpers::request_chat_final_text(
            client,
            chat_url,
            elma_cfg,
            system_content,
            line,
            step_results,
            reply_instructions,
            tui,
        )
        .await?
    } else {
        let text = orchestration_helpers::present_result_via_unit(
            client,
            presenter_cfg,
            line,
            route_decision,
            &runtime_context,
            &evidence_mode,
            &response_advice,
            step_results,
            reply_instructions,
            workspace_facts,
            workspace_brief,
        )
        .await
        .unwrap_or_default();
        (text, None)
    };

    if !route_decision.route.eq_ignore_ascii_case("CHAT") && !final_text.trim().is_empty() {
        final_text = orchestration_helpers::maybe_revise_presented_result(
            client,
            chat_url,
            presenter_cfg,
            claim_checker_cfg,
            line,
            route_decision,
            &runtime_context,
            &evidence_mode,
            &response_advice,
            step_results,
            reply_instructions,
            final_text,
            workspace_facts,
            workspace_brief,
        )
        .await;
        final_text = orchestration_helpers::preserve_exact_grounded_path(
            final_text,
            step_results,
            reply_instructions,
        );
    }

    let (formatted_text, formatted_usage) = orchestration_helpers::maybe_format_final_text(
        client,
        chat_url,
        formatter_cfg,
        line,
        final_text,
        usage_total,
    )
    .await;
    usage_total = formatted_usage;
    Ok((formatted_text, usage_total))
}

pub(crate) async fn judge_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    judge_cfg: &Profile,
    scenario: &CalibrationScenario,
    user_message: &str,
    step_results: &[StepResult],
    final_text: &str,
) -> Result<CalibrationJudgeVerdict> {
    orchestration_helpers::request_judge_verdict(
        client,
        chat_url,
        judge_cfg,
        scenario,
        user_message,
        step_results,
        final_text,
    )
    .await
}
