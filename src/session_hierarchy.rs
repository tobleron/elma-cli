//! @efficiency-role: infra-adapter
//!
//! Session - Hierarchy Persistence (Task 023)

use crate::*;

/// Helper to save JSON to file
fn save_json<T: Serialize + ?Sized>(path: &PathBuf, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)
        .with_context(|| format!("serialize {}", path.display()))?;
    std::fs::write(path, json).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Helper to load JSON from file
fn load_json<T: DeserializeOwned>(path: &PathBuf) -> Option<T> {
    if !path.exists() {
        return None;
    }
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

/// Ensure hierarchy directory structure exists
pub(crate) fn ensure_hierarchy_dir(session_root: &PathBuf) -> Result<PathBuf> {
    let hierarchy_dir = session_root.join("hierarchy");
    std::fs::create_dir_all(&hierarchy_dir)
        .with_context(|| format!("mkdir {}", hierarchy_dir.display()))?;
    Ok(hierarchy_dir)
}

/// Save complete hierarchy to session files
pub(crate) fn save_hierarchy(
    session_root: &PathBuf,
    goal: &Goal,
    subgoals: &[Subgoal],
    tasks: &[Task],
    methods: &[Method],
) -> Result<PathBuf> {
    let hierarchy_dir = ensure_hierarchy_dir(session_root)?;

    save_json(&hierarchy_dir.join("goal.json"), goal)?;
    save_json(&hierarchy_dir.join("subgoals.json"), subgoals)?;
    save_json(&hierarchy_dir.join("tasks.json"), tasks)?;
    save_json(&hierarchy_dir.join("methods.json"), methods)?;

    let manifest = serde_json::json!({
        "created_unix_s": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "goal_id": goal.id,
        "subgoals_count": subgoals.len(),
        "tasks_count": tasks.len(),
        "methods_count": methods.len(),
        "depth": 4,
    });
    save_json(&hierarchy_dir.join("manifest.json"), &manifest)?;

    Ok(hierarchy_dir)
}

/// Save hierarchy progress for crash recovery
pub(crate) fn save_hierarchy_progress(
    session_root: &PathBuf,
    progress: &HierarchyProgress,
) -> Result<()> {
    let hierarchy_dir = ensure_hierarchy_dir(session_root)?;
    save_json(&hierarchy_dir.join("progress.json"), progress)
}

/// Load hierarchy progress for resumption
pub(crate) fn load_hierarchy_progress(session_root: &PathBuf) -> Option<HierarchyProgress> {
    let path = session_root.join("hierarchy").join("progress.json");
    load_json(&path)
}

/// Load goal from session
pub(crate) fn load_hierarchy_goal(session_root: &PathBuf) -> Option<Goal> {
    let path = session_root.join("hierarchy").join("goal.json");
    load_json(&path)
}

/// Load subgoals from session
pub(crate) fn load_hierarchy_subgoals(session_root: &PathBuf) -> Option<Vec<Subgoal>> {
    let path = session_root.join("hierarchy").join("subgoals.json");
    load_json(&path)
}

/// Load tasks from session
pub(crate) fn load_hierarchy_tasks(session_root: &PathBuf) -> Option<Vec<Task>> {
    let path = session_root.join("hierarchy").join("tasks.json");
    load_json(&path)
}

/// Load methods from session
pub(crate) fn load_hierarchy_methods(session_root: &PathBuf) -> Option<Vec<Method>> {
    let path = session_root.join("hierarchy").join("methods.json");
    load_json(&path)
}

/// Check if hierarchy exists for this session
pub(crate) fn has_hierarchy(session_root: &PathBuf) -> bool {
    session_root.join("hierarchy").join("goal.json").exists()
}

/// Load complete hierarchy (all levels)
pub(crate) fn load_full_hierarchy(
    session_root: &PathBuf,
) -> Option<(Goal, Vec<Subgoal>, Vec<Task>, Vec<Method>)> {
    let goal = load_hierarchy_goal(session_root)?;
    let subgoals = load_hierarchy_subgoals(session_root)?;
    let tasks = load_hierarchy_tasks(session_root)?;
    let methods = load_hierarchy_methods(session_root)?;
    Some((goal, subgoals, tasks, methods))
}
