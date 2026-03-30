use crate::*;

#[derive(Debug, Clone)]
pub(crate) struct SessionPaths {
    pub(crate) root: PathBuf,
    pub(crate) shell_dir: PathBuf,
    pub(crate) artifacts_dir: PathBuf,
    pub(crate) snapshots_dir: PathBuf,
    pub(crate) plans_dir: PathBuf,
    pub(crate) decisions_dir: PathBuf,
    pub(crate) tune_dir: PathBuf,
}

pub(crate) fn new_session_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;
    Ok(format!("s_{:010}_{}", now.as_secs(), now.subsec_nanos()))
}

pub(crate) fn ensure_session_layout(sessions_root: &PathBuf) -> Result<SessionPaths> {
    std::fs::create_dir_all(sessions_root)
        .with_context(|| format!("mkdir {}", sessions_root.display()))?;

    let sid = new_session_id()?;
    let root = sessions_root.join(&sid);
    let shell_dir = root.join("shell");
    let artifacts_dir = root.join("artifacts");
    let snapshots_dir = root.join("snapshots");
    let plans_dir = root.join("plans");
    let decisions_dir = root.join("decisions");
    let tune_dir = root.join("tune");

    std::fs::create_dir_all(&shell_dir)
        .with_context(|| format!("mkdir {}", shell_dir.display()))?;
    std::fs::create_dir_all(&artifacts_dir)
        .with_context(|| format!("mkdir {}", artifacts_dir.display()))?;
    std::fs::create_dir_all(&snapshots_dir)
        .with_context(|| format!("mkdir {}", snapshots_dir.display()))?;
    std::fs::create_dir_all(&plans_dir)
        .with_context(|| format!("mkdir {}", plans_dir.display()))?;
    std::fs::create_dir_all(&decisions_dir)
        .with_context(|| format!("mkdir {}", decisions_dir.display()))?;
    std::fs::create_dir_all(&tune_dir).with_context(|| format!("mkdir {}", tune_dir.display()))?;

    let master = plans_dir.join("_master.md");
    if !master.exists() {
        std::fs::write(
            &master,
            "# Master Plan\n\n- [ ] (Add high-level plan items here)\n",
        )
        .with_context(|| format!("write {}", master.display()))?;
    }

    Ok(SessionPaths {
        root,
        shell_dir,
        artifacts_dir,
        snapshots_dir,
        plans_dir,
        decisions_dir,
        tune_dir,
    })
}

pub(crate) fn next_shell_seq(shell_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(shell_dir).with_context(|| format!("read_dir {}", shell_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        let digits = name
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>();
        if digits.len() >= 3 {
            if let Ok(n) = digits[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn write_shell_action(shell_dir: &PathBuf, cmd_line: &str) -> Result<PathBuf> {
    let n = next_shell_seq(shell_dir)?;
    let path = shell_dir.join(format!("{n:03}.sh"));
    std::fs::write(&path, format!("{cmd_line}\n"))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn write_shell_output(shell_dir: &PathBuf, seq_path: &PathBuf, output: &str) -> Result<PathBuf> {
    let stem = seq_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "000".to_string());
    let path = shell_dir.join(format!("{stem}.out"));
    std::fs::write(&path, output).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn next_artifact_seq(artifacts_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in std::fs::read_dir(artifacts_dir)
        .with_context(|| format!("read_dir {}", artifacts_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        let digits = name
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if digits.len() >= 3 {
            if let Ok(n) = digits[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn reserve_artifact_path(
    artifacts_dir: &PathBuf,
    kind: &str,
    extension: &str,
) -> Result<(String, PathBuf)> {
    let n = next_artifact_seq(artifacts_dir)?;
    let artifact_id = format!("a_{n:03}");
    let safe_kind = kind
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let ext = extension.trim_start_matches('.');
    let path = artifacts_dir.join(format!("{n:03}_{safe_kind}.{ext}"));
    Ok((artifact_id, path))
}

pub(crate) fn append_artifact_manifest_record(
    artifacts_dir: &PathBuf,
    record: &ArtifactRecord,
) -> Result<PathBuf> {
    let path = artifacts_dir.join("manifest.jsonl");
    let line = serde_json::to_string(record).context("serialize artifact record")?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?
        .write_all(format!("{line}\n").as_bytes())
        .with_context(|| format!("append {}", path.display()))?;
    Ok(path)
}

pub(crate) fn next_plan_seq(plans_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(plans_dir).with_context(|| format!("read_dir {}", plans_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("plan_") {
            if rest.len() >= 7 && rest.as_bytes()[3] == b'.' {
                let digits = &rest[..3];
                if let Ok(n) = digits.parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn write_plan_file(plans_dir: &PathBuf, content: &str) -> Result<PathBuf> {
    let n = next_plan_seq(plans_dir)?;
    let path = plans_dir.join(format!("plan_{n:03}.md"));
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn append_master_link(plans_dir: &PathBuf, plan_path: &PathBuf, title: &str) -> Result<()> {
    let master = plans_dir.join("_master.md");
    let rel = plan_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "plan_???".to_string());
    let line = format!("- [ ] {title} ({rel})\n");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&master)
        .with_context(|| format!("open {}", master.display()))?
        .write_all(line.as_bytes())
        .with_context(|| format!("append {}", master.display()))?;
    Ok(())
}

pub(crate) fn next_decision_seq(decisions_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in std::fs::read_dir(decisions_dir)
        .with_context(|| format!("read_dir {}", decisions_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if name.len() >= 7 && name.ends_with(".txt") {
            if let Ok(n) = name[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn write_decision(decisions_dir: &PathBuf, word: &str) -> Result<PathBuf> {
    let n = next_decision_seq(decisions_dir)?;
    let path = decisions_dir.join(format!("{n:03}.txt"));
    std::fs::write(&path, format!("{}\n", word.trim()))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn next_gate_why_seq(tune_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(tune_dir).with_context(|| format!("read_dir {}", tune_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("gate_why_") {
            if rest.len() >= 7 && rest.ends_with(".txt") {
                if let Ok(n) = rest[..3].parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn write_gate_why(tune_dir: &PathBuf, text: &str) -> Result<PathBuf> {
    let n = next_gate_why_seq(tune_dir)?;
    let path = tune_dir.join(format!("gate_why_{n:03}.txt"));
    std::fs::write(&path, text.trim().to_string() + "\n")
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn write_thinking_log(
    shell_dir: &PathBuf,
    seq: u32,
    thinking_content: &str,
) -> Result<PathBuf> {
    let path = shell_dir.join(format!("{seq:03}.thinking.log"));
    std::fs::write(&path, thinking_content)
        .with_context(|| format!("write thinking log {}", path.display()))?;
    Ok(path)
}

pub(crate) fn append_thinking_to_manifest(
    artifacts_dir: &PathBuf,
    step_id: &str,
    thinking_path: &PathBuf,
    bytes_written: u64,
) -> Result<PathBuf> {
    let manifest_path = artifacts_dir.join("thinking_manifest.jsonl");
    let record = serde_json::json!({
        "step_id": step_id,
        "thinking_path": thinking_path.display().to_string(),
        "bytes": bytes_written,
        "created_unix_s": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });
    let line = serde_json::to_string(&record).context("serialize thinking record")?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&manifest_path)
        .with_context(|| format!("open thinking manifest {}", manifest_path.display()))?
        .write_all(format!("{line}\n").as_bytes())
        .with_context(|| format!("append thinking manifest {}", manifest_path.display()))?;
    Ok(manifest_path)
}

/// Save goal state to session file (Task 014)
pub(crate) fn save_goal_state(
    session_root: &PathBuf,
    goal_state: &GoalState,
) -> Result<PathBuf> {
    let path = session_root.join("goal_state.json");
    let json = serde_json::to_string_pretty(goal_state)
        .context("serialize goal state")?;
    std::fs::write(&path, json)
        .with_context(|| format!("write goal state {}", path.display()))?;
    Ok(path)
}

/// Load goal state from session file (Task 014)
pub(crate) fn load_goal_state(
    session_root: &PathBuf,
) -> Option<GoalState> {
    let path = session_root.join("goal_state.json");
    if !path.exists() {
        return None;
    }
    let json = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

// ============================================================================
// Task 023: Hierarchy Persistence
// ============================================================================

/// Helper to save JSON to file
fn save_json<T: Serialize + ?Sized>(path: &PathBuf, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)
        .with_context(|| format!("serialize {}", path.display()))?;
    std::fs::write(path, json)
        .with_context(|| format!("write {}", path.display()))?;
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
///
/// Persists:
/// - hierarchy/goal.json - Level 1 masterplan
/// - hierarchy/subgoals.json - Level 2 milestones
/// - hierarchy/tasks.json - Level 3 work units
/// - hierarchy/methods.json - Level 4 decomposition strategies
pub(crate) fn save_hierarchy(
    session_root: &PathBuf,
    goal: &Goal,
    subgoals: &[Subgoal],
    tasks: &[Task],
    methods: &[Method],
) -> Result<PathBuf> {
    let hierarchy_dir = ensure_hierarchy_dir(session_root)?;
    
    // Save each level separately for easier debugging and partial recovery
    save_json(&hierarchy_dir.join("goal.json"), goal)?;
    save_json(&hierarchy_dir.join("subgoals.json"), subgoals)?;
    save_json(&hierarchy_dir.join("tasks.json"), tasks)?;
    save_json(&hierarchy_dir.join("methods.json"), methods)?;
    
    // Create manifest with metadata
    let manifest = serde_json::json!({
        "created_unix_s": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "goal_id": goal.id,
        "subgoals_count": subgoals.len(),
        "tasks_count": tasks.len(),
        "methods_count": methods.len(),
        "depth": 4, // goal → subgoal → task → method
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
pub(crate) fn load_hierarchy_progress(
    session_root: &PathBuf,
) -> Option<HierarchyProgress> {
    let path = session_root.join("hierarchy").join("progress.json");
    load_json(&path)
}

/// Load goal from session (for multi-turn persistence)
pub(crate) fn load_hierarchy_goal(
    session_root: &PathBuf,
) -> Option<Goal> {
    let path = session_root.join("hierarchy").join("goal.json");
    load_json(&path)
}

/// Load subgoals from session
pub(crate) fn load_hierarchy_subgoals(
    session_root: &PathBuf,
) -> Option<Vec<Subgoal>> {
    let path = session_root.join("hierarchy").join("subgoals.json");
    load_json(&path)
}

/// Load tasks from session
pub(crate) fn load_hierarchy_tasks(
    session_root: &PathBuf,
) -> Option<Vec<Task>> {
    let path = session_root.join("hierarchy").join("tasks.json");
    load_json(&path)
}

/// Load methods from session
pub(crate) fn load_hierarchy_methods(
    session_root: &PathBuf,
) -> Option<Vec<Method>> {
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
