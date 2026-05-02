//! @efficiency-role: data-model

use crate::*;

pub(crate) struct LoadedProfiles {
    pub(crate) elma_cfg: Profile,
    pub(crate) intent_helper_cfg: Profile,
    pub(crate) expert_advisor_cfg: Profile,
    pub(crate) the_maestro_cfg: Profile,
    pub(crate) status_message_cfg: Profile,
    pub(crate) planner_master_cfg: Profile,
    pub(crate) planner_cfg: Profile,
    pub(crate) decider_cfg: Profile,
    pub(crate) selector_cfg: Profile,
    pub(crate) summarizer_cfg: Profile,
    pub(crate) turn_summary_cfg: Profile,
    pub(crate) formatter_cfg: Profile,
    pub(crate) json_outputter_cfg: Profile,
    pub(crate) final_answer_extractor_cfg: Profile,
    pub(crate) complexity_cfg: Profile,
    pub(crate) evidence_need_cfg: Profile,
    pub(crate) action_need_cfg: Profile,
    pub(crate) formula_cfg: Profile,
    pub(crate) workflow_planner_cfg: Profile,
    pub(crate) evidence_mode_cfg: Profile,
    pub(crate) command_repair_cfg: Profile,
    pub(crate) task_semantics_guard_cfg: Profile,
    pub(crate) execution_sufficiency_cfg: Profile,
    pub(crate) outcome_verifier_cfg: Profile,
    pub(crate) memory_gate_cfg: Profile,
    pub(crate) command_preflight_cfg: Profile,
    pub(crate) scope_builder_cfg: Profile,
    pub(crate) evidence_compactor_cfg: Profile,
    pub(crate) artifact_classifier_cfg: Profile,
    pub(crate) result_presenter_cfg: Profile,
    pub(crate) result_presenter_concise_cfg: Profile,
    pub(crate) result_presenter_long_cfg: Profile,
    pub(crate) claim_checker_cfg: Profile,
    pub(crate) orchestrator_cfg: Profile,
    pub(crate) critic_cfg: Profile,
    pub(crate) logical_reviewer_cfg: Profile,
    pub(crate) efficiency_reviewer_cfg: Profile,
    pub(crate) risk_reviewer_cfg: Profile,
    pub(crate) refinement_cfg: Profile,
    pub(crate) reflection_cfg: Profile,
    pub(crate) meta_review_cfg: Profile,
    pub(crate) router_cfg: Profile,
    pub(crate) mode_router_cfg: Profile,
    pub(crate) speech_act_cfg: Profile,
    pub(crate) router_cal: RouterCalibration,
}

pub(crate) struct AppRuntime {
    pub(crate) args: Args,
    pub(crate) client: reqwest::Client,
    pub(crate) chat_url: Url,
    pub(crate) model_id: String,
    pub(crate) model_cfg_dir: PathBuf,
    pub(crate) ctx_max: Option<u64>,
    pub(crate) session: SessionPaths,
    pub(crate) repo: PathBuf,
    pub(crate) ws: String,
    pub(crate) ws_brief: String,
    pub(crate) guidance: GuidanceSnapshot,
    pub(crate) system_content: String,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) profiles: LoadedProfiles,
    pub(crate) goal_state: GoalState,
    pub(crate) execution_plan: ExecutionPlanSelection,
    pub(crate) active_runtime_task: Option<RuntimeTaskRecord>,
    pub(crate) last_stop_outcome: Option<StopOutcome>,
    pub(crate) verbose: bool,
    pub(crate) retry_attempt: u32,
    pub(crate) tool_registry: tool_discovery::ToolRegistry,
    pub(crate) execution_profile: ExecutionProfile,
    pub(crate) turn_count: u32,
}

pub(crate) async fn run(args: Args) -> Result<()> {
    let Some(mut runtime) = app_bootstrap::bootstrap_app(args).await? else {
        return Ok(());
    };

    // Auto-cleanup old sessions (older than 30 days)
    let sessions_root = runtime
        .session
        .root
        .parent()
        .unwrap_or(&runtime.session.root)
        .to_path_buf();
    if let Ok((count, size)) = crate::session_gc::auto_cleanup_sessions(&sessions_root, 30) {
        if count > 0 {
            tracing::info!("Auto-cleaned {} sessions, reclaimed {} bytes", count, size);
        }
    }

    let result = app_chat::run_chat_loop(&mut runtime).await;
    // Finalize session status (Task 282: index update)
    let session_root = runtime.session.root.clone();
    match &result {
        Ok(()) => {
            let _ = crate::write_session_status(
                &session_root,
                "completed",
                runtime.turn_count,
                None,
                None,
            );
        }
        Err(e) => {
            let _ = crate::write_session_status(
                &session_root,
                "error",
                runtime.turn_count,
                None,
                Some(&e.to_string()),
            );
        }
    }
    result
}
