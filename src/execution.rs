use crate::*;

pub(crate) struct ExecutionState {
    pub(crate) step_results: Vec<StepResult>,
    pub(crate) final_reply: Option<String>,
    pub(crate) artifacts: HashMap<String, String>,
}

pub(crate) async fn execute_program(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    program: &Program,
    planner_cfg: &Profile,
    planner_master_cfg: &Profile,
    decider_cfg: &Profile,
    summarizer_cfg: &Profile,
    command_repair_cfg: Option<&Profile>,
    evidence_compactor_cfg: Option<&Profile>,
    artifact_classifier_cfg: Option<&Profile>,
    scope: &ScopePlan,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    objective: &str,
    emit_shell_output: bool,
    readonly_only: bool,
) -> Result<(Vec<StepResult>, Option<String>)> {
    let mut state = ExecutionState {
        step_results: Vec::new(),
        final_reply: None,
        artifacts: HashMap::new(),
    };

    for step in program.steps.clone() {
        execution_steps::handle_program_step(
            args,
            client,
            chat_url,
            session,
            workdir,
            planner_cfg,
            planner_master_cfg,
            decider_cfg,
            summarizer_cfg,
            command_repair_cfg,
            evidence_compactor_cfg,
            artifact_classifier_cfg,
            scope,
            complexity,
            formula,
            objective,
            emit_shell_output,
            readonly_only,
            step,
            &mut state,
        )
        .await?;
    }

    Ok((state.step_results, state.final_reply))
}
