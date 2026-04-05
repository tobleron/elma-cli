//! @efficiency-role: service-orchestrator
//!
//! Core Orchestration Module
//!
//! Provides core orchestration function for program generation,
//! recovery, criticism, and final answer generation.

use crate::app::AppRuntime;
use crate::app_chat_fast_paths::build_direct_reply_program;
use crate::formulas::{select_optimal_formula, FormulaPattern, FormulaScores};
use crate::intel_units::{MaestroOutput, MaestroUnit};
use crate::tools::ToolRegistry;
use crate::*;

// ============================================================================
// New: Single-instruction orchestration (Maestro → Orchestrator pipeline)
// ============================================================================

/// Transform a single Maestro instruction into 1-3 structured JSON steps.
/// Accumulates steps with proper depends_on wiring.
pub(crate) async fn orchestrate_instruction_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    instruction: &str,
    user_message: &str,
    intent: &str,
    expert_advice: &str,
    ws: &str,
    ws_brief: &str,
    previous_steps: &[Step],
    step_counter: &mut u32,
) -> Result<Vec<Step>> {
    let workspace_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let _tool_registry = ToolRegistry::new(&workspace_path);

    // Build capabilities list in plain English
    let capabilities = "Available capabilities:\n\
        - shell: Execute shell commands (run commands, list files, check system state)\n\
        - read: Read file contents (inspect specific files)\n\
        - search: Search with ripgrep (find patterns, locate definitions)\n\
        - edit: Edit files (modify content, fix bugs)\n\
        - explore: Explore codebases (map unfamiliar modules, form and test hypotheses)\n\
        - write: Create new files (write new content)\n\
        - delete: Remove files (delete content)\n\
        - select: Select items from list (choose from options)\n\
        - decide: Make decisions (evaluate options, choose best path)\n\
        - plan: Create plans (break complex work into steps)\n\
        - summarize: Summarize findings (organize and present conclusions)\n\
        - reply: Respond to users (answer from knowledge or evidence)";

    let previous_steps_text = if previous_steps.is_empty() {
        "No previous steps.".to_string()
    } else {
        format!(
            "Previous steps (use their IDs for depends_on if needed):\n{}",
            previous_steps
                .iter()
                .map(|s| format!("- {} ({}) — {}", step_id(s), step_kind(s), step_purpose(s)))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let next_id = *step_counter + 1;

    let prompt = format!(
        r#"USER REQUEST: {user_message}

INTENT: {intent}

EXPERT ADVICE: {expert_advice}

CURRENT INSTRUCTION (transform this into 1-3 structured steps):
{instruction}

WORKSPACE FACTS:
{ws}

WORKSPACE BRIEF:
{ws_brief}

{capabilities}

{previous_steps_text}

TASK: Transform the current instruction into 1-3 structured JSON steps.
- Use step IDs starting from s{next_id}
- Wire depends_on to reference previous step IDs if this instruction depends on prior work
- Each step must have a clear purpose and success condition
- Use the simplest step types that achieve the goal

Output ONLY valid JSON object with a "steps" array, like:
{{"steps":[
  {{"id":"s1","type":"shell","cmd":"ls -1 src/","purpose":"list source files","depends_on":[],"success_condition":"file list returned"}},
  {{"id":"s2","type":"reply","instructions":"Summarize findings","purpose":"answer user","depends_on":["s1"],"success_condition":"user receives summary"}}
]}}"#,
        user_message = user_message.trim(),
        intent = intent.trim(),
        expert_advice = expert_advice.trim(),
        instruction = instruction.trim(),
        ws = ws.trim(),
        ws_brief = ws_brief.trim(),
        next_id = next_id,
    );

    let orch_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are Elma's step composer. Transform English instructions into 1-3 structured JSON steps for Elma's execution pipeline.

Step types available: shell, read, search, edit, explore, write, delete, select, decide, plan, masterplan, summarize, reply, respond.

Output ONLY valid JSON with a steps array:
{\"steps\":[{\"id\":\"s1\",\"type\":\"shell\",\"cmd\":\"ls -1\",\"purpose\":\"list files\",\"depends_on\":[],\"success_condition\":\"files listed\"}]}

Each step needs: id, type, purpose, depends_on (array of step IDs), success_condition.
Shell steps need: cmd. Read steps need: path. Search steps need: query and paths. Edit steps need: path, operation, find, replace. Reply/Respond steps need: instructions."
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            },
        ],
        temperature: orchestrator_cfg.temperature,
        top_p: orchestrator_cfg.top_p,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens.min(2048),
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
        grammar: None,
    };

    // Call LLM directly with our custom system prompt (not the profile's)
    let program: Program = crate::ui_chat::chat_json_with_repair_timeout(
        client,
        chat_url,
        &orch_req,
        orchestrator_cfg.timeout_s.min(45),
    )
    .await?;

    // Update step counter
    *step_counter += program.steps.len() as u32;

    Ok(program.steps)
}

/// Build a program from Maestro instructions.
/// Calls Maestro, then loops through each instruction transforming it into steps.
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

    let output = match unit.execute_with_fallback(&context).await {
        Ok(o) => o,
        Err(e) => {
            // Maestro failed — return error so caller can use fallback
            return Err(anyhow::anyhow!("Maestro execution failed: {}", e));
        }
    };
    let maestro_output: MaestroOutput = serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse Maestro output: {}", e))?;

    if maestro_output.steps.is_empty() {
        return Err(anyhow::anyhow!("Maestro produced empty steps"));
    }

    // Step 2: Loop through instructions, transform each into steps
    let mut all_steps: Vec<Step> = Vec::new();
    let mut step_counter: u32 = 0;

    let intent = line;
    let expert_advice = "";

    for maestro_instruction in &maestro_output.steps {
        let steps = orchestrate_instruction_once(
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
        .await?;

        all_steps.extend(steps);
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
            },
        });
    }

    Ok(Program {
        objective: line.to_string(),
        steps: all_steps,
    })
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
) -> Result<(String, Option<u64>)> {
    let runtime_context = serde_json::json!({
        "model_id": model_id,
        "base_url": base_url,
    });
    if route_decision.route.eq_ignore_ascii_case("CHAT") && step_results.is_empty() {
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

        return Ok(orchestration_helpers::maybe_format_final_text(
            client,
            chat_url,
            formatter_cfg,
            line,
            final_text,
            None,
        )
        .await);
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

    let (mut final_text, mut usage_total) = if route_decision.route.eq_ignore_ascii_case("CHAT") {
        orchestration_helpers::request_chat_final_text(
            client,
            chat_url,
            elma_cfg,
            system_content,
            line,
            step_results,
            reply_instructions,
        )
        .await?
    } else {
        (
            orchestration_helpers::present_result_via_unit(
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
            .unwrap_or_default(),
            None,
        )
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
