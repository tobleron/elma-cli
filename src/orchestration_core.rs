//! @efficiency-role: service-orchestrator
//!
//! Core Orchestration Module
//!
//! Tool-calling pipeline: Maestro sets context → model calls tools directly → final answer.

use crate::app::AppRuntime;
use crate::app_chat_fast_paths::build_direct_reply_program;
use crate::formulas::{select_optimal_formula, FormulaPattern, FormulaScores};
use crate::tool_loop::run_tool_loop;
use crate::tools::ToolRegistry;
use crate::*;

// ============================================================================
// Tool-Calling Orchestration (no Maestro — model plans itself)
// ============================================================================

/// Build a system prompt for tool calling without any intermediate planner.
/// The model has full context (workspace, conversation, tools) and plans directly.
fn build_tool_calling_system_prompt(runtime: &AppRuntime, _line: &str) -> String {
    // Include conversation excerpt for continuity
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
        format!("\nRECENT CONVERSATION:\n{}", last_msgs.join("\n"))
    };

    let skill_context = build_skill_context(runtime);

    // Build dynamic tool list from the registry (only always-available tools)
    let tool_list = {
        let registry = crate::tool_registry::get_registry();
        let mut lines = String::new();
        for tool in registry.default_tools() {
            lines.push_str(&format!("- {}: {}\n", tool.function.name, tool.function.description));
        }
        lines
    };

    format!(
        r#"You are Elma — a local-first AI assistant that helps users with their requests.

IMPORTANT IDENTITY: You are Elma, NOT a "maestro", "orchestrator", or "system". You are an AI assistant. If anyone asks who you are, answer clearly: "I am Elma."

WORKSPACE FACTS:
{}

WORKSPACE BRIEF:
{}
{}

EXECUTION MODE:
{}

FORMULA STAGES:
{}

PLAN DIRECTIVE:
{}

PROJECT GUIDANCE SNAPSHOT:
{}

SKILL CONTEXT:
{}

TOOLS AVAILABLE (always loaded):
{}

DYNAMIC TOOL LOADING:
Additional tools (shell, read, search, update_todo_list) are not loaded by default to reduce token usage. Use `tool_search` with capability hints to load them on demand. Example queries: "execute shell command", "read file contents", "search text in files", "manage todo list".

HOW TO INVESTIGATE (MINIMUM SUFFICIENT EVIDENCE):
1. Start with the smallest direct source of truth (specific file over broad directory scans).
2.如果需要读取文件或搜索内容，首先使用 tool_search 加载对应工具。
3. As soon as evidence answers the question, call `respond`.

DOCUMENT READING RULES:
- PDF, EPUB, and HTML files should be read with the `read` tool, NOT with shell pipelines like `pdftotext`, `strings`, or `cat`.
- The `read` tool extracts clean text from documents automatically.
- Do NOT run destructive shell pipelines on PDFs (e.g., `pdftotext ... | grep | awk | sort`). These produce garbage output.

STORAGE & RETENTION QUERY PLAYBOOK:
- When calculating directory size: use `du -sh <dir>` instead of per-file `stat` loops.
- When inspecting large files: use `find <dir> -type f -exec du -h {{}} + | sort -rh | head -n 10` instead of `stat`.
- When filtering by date/time: use `find <dir> -type f -mtime +<days>` or `-mmin +<mins>`.
- If date predicates differ by platform (BSD vs GNU), use simple `-mtime` fallbacks before giving up.
- Avoid reading file contents entirely when you only need size, count, or dates.
- For counts, use `find <dir> -type f | wc -l`.

CRITICAL RULES:
- To find files by name or extension: use `shell` with `find` or `ls`, NOT `search`
- `search` searches INSIDE files for text patterns — it does NOT find files by name
- If you already ran a command that showed what you need, use THAT OUTPUT — do not re-list the same paths
- Avoid repetitive commands that only restate known information
- Always ground your answer in actual tool output, not assumptions
- If a command fails, try a different approach
- Always use `respond` when you have the answer
- For conversational requests, respond directly without using tools
- Follow the selected skill's scope before falling back to general behavior"#,
        runtime.ws.trim(),
        runtime.ws_brief.trim(),
        conversation,
        runtime.execution_plan.request_class.as_str(),
        runtime
            .execution_plan
            .formula
            .stages
            .iter()
            .map(|stage| format!("- {}: {}", stage.skill_id.as_str(), stage.action))
            .collect::<Vec<_>>()
            .join("\n"),
        runtime.execution_plan.summary_directive,
        runtime.guidance.render_for_system_prompt(),
        skill_context,
        tool_list.trim() // trim trailing newline
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
) -> Result<(String, usize, usize, bool)> {
    let system_prompt = build_tool_calling_system_prompt(runtime, line);
    trace(
        &runtime.args,
        "tool_calling: direct model planning (no Maestro)",
    );

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
        2048,
        tui,
        Some(&runtime.profiles.summarizer_cfg),
    )
    .await?;

    runtime.last_stop_outcome = result.stop_outcome.clone();

    // Strip leaked thinking/tool_call blocks before returning to the user
    let clean_answer = crate::text_utils::strip_thinking_blocks(&result.final_answer);

    Ok((
        clean_answer,
        result.iterations,
        result.tool_calls_made,
        result.stopped_by_max,
    ))
}

/// Compute risk deterministically from the tool-calling result metadata.
pub(crate) fn compute_program_risk(_tool_calls_made: usize, _iterations: usize) -> ProgramRisk {
    ProgramRisk::Low
}

// ============================================================================
// Legacy compatibility — keep for non-tool-calling paths
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
            ChatMessage::simple(
                "system",
                r#"You are Elma's step composer. Transform English instructions into 1-3 structured JSON steps for Elma's execution pipeline.

Step types available: shell, read, search, edit, explore, write, delete, select, decide, plan, masterplan, summarize, reply, respond.

Output ONLY valid JSON with a steps array:
{"steps":[{"id":"s1","type":"shell","cmd":"ls -1","purpose":"list files","depends_on":[],"success_condition":"files listed"}]}

Each step needs: id, type, purpose, depends_on (array of step IDs), success_condition.
Shell steps need: cmd. Read steps need: path. Search steps need: query and paths. Edit steps need: path, operation, find, replace. Reply/Respond steps need: instructions."#,
            ),
            ChatMessage::simple("user", &prompt),
        ],
        temperature: orchestrator_cfg.temperature,
        top_p: orchestrator_cfg.top_p,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens.min(2048),
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
        grammar: None,
        tools: None,
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
