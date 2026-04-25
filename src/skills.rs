//! @efficiency-role: service-orchestrator
//!
//! Built-in skill registry, predictive main-task gate, and bounded formula selector.

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SkillId {
    General,
    TaskSteward,
    RepoExplorer,
    DocumentReader,
    FileScout,
}

impl SkillId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SkillId::General => "general",
            SkillId::TaskSteward => "task_steward",
            SkillId::RepoExplorer => "repo_explorer",
            SkillId::DocumentReader => "document_reader",
            SkillId::FileScout => "file_scout",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RequestClass {
    Simple,
    MainTask,
}

impl RequestClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            RequestClass::Simple => "simple",
            RequestClass::MainTask => "main_task",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SkillFormulaId {
    GeneralReply,
    RepoExploreThenReply,
    DocumentReadThenReply,
    FileScoutThenReply,
    FileScoutDocumentReply,
    ProjectTaskSteward,
}

impl SkillFormulaId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SkillFormulaId::GeneralReply => "general_reply",
            SkillFormulaId::RepoExploreThenReply => "repo_explore_then_reply",
            SkillFormulaId::DocumentReadThenReply => "document_read_then_reply",
            SkillFormulaId::FileScoutThenReply => "file_scout_then_reply",
            SkillFormulaId::FileScoutDocumentReply => "file_scout_document_reply",
            SkillFormulaId::ProjectTaskSteward => "project_task_steward",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SkillManifest {
    pub(crate) id: SkillId,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) directive: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FormulaStage {
    pub(crate) skill_id: SkillId,
    pub(crate) action: String,
    pub(crate) success_check: String,
    pub(crate) stop_budget: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillFormula {
    pub(crate) id: SkillFormulaId,
    pub(crate) stages: Vec<FormulaStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MainTaskGateVerdict {
    pub(crate) class: RequestClass,
    pub(crate) predicted_tool_calls: u32,
    pub(crate) expected_resume_value: bool,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExecutionPlanSelection {
    pub(crate) request_class: RequestClass,
    pub(crate) formula: SkillFormula,
    pub(crate) reason: String,
    pub(crate) summary_directive: String,
    pub(crate) gate: MainTaskGateVerdict,
}

impl ExecutionPlanSelection {
    pub(crate) fn simple_general() -> Self {
        let formula = builtin_formula(SkillFormulaId::GeneralReply);
        let summary_directive = formula_stage_directives(&formula).join(" ");
        Self {
            request_class: RequestClass::Simple,
            formula,
            reason: "fallback to simple general handling".to_string(),
            summary_directive,
            gate: MainTaskGateVerdict {
                class: RequestClass::Simple,
                predicted_tool_calls: 0,
                expected_resume_value: false,
                reason: "fallback".to_string(),
            },
        }
    }

    pub(crate) fn primary_skill(&self) -> SkillId {
        self.formula
            .stages
            .first()
            .map(|stage| stage.skill_id)
            .unwrap_or(SkillId::General)
    }

    pub(crate) fn short_label(&self) -> String {
        format!(
            "{}:{}",
            self.request_class.as_str(),
            self.formula.id.as_str()
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ExecutionPlanVerdict {
    request_class: String,
    formula: String,
    reason: String,
    predicted_tool_calls: Option<u32>,
    expected_resume_value: Option<bool>,
}

pub(crate) fn builtin_skill_manifests() -> Vec<SkillManifest> {
    vec![
        SkillManifest {
            id: SkillId::General,
            name: "general",
            description: "Default bounded Elma behavior for requests that do not need a specialized operating mode.",
            directive: "General mode: solve the request with minimum sufficient evidence and without assuming a specialized operating mode.",
        },
        SkillManifest {
            id: SkillId::TaskSteward,
            name: "task_steward",
            description: "Manage AGENTS.md-driven project task ledgers and `_tasks` protocol when the user is doing project planning work.",
            directive: "Task steward mode: read AGENTS.md and _tasks/TASKS.md before project task mutations, keep numbering grounded in actual inventory, and prefer updating the relevant active master plan over creating duplicate planning.",
        },
        SkillManifest {
            id: SkillId::RepoExplorer,
            name: "repo_explorer",
            description: "Explore repository structure, entry points, and implementation patterns from grounded evidence.",
            directive: "Repo explorer mode: map structure first, inspect representative files, and report grounded findings with file references instead of broad claims.",
        },
        SkillManifest {
            id: SkillId::DocumentReader,
            name: "document_reader",
            description: "Read supported documents, normalize extracted content, summarize it, or locate a specific passage.",
            directive: "Document reader mode: focus on extraction, chunking, summary, and needle-in-haystack lookup. Prefer local offline backends and stay read-only.",
        },
        SkillManifest {
            id: SkillId::FileScout,
            name: "file_scout",
            description: "Search the machine in read-only mode, disclose searched roots, and hand candidate files to the next stage.",
            directive: "File scout mode: use bounded on-demand discovery, stay read-only outside the workspace, and tell the user which roots and files were inspected.",
        },
    ]
}

pub(crate) fn builtin_formula(formula_id: SkillFormulaId) -> SkillFormula {
    match formula_id {
        SkillFormulaId::GeneralReply => SkillFormula {
            id: formula_id,
            stages: vec![FormulaStage {
                skill_id: SkillId::General,
                action: "solve directly with minimum sufficient evidence".to_string(),
                success_check: "the request is answered without task persistence".to_string(),
                stop_budget: 1,
            }],
        },
        SkillFormulaId::RepoExploreThenReply => SkillFormula {
            id: formula_id,
            stages: vec![
                FormulaStage {
                    skill_id: SkillId::RepoExplorer,
                    action: "inspect the repository and gather grounded evidence".to_string(),
                    success_check: "key files and structure are identified".to_string(),
                    stop_budget: 3,
                },
                FormulaStage {
                    skill_id: SkillId::General,
                    action: "synthesize the findings into a direct user answer".to_string(),
                    success_check: "the answer cites the inspected evidence".to_string(),
                    stop_budget: 1,
                },
            ],
        },
        SkillFormulaId::DocumentReadThenReply => SkillFormula {
            id: formula_id,
            stages: vec![
                FormulaStage {
                    skill_id: SkillId::DocumentReader,
                    action: "extract and normalize the requested document content".to_string(),
                    success_check: "searchable document chunks are available".to_string(),
                    stop_budget: 3,
                },
                FormulaStage {
                    skill_id: SkillId::General,
                    action: "answer using grounded document evidence".to_string(),
                    success_check: "summary or match results are returned".to_string(),
                    stop_budget: 1,
                },
            ],
        },
        SkillFormulaId::FileScoutThenReply => SkillFormula {
            id: formula_id,
            stages: vec![
                FormulaStage {
                    skill_id: SkillId::FileScout,
                    action: "discover candidate files in the requested search scope".to_string(),
                    success_check: "searched roots and candidate files are known".to_string(),
                    stop_budget: 3,
                },
                FormulaStage {
                    skill_id: SkillId::General,
                    action: "summarize findings from the inspected file set".to_string(),
                    success_check: "the answer identifies inspected files".to_string(),
                    stop_budget: 1,
                },
            ],
        },
        SkillFormulaId::FileScoutDocumentReply => SkillFormula {
            id: formula_id,
            stages: vec![
                FormulaStage {
                    skill_id: SkillId::FileScout,
                    action: "discover and rank candidate files in the requested search scope"
                        .to_string(),
                    success_check: "target documents are identified".to_string(),
                    stop_budget: 3,
                },
                FormulaStage {
                    skill_id: SkillId::DocumentReader,
                    action: "extract and normalize content from the selected documents".to_string(),
                    success_check: "document chunks and metadata are available".to_string(),
                    stop_budget: 3,
                },
                FormulaStage {
                    skill_id: SkillId::General,
                    action: "answer using the extracted document evidence".to_string(),
                    success_check: "the answer names inspected files and cites matches".to_string(),
                    stop_budget: 1,
                },
            ],
        },
        SkillFormulaId::ProjectTaskSteward => SkillFormula {
            id: formula_id,
            stages: vec![
                FormulaStage {
                    skill_id: SkillId::TaskSteward,
                    action:
                        "update the project task inventory and master plan according to AGENTS.md"
                            .to_string(),
                    success_check: "project task state matches the requested planning change"
                        .to_string(),
                    stop_budget: 3,
                },
                FormulaStage {
                    skill_id: SkillId::General,
                    action: "report the resulting task state to the user".to_string(),
                    success_check: "the user sees the task outcome clearly".to_string(),
                    stop_budget: 1,
                },
            ],
        },
    }
}

fn formula_from_str(value: &str) -> SkillFormulaId {
    match value.trim() {
        "repo_explore_then_reply" => SkillFormulaId::RepoExploreThenReply,
        "document_read_then_reply" => SkillFormulaId::DocumentReadThenReply,
        "file_scout_then_reply" => SkillFormulaId::FileScoutThenReply,
        "file_scout_document_reply" => SkillFormulaId::FileScoutDocumentReply,
        "project_task_steward" => SkillFormulaId::ProjectTaskSteward,
        _ => SkillFormulaId::GeneralReply,
    }
}

fn request_class_from_str(value: &str) -> RequestClass {
    match value.trim() {
        "main_task" => RequestClass::MainTask,
        _ => RequestClass::Simple,
    }
}

pub(crate) fn formula_stage_directives(formula: &SkillFormula) -> Vec<&'static str> {
    formula
        .stages
        .iter()
        .filter_map(|stage| {
            builtin_skill_manifests()
                .into_iter()
                .find(|skill| skill.id == stage.skill_id)
                .map(|skill| skill.directive)
        })
        .collect()
}

fn render_formula_catalog() -> Vec<String> {
    [
        SkillFormulaId::GeneralReply,
        SkillFormulaId::RepoExploreThenReply,
        SkillFormulaId::DocumentReadThenReply,
        SkillFormulaId::FileScoutThenReply,
        SkillFormulaId::FileScoutDocumentReply,
        SkillFormulaId::ProjectTaskSteward,
    ]
    .into_iter()
    .map(|id| {
        let formula = builtin_formula(id);
        let skills = formula
            .stages
            .iter()
            .map(|stage| stage.skill_id.as_str())
            .collect::<Vec<_>>()
            .join(" -> ");
        format!("- {}: {}", id.as_str(), skills)
    })
    .collect()
}

pub(crate) async fn select_execution_plan_for_request(
    client: &reqwest::Client,
    chat_url: &Url,
    selector_cfg: &Profile,
    line: &str,
    guidance: &GuidanceSnapshot,
) -> Result<ExecutionPlanSelection> {
    let skill_catalog = builtin_skill_manifests()
        .iter()
        .map(|skill| format!("- {}: {}", skill.name, skill.description))
        .collect::<Vec<_>>()
        .join("\n");
    let formula_catalog = render_formula_catalog().join("\n");

    let prompt = format!(
        "User request:\n{line}\n\nAvailable skills:\n{skill_catalog}\n\nAvailable formulas:\n{formula_catalog}\n\nProject guidance snapshot:\n{}\n\nClassify the request as `simple` or `main_task`. Use `main_task` for multi-step, multi-skill, resumable, or evidence-heavy work. Return JSON: {{\"request_class\":\"simple|main_task\",\"formula\":\"general_reply|repo_explore_then_reply|document_read_then_reply|file_scout_then_reply|file_scout_document_reply|project_task_steward\",\"reason\":\"one short sentence\",\"predicted_tool_calls\":0,\"expected_resume_value\":false}}",
        guidance.render_for_system_prompt()
    );

    let req = ChatCompletionRequest {
        model: selector_cfg.model.clone(),
        messages: vec![
            ChatMessage::simple(
                "system",
                "Choose a bounded Elma execution plan. Prefer simple handling unless the request clearly needs persistent task tracking or multiple stages. Output JSON only.",
            ),
            ChatMessage::simple("user", &prompt),
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: 160,
        n_probs: None,
        repeat_penalty: Some(selector_cfg.repeat_penalty),
        reasoning_format: Some("none".to_string()),
        grammar: None,
        tools: None,
    };

    let verdict: ExecutionPlanVerdict =
        crate::ui_chat::chat_json_with_repair_timeout(client, chat_url, &req, 30).await?;
    let request_class = request_class_from_str(&verdict.request_class);
    let formula = builtin_formula(formula_from_str(&verdict.formula));
    let summary_directive = formula_stage_directives(&formula).join(" ");
    Ok(ExecutionPlanSelection {
        request_class,
        reason: verdict.reason.clone(),
        summary_directive,
        gate: MainTaskGateVerdict {
            class: request_class,
            predicted_tool_calls: verdict.predicted_tool_calls.unwrap_or_default(),
            expected_resume_value: verdict.expected_resume_value.unwrap_or(false),
            reason: verdict.reason,
        },
        formula,
    })
}

pub(crate) fn render_skill_catalog(current: &ExecutionPlanSelection) -> String {
    let mut lines = vec![format!("Current execution plan: {}", current.short_label())];
    lines.push(format!(
        "Request class: {} | Reason: {}",
        current.request_class.as_str(),
        current.reason
    ));
    lines.push(String::new());
    lines.push("Active formula stages:".to_string());
    for (i, stage) in current.formula.stages.iter().enumerate() {
        lines.push(format!(
            "  {}. {} — {} (budget: {})",
            i + 1,
            stage.skill_id.as_str(),
            stage.action,
            stage.stop_budget
        ));
    }
    lines.push(String::new());
    lines.push("Skills:".to_string());
    for skill in builtin_skill_manifests() {
        lines.push(format!("{} - {}", skill.name, skill.description));
    }
    lines.push(String::new());
    lines.push("Formulas:".to_string());
    lines.extend(render_formula_catalog());
    lines.push(String::new());
    lines.push("Notes:".to_string());
    lines.push("- Main tasks are persisted in the session ledger.".to_string());
    lines.push(
        "- Project-task mirroring is reserved for project planning work or explicit user requests."
            .to_string(),
    );
    lines.push("- Simple requests do not create runtime task state.".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_skill_is_available() {
        let manifests = builtin_skill_manifests();
        assert!(manifests.iter().any(|skill| skill.id == SkillId::General));
    }

    #[test]
    fn formulas_include_multi_stage_document_flow() {
        let formula = builtin_formula(SkillFormulaId::FileScoutDocumentReply);
        assert_eq!(formula.stages.len(), 3);
        assert_eq!(formula.stages[0].skill_id, SkillId::FileScout);
        assert_eq!(formula.stages[1].skill_id, SkillId::DocumentReader);
    }

    #[test]
    fn render_skill_catalog_mentions_formula_and_mode() {
        let rendered = render_skill_catalog(&ExecutionPlanSelection::simple_general());
        assert!(rendered.contains("general_reply"));
        assert!(rendered.contains("session ledger"));
    }
}
