//! @efficiency-role: infra-adapter
//!
//! Runtime task persistence for session-scoped main tasks.

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TaskMirrorPolicy {
    SessionOnly,
    SessionAndProject,
}

impl TaskMirrorPolicy {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            TaskMirrorPolicy::SessionOnly => "session_only",
            TaskMirrorPolicy::SessionAndProject => "session_and_project",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RuntimeTaskRecord {
    pub(crate) version: u32,
    pub(crate) task_id: String,
    pub(crate) created_unix_s: u64,
    pub(crate) updated_unix_s: u64,
    pub(crate) objective: String,
    pub(crate) request_class: RequestClass,
    pub(crate) formula_id: SkillFormulaId,
    pub(crate) formula_stages: Vec<FormulaStage>,
    pub(crate) current_stage_index: usize,
    pub(crate) mirror_policy: TaskMirrorPolicy,
    pub(crate) gate_reason: String,
    pub(crate) selected_skill_label: String,
    pub(crate) stage_notes: Vec<String>,
    pub(crate) stop_reason: Option<String>,
    pub(crate) completed: bool,
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn runtime_tasks_dir(session_root: &PathBuf) -> PathBuf {
    session_root.join("runtime_tasks")
}

pub(crate) fn ensure_runtime_tasks_dir(session_root: &PathBuf) -> Result<PathBuf> {
    let dir = runtime_tasks_dir(session_root);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    Ok(dir)
}

pub(crate) fn build_runtime_task_record(
    line: &str,
    plan: &ExecutionPlanSelection,
    mirror_policy: TaskMirrorPolicy,
) -> RuntimeTaskRecord {
    let now = now_unix_s();
    RuntimeTaskRecord {
        version: 1,
        task_id: format!("rt_{now}"),
        created_unix_s: now,
        updated_unix_s: now,
        objective: line.trim().to_string(),
        request_class: plan.request_class,
        formula_id: plan.formula.id,
        formula_stages: plan.formula.stages.clone(),
        current_stage_index: 0,
        mirror_policy,
        gate_reason: plan.gate.reason.clone(),
        selected_skill_label: plan.short_label(),
        stage_notes: Vec::new(),
        stop_reason: None,
        completed: false,
    }
}

pub(crate) fn save_runtime_task_record(
    session_root: &PathBuf,
    record: &RuntimeTaskRecord,
) -> Result<PathBuf> {
    // Primary: store current + bounded history in session.json.runtime_task
    use crate::session_write::mutate_session_doc;
    let _ = mutate_session_doc(session_root, |doc| {
        let current = serde_json::to_value(record).expect("serialize runtime task");
        doc["runtime_task"] = serde_json::json!({
            "current": current,
        });
    });

    // Legacy: also write to runtime_tasks/ dir
    let dir = ensure_runtime_tasks_dir(session_root)?;
    let latest = dir.join("latest.json");
    let history = dir.join(format!("{}.json", record.task_id));
    let json = serde_json::to_string_pretty(record).context("serialize runtime task")?;
    std::fs::write(&latest, &json).with_context(|| format!("write {}", latest.display()))?;
    std::fs::write(&history, json).with_context(|| format!("write {}", history.display()))?;
    Ok(history)
}

pub(crate) fn load_latest_runtime_task(session_root: &PathBuf) -> Option<RuntimeTaskRecord> {
    // Try session.json first (new path)
    use crate::session_write::load_session_doc;
    let doc = load_session_doc(session_root);
    if let Some(current) = doc.get("runtime_task").and_then(|r| r.get("current")) {
        if let Ok(record) = serde_json::from_value::<RuntimeTaskRecord>(current.clone()) {
            return Some(record);
        }
    }
    // Legacy fallback
    let path = runtime_tasks_dir(session_root).join("latest.json");
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

pub(crate) fn advance_runtime_task_stage(
    session_root: &PathBuf,
    record: &mut RuntimeTaskRecord,
    note: impl Into<String>,
) -> Result<()> {
    record.updated_unix_s = now_unix_s();
    record.stage_notes.push(note.into());
    if record.current_stage_index + 1 < record.formula_stages.len() {
        record.current_stage_index += 1;
    }
    save_runtime_task_record(session_root, record)?;
    Ok(())
}

pub(crate) fn finalize_runtime_task(
    session_root: &PathBuf,
    record: &mut RuntimeTaskRecord,
    stop_reason: Option<String>,
) -> Result<()> {
    record.updated_unix_s = now_unix_s();
    record.stop_reason = stop_reason;
    record.completed = true;
    save_runtime_task_record(session_root, record)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_task_record_roundtrip_works() {
        let temp = std::env::temp_dir().join(format!("elma_task_test_{}", now_unix_s()));
        std::fs::create_dir_all(&temp).unwrap();
        let plan = ExecutionPlanSelection::simple_general();
        let record =
            build_runtime_task_record("test objective", &plan, TaskMirrorPolicy::SessionOnly);
        save_runtime_task_record(&temp, &record).unwrap();
        let loaded = load_latest_runtime_task(&temp).unwrap();
        assert_eq!(loaded.objective, "test objective");
        let _ = std::fs::remove_dir_all(temp);
    }
}
