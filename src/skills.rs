//! @efficiency-role: service-orchestrator
//!
//! Built-in skill registry, predictive main-task gate, and bounded formula selector.

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        "User request:\n{line}\n\nAvailable skills:\n{skill_catalog}\n\nAvailable formulas:\n{formula_catalog}\n\nProject guidance snapshot:\n{}\n\nClassify the request as `simple` or `main_task`. Use `main_task` for multi-step, multi-skill, resumable, or evidence-heavy work. Return a single DSL line using the PLAN format described in the system instructions.",
        guidance.render_for_system_prompt()
    );

    let req = chat_request_system_user(
        selector_cfg,
        "Choose a bounded Elma execution plan.\n\nReturn exactly one DSL line and nothing else:\nPLAN request_class=simple formula=general_reply reason=\"one short sentence\" predicted_tool_calls=0 expected_resume_value=false\n\nRules:\n- request_class: simple | main_task\n- formula: general_reply | repo_explore_then_reply | document_read_then_reply | file_scout_then_reply | file_scout_document_reply | project_task_steward\n- No JSON, Markdown fences, or prose outside the DSL line.",
        &prompt,
        ChatRequestOptions::deterministic(160),
    );

    let dsl_value = crate::ui_chat::chat_dsl_with_repair_for_profile_timeout(
        client,
        chat_url,
        &req,
        &selector_cfg.name,
        30,
    )
    .await?;
    let verdict: ExecutionPlanVerdict = serde_json::from_value(dsl_value)?;
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
    fn all_skills_have_unique_ids() {
        let manifests = builtin_skill_manifests();
        assert_eq!(manifests.len(), 5);
        let mut seen = std::collections::HashSet::new();
        for skill in &manifests {
            assert!(seen.insert(skill.id), "duplicate skill id: {:?}", skill.id);
        }
    }

    #[test]
    fn formulas_include_multi_stage_document_flow() {
        let formula = builtin_formula(SkillFormulaId::FileScoutDocumentReply);
        assert_eq!(formula.stages.len(), 3);
        assert_eq!(formula.stages[0].skill_id, SkillId::FileScout);
        assert_eq!(formula.stages[1].skill_id, SkillId::DocumentReader);
    }

    #[test]
    fn all_formulas_have_correct_stage_counts() {
        let cases = [
            (SkillFormulaId::GeneralReply, 1),
            (SkillFormulaId::RepoExploreThenReply, 2),
            (SkillFormulaId::DocumentReadThenReply, 2),
            (SkillFormulaId::FileScoutThenReply, 2),
            (SkillFormulaId::FileScoutDocumentReply, 3),
            (SkillFormulaId::ProjectTaskSteward, 2),
        ];
        for (id, expected) in &cases {
            let formula = builtin_formula(*id);
            assert_eq!(
                formula.stages.len(),
                *expected,
                "formula {:?} expected {} stages, got {}",
                id,
                expected,
                formula.stages.len()
            );
        }
    }

    #[test]
    fn formula_from_str_maps_correctly() {
        assert_eq!(
            formula_from_str("general_reply"),
            SkillFormulaId::GeneralReply
        );
        assert_eq!(
            formula_from_str("repo_explore_then_reply"),
            SkillFormulaId::RepoExploreThenReply
        );
        assert_eq!(
            formula_from_str("document_read_then_reply"),
            SkillFormulaId::DocumentReadThenReply
        );
        assert_eq!(
            formula_from_str("file_scout_then_reply"),
            SkillFormulaId::FileScoutThenReply
        );
        assert_eq!(
            formula_from_str("file_scout_document_reply"),
            SkillFormulaId::FileScoutDocumentReply
        );
        assert_eq!(
            formula_from_str("project_task_steward"),
            SkillFormulaId::ProjectTaskSteward
        );
    }

    #[test]
    fn formula_from_str_unknown_defaults_to_general() {
        assert_eq!(
            formula_from_str("nonexistent_formula"),
            SkillFormulaId::GeneralReply
        );
        assert_eq!(formula_from_str(""), SkillFormulaId::GeneralReply);
    }

    #[test]
    fn all_multi_stage_formulas_end_with_general() {
        let multi_stage = [
            SkillFormulaId::RepoExploreThenReply,
            SkillFormulaId::DocumentReadThenReply,
            SkillFormulaId::FileScoutThenReply,
            SkillFormulaId::FileScoutDocumentReply,
            SkillFormulaId::ProjectTaskSteward,
        ];
        for id in &multi_stage {
            let formula = builtin_formula(*id);
            let last = formula.stages.last().unwrap();
            assert_eq!(
                last.skill_id,
                SkillId::General,
                "formula {:?} last stage should be General, got {:?}",
                id,
                last.skill_id
            );
        }
    }

    #[test]
    fn general_formula_has_minimum_budget() {
        let formula = builtin_formula(SkillFormulaId::GeneralReply);
        assert_eq!(formula.stages[0].stop_budget, 1);
        assert_eq!(formula.stages.len(), 1);
    }

    #[test]
    fn formula_stage_budgets_are_reasonable() {
        let all_ids = [
            SkillFormulaId::GeneralReply,
            SkillFormulaId::RepoExploreThenReply,
            SkillFormulaId::DocumentReadThenReply,
            SkillFormulaId::FileScoutThenReply,
            SkillFormulaId::FileScoutDocumentReply,
            SkillFormulaId::ProjectTaskSteward,
        ];
        for id in &all_ids {
            let formula = builtin_formula(*id);
            for (i, stage) in formula.stages.iter().enumerate() {
                assert!(
                    (1..=3).contains(&stage.stop_budget),
                    "formula {:?} stage {} has budget {} (expected 1-3)",
                    id,
                    i,
                    stage.stop_budget
                );
            }
        }
    }

    #[test]
    fn formula_stage_directives_match_stage_count() {
        let all_ids = [
            SkillFormulaId::GeneralReply,
            SkillFormulaId::RepoExploreThenReply,
            SkillFormulaId::DocumentReadThenReply,
            SkillFormulaId::FileScoutThenReply,
            SkillFormulaId::FileScoutDocumentReply,
            SkillFormulaId::ProjectTaskSteward,
        ];
        for id in &all_ids {
            let formula = builtin_formula(*id);
            let directives = formula_stage_directives(&formula);
            assert_eq!(
                directives.len(),
                formula.stages.len(),
                "formula {:?}: expected {} directives, got {}",
                id,
                formula.stages.len(),
                directives.len()
            );
        }
    }

    #[test]
    fn render_formula_catalog_includes_all_six() {
        let catalog = render_formula_catalog();
        assert!(catalog.iter().any(|l| l.contains("general_reply")));
        assert!(catalog
            .iter()
            .any(|l| l.contains("repo_explore_then_reply")));
        assert!(catalog
            .iter()
            .any(|l| l.contains("document_read_then_reply")));
        assert!(catalog.iter().any(|l| l.contains("file_scout_then_reply")));
        assert!(catalog
            .iter()
            .any(|l| l.contains("file_scout_document_reply")));
        assert!(catalog.iter().any(|l| l.contains("project_task_steward")));
        assert_eq!(catalog.len(), 6);
    }

    #[test]
    fn render_skill_catalog_mentions_formula_and_mode() {
        let rendered = render_skill_catalog(&ExecutionPlanSelection::simple_general());
        assert!(rendered.contains("general_reply"));
        assert!(rendered.contains("session ledger"));
    }

    #[test]
    fn primary_skill_returns_expected_skill() {
        let cases = [
            (SkillFormulaId::GeneralReply, SkillId::General),
            (SkillFormulaId::RepoExploreThenReply, SkillId::RepoExplorer),
            (
                SkillFormulaId::DocumentReadThenReply,
                SkillId::DocumentReader,
            ),
            (SkillFormulaId::FileScoutThenReply, SkillId::FileScout),
            (SkillFormulaId::FileScoutDocumentReply, SkillId::FileScout),
            (SkillFormulaId::ProjectTaskSteward, SkillId::TaskSteward),
        ];
        for (formula_id, expected_skill) in &cases {
            let plan = ExecutionPlanSelection {
                request_class: RequestClass::MainTask,
                formula: builtin_formula(*formula_id),
                reason: String::new(),
                summary_directive: String::new(),
                gate: MainTaskGateVerdict {
                    class: RequestClass::MainTask,
                    predicted_tool_calls: 0,
                    expected_resume_value: false,
                    reason: String::new(),
                },
            };
            assert_eq!(
                plan.primary_skill(),
                *expected_skill,
                "formula {:?} expected primary skill {:?}",
                formula_id,
                expected_skill
            );
        }
    }

    #[test]
    fn simple_general_uses_general_formula() {
        let plan = ExecutionPlanSelection::simple_general();
        assert_eq!(plan.request_class, RequestClass::Simple);
        assert_eq!(plan.formula.id, SkillFormulaId::GeneralReply);
        assert_eq!(plan.primary_skill(), SkillId::General);
        assert_eq!(plan.gate.class, RequestClass::Simple);
    }

    #[test]
    fn semantic_continuity_simple_does_not_use_multi_stage() {
        let plan = ExecutionPlanSelection::simple_general();
        // A simple question like "what is 2+2" must not trigger multi-tool stages
        assert_eq!(
            plan.formula.stages.len(),
            1,
            "simple formula should have exactly 1 stage"
        );
        assert_eq!(
            plan.formula.stages[0].stop_budget, 1,
            "simple formula should have minimum budget"
        );
    }

    #[test]
    fn request_class_from_str_maps_correctly() {
        assert_eq!(request_class_from_str("simple"), RequestClass::Simple);
        assert_eq!(request_class_from_str("main_task"), RequestClass::MainTask);
        assert_eq!(request_class_from_str("unknown"), RequestClass::Simple);
        assert_eq!(request_class_from_str(""), RequestClass::Simple);
    }

    #[test]
    fn skill_id_as_str_round_trips() {
        let all_ids = [
            SkillId::General,
            SkillId::TaskSteward,
            SkillId::RepoExplorer,
            SkillId::DocumentReader,
            SkillId::FileScout,
        ];
        for id in &all_ids {
            let s = id.as_str();
            let found = builtin_skill_manifests()
                .iter()
                .any(|m| m.id == *id && m.name == s);
            assert!(
                found,
                "skill {:?} has as_str '{}' but manifest name mismatch",
                id, s
            );
        }
    }

    #[test]
    fn short_label_format_is_correct() {
        let plan = ExecutionPlanSelection::simple_general();
        let label = plan.short_label();
        assert_eq!(label, "simple:general_reply");
    }
}
