//! @efficiency-role: scenario-spec
//!
//! Scenario Runtime Evaluation.

use crate::tune::{ScenarioRuntimeOutcome, TuneResources};
use crate::tune_scenario_helpers::*;
use crate::*;

pub(crate) async fn evaluate_runtime_scenario(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    resources: &TuneResources,
    scenario: CalibrationScenario,
) -> Result<ScenarioRuntimeOutcome> {
    let scenario_path = calibration_scenario_path(&resources.repo, &scenario);
    let txt = std::fs::read_to_string(&scenario_path)
        .with_context(|| format!("read {}", scenario_path.display()))?;
    let (user_message, recent_messages) = parse_scenario_dialog(&txt);
    let mut conversation_messages = vec![ChatMessage::simple("system", &String::new())];
    conversation_messages.extend(recent_messages.clone());

    let decision = infer_route_prior(
        client,
        chat_url,
        &resources.speech_act_cfg,
        &resources.router_cfg,
        &resources.mode_router_cfg,
        &resources.cal,
        &user_message,
        &resources.ws,
        &resources.ws_brief,
        &conversation_messages,
    )
    .await?;

    let speech_ok = decision
        .speech_act
        .choice
        .eq_ignore_ascii_case(&scenario.speech_act);
    let workflow_ok = decision
        .workflow
        .choice
        .eq_ignore_ascii_case(&scenario.workflow);
    let mode_ok = scenario
        .mode
        .as_ref()
        .map(|mode| decision.mode.choice.eq_ignore_ascii_case(mode));
    let route_ok = decision.route.eq_ignore_ascii_case(&scenario.route);

    let memories = load_recent_formula_memories(
        &resources
            .tune_sessions_root
            .parent()
            .unwrap_or(&resources.tune_sessions_root)
            .to_path_buf(),
        8,
    )
    .unwrap_or_default();
    let (workflow_plan, complexity, scope, formula, _) = derive_planning_prior(
        client,
        chat_url,
        &resources.workflow_planner_cfg,
        &resources.complexity_cfg,
        &resources.scope_builder_cfg,
        &resources.formula_cfg,
        &user_message,
        &decision,
        &resources.ws,
        &resources.ws_brief,
        &memories,
        &conversation_messages,
    )
    .await;

    let (scope_eval_ok, scope_eval_reason) = evaluate_scope(&scope, &scenario);

    let (program_opt, program_eval): (Option<Program>, ProgramEvaluation) =
        orchestrate_and_evaluate_program(
            client,
            chat_url,
            &resources.orchestrator_cfg,
            &user_message,
            &decision,
            workflow_plan.as_ref(),
            &complexity,
            &scope,
            &formula,
            &resources.ws,
            &resources.ws_brief,
            &conversation_messages,
            &scenario,
            args,
        )
        .await?;

    let consistency_ok = check_program_consistency(
        client,
        chat_url,
        &resources.orchestrator_cfg,
        &user_message,
        &decision,
        workflow_plan.as_ref(),
        &complexity,
        &scope,
        &formula,
        &resources.ws,
        &resources.ws_brief,
        &conversation_messages,
        &program_opt,
    )
    .await;

    let mut actual_step_count = program_opt
        .as_ref()
        .map(|p| p.steps.len())
        .unwrap_or_default();
    let mut tool_economy = tool_economy_score(
        actual_step_count,
        scenario.minimum_step_count,
        scenario.maximum_step_count,
    );

    let (
        executed_in_tune,
        execution_ok,
        critic_ok,
        critic_reason,
        response_ok,
        response_reason,
        response_plain_text,
        compaction_ok,
        compaction_reason,
        classification_ok,
        classification_reason,
        claim_check_ok,
        claim_check_reason,
        presentation_ok,
        presentation_reason,
    ) = if let Some(program) = program_opt.clone() {
        if program_eval.parsed
            && program_eval.shape_ok
            && program_eval.policy_ok
            && (scenario.route.eq_ignore_ascii_case("CHAT") || program_eval.executable_in_tune)
        {
            execute_and_evaluate_program(
                args,
                client,
                chat_url,
                resources,
                &scenario,
                &user_message,
                &decision,
                workflow_plan.as_ref(),
                &complexity,
                &scope,
                &formula,
                &conversation_messages,
                program,
                actual_step_count,
            )
            .await?
        } else {
            (
                false, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            )
        }
    } else {
        (
            false, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None,
        )
    };

    let all_ok = speech_ok
        && workflow_ok
        && mode_ok.unwrap_or(true)
        && route_ok
        && (!scope_eval_ok || scope_eval_ok)
        && program_eval.parsed
        && program_eval.shape_ok
        && program_eval.policy_ok
        && consistency_ok
        && compaction_ok.unwrap_or(true)
        && classification_ok.unwrap_or(true)
        && execution_ok.unwrap_or(true)
        && critic_ok.unwrap_or(true)
        && claim_check_ok.unwrap_or(true)
        && presentation_ok.unwrap_or(true)
        && response_ok.unwrap_or(true);

    Ok(ScenarioRuntimeOutcome {
        speech_pair: (
            scenario.speech_act.clone(),
            decision.speech_act.choice.clone(),
        ),
        workflow_pair: (scenario.workflow.clone(), decision.workflow.choice.clone()),
        mode_pair: scenario
            .mode
            .clone()
            .map(|m| (m, decision.mode.choice.clone())),
        route_pair: (scenario.route.clone(), decision.route.clone()),
        scenario_result: ScenarioCalibrationResult {
            suite: scenario.suite.clone(),
            file: scenario.file.clone(),
            notes: scenario.notes.clone(),
            speech_act_expected: scenario.speech_act.clone(),
            speech_act_predicted: decision.speech_act.choice.clone(),
            speech_act_probability: probability_of(
                &decision.speech_act.distribution,
                &scenario.speech_act,
            ),
            speech_act_ok: speech_ok,
            workflow_expected: scenario.workflow.clone(),
            workflow_predicted: decision.workflow.choice.clone(),
            workflow_probability: probability_of(
                &decision.workflow.distribution,
                &scenario.workflow,
            ),
            workflow_ok,
            mode_expected: scenario.mode.clone(),
            mode_predicted: scenario.mode.as_ref().map(|_| decision.mode.choice.clone()),
            mode_probability: scenario
                .mode
                .as_ref()
                .map(|m| probability_of(&decision.mode.distribution, m)),
            mode_ok,
            route_expected: scenario.route.clone(),
            route_predicted: decision.route.clone(),
            route_probability: probability_of(&decision.distribution, &scenario.route),
            route_ok,
            program_signature: program_eval.signature.clone(),
            program_parse_ok: program_eval.parsed,
            program_parse_error: program_eval.parse_error,
            program_shape_ok: program_eval.shape_ok,
            program_shape_reason: program_eval.shape_reason,
            program_policy_ok: program_eval.policy_ok,
            program_policy_reason: program_eval.policy_reason,
            program_consistency_ok: consistency_ok,
            executed_in_tune,
            execution_ok,
            critic_ok,
            critic_reason,
            response_ok,
            response_reason,
            response_plain_text,
            scope_ok: Some(scope_eval_ok),
            scope_reason: Some(scope_eval_reason),
            compaction_ok,
            compaction_reason,
            classification_ok,
            classification_reason,
            claim_check_ok,
            claim_check_reason,
            presentation_ok,
            presentation_reason,
            tool_economy_score: Some(tool_economy),
            all_ok,
        },
        efficiency_result: EfficiencyScenarioResult {
            suite: scenario.suite,
            file: scenario.file,
            task_success: all_ok,
            grounding_ok: response_ok,
            scope_ok: Some(scope_eval_ok),
            compaction_ok,
            classification_ok,
            claim_check_ok,
            presentation_ok,
            tool_economy_score: tool_economy,
            actual_steps: actual_step_count,
            expected_min_steps: scenario.minimum_step_count,
            expected_max_steps: scenario.maximum_step_count,
        },
    })
}
