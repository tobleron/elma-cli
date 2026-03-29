use crate::*;

pub(crate) fn program_safety_check(cmd: &str) -> bool {
    is_command_sane(cmd) && is_command_allowed(cmd)
}

pub(crate) fn step_kind(s: &Step) -> &'static str {
    match s {
        Step::Shell { .. } => "shell",
        Step::Select { .. } => "select",
        Step::Plan { .. } => "plan",
        Step::MasterPlan { .. } => "masterplan",
        Step::Decide { .. } => "decide",
        Step::Summarize { .. } => "summarize",
        Step::Edit { .. } => "edit",
        Step::Reply { .. } => "reply",
    }
}

pub(crate) fn step_id(s: &Step) -> &str {
    match s {
        Step::Shell { id, .. } => id,
        Step::Select { id, .. } => id,
        Step::Plan { id, .. } => id,
        Step::MasterPlan { id, .. } => id,
        Step::Decide { id, .. } => id,
        Step::Summarize { id, .. } => id,
        Step::Edit { id, .. } => id,
        Step::Reply { id, .. } => id,
    }
}

pub(crate) fn step_common(s: &Step) -> &StepCommon {
    match s {
        Step::Shell { common, .. } => common,
        Step::Select { common, .. } => common,
        Step::Plan { common, .. } => common,
        Step::MasterPlan { common, .. } => common,
        Step::Decide { common, .. } => common,
        Step::Summarize { common, .. } => common,
        Step::Edit { common, .. } => common,
        Step::Reply { common, .. } => common,
    }
}

pub(crate) fn step_purpose(s: &Step) -> String {
    let common = step_common(s);
    if !common.purpose.trim().is_empty() {
        return common.purpose.trim().to_string();
    }
    match s {
        Step::Shell { .. } => "shell".to_string(),
        Step::Select { .. } => "select".to_string(),
        Step::Plan { .. } => "plan".to_string(),
        Step::MasterPlan { .. } => "masterplan".to_string(),
        Step::Decide { .. } => "decide".to_string(),
        Step::Summarize { .. } => "summarize".to_string(),
        Step::Edit { .. } => "edit".to_string(),
        Step::Reply { .. } => "answer".to_string(),
    }
}

pub(crate) fn step_success_condition(s: &Step) -> String {
    step_common(s).success_condition.trim().to_string()
}

pub(crate) fn step_depends_on(s: &Step) -> Vec<String> {
    step_common(s).depends_on.clone()
}

pub(crate) fn is_command_allowed(cmd: &str) -> bool {
    // For now: workspace-only, no network/remote, no destructive operations.
    // This is intentionally strict to keep "no internet" and avoid dangerous commands.
    let lower = cmd.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')' | '<' | '>'))
        .filter(|s| !s.is_empty())
        .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
        .collect();

    let banned_cmds = [
        "curl", "wget", "ssh", "scp", "rsync", "nc", "netcat", "ping", "sudo", "shutdown", "reboot",
    ];

    if tokens.iter().any(|t| banned_cmds.contains(&t.as_str())) {
        return false;
    }

    for pair in tokens.windows(2) {
        if pair[0] == "rm" && (pair[1] == "-rf" || pair[1] == "-fr") {
            return false;
        }
    }

    true
}

pub(crate) fn command_is_readonly(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    if lower.contains(" >")
        || lower.contains(">>")
        || lower.contains(">|")
        || lower.contains("tee ")
        || lower.contains("sed -i")
        || lower.contains("perl -pi")
    {
        return false;
    }

    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')' | '<' | '>'))
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return false;
    }

    let first = tokens[0].rsplit('/').next().unwrap_or(tokens[0]);
    match first {
        "ls" | "pwd" | "cat" | "head" | "tail" | "rg" | "grep" | "find" | "awk" | "cut"
        | "sort" | "uniq" | "wc" | "basename" | "dirname" | "stat" | "tree" | "fd" | "jq"
        | "uname" | "whoami" | "tty" => return true,
        "sed" => return !tokens.iter().any(|t| *t == "-i"),
        "git" => {
            let sub = tokens.get(1).copied().unwrap_or("");
            return matches!(
                sub,
                "status" | "diff" | "log" | "show" | "branch" | "rev-parse"
            );
        }
        _ => {}
    }

    false
}

pub(crate) fn program_signature(program: &Program) -> String {
    program
        .steps
        .iter()
        .map(|step| match step {
            Step::Shell { cmd, .. } => format!("shell:{}", normalize_shell_cmd(cmd)),
            Step::Select { instructions, .. } => {
                format!("select:{}", instructions.trim())
            }
            Step::Plan { .. } => "plan".to_string(),
            Step::MasterPlan { .. } => "masterplan".to_string(),
            Step::Decide { .. } => "decide".to_string(),
            Step::Summarize { .. } => "summarize".to_string(),
            Step::Edit { spec, .. } => format!(
                "edit:{}:{}",
                spec.operation.trim(),
                spec.path.trim()
            ),
            Step::Reply { .. } => "reply".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

pub(crate) fn evaluate_program_for_scenario(
    program: &Program,
    scenario: &CalibrationScenario,
) -> ProgramEvaluation {
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut has_reply = false;
    let mut has_shell = false;
    let mut has_plan = false;
    let mut has_masterplan = false;
    let mut has_decide = false;
    let mut has_edit = false;
    let mut shape_errors = Vec::new();
    let mut policy_errors = Vec::new();
    let mut executable_in_tune = true;

    for step in &program.steps {
        let sid = step_id(step).to_string();
        *ids.entry(sid.clone()).or_insert(0usize) += 1;
        if step_purpose(step).trim().is_empty() {
            shape_errors.push(format!("step {sid} missing purpose"));
        }
        if step_success_condition(step).trim().is_empty() {
            shape_errors.push(format!("step {sid} missing success_condition"));
        }

        match step {
            Step::Shell { cmd, .. } => {
                has_shell = true;
                let normalized = normalize_shell_cmd(cmd);
                if !program_safety_check(&normalized) {
                    policy_errors.push(format!("shell step {sid} blocked by policy"));
                }
                if scenario.mode.as_deref() == Some("INSPECT") && !command_is_readonly(&normalized)
                {
                    policy_errors.push(format!("inspect shell step {sid} is not read-only"));
                }
                if !command_is_readonly(&normalized) {
                    executable_in_tune = false;
                }
            }
            Step::Select { .. } => {}
            Step::Plan { .. } => has_plan = true,
            Step::MasterPlan { .. } => has_masterplan = true,
            Step::Decide { .. } => has_decide = true,
            Step::Summarize { .. } => {}
            Step::Edit { spec, .. } => {
                has_edit = true;
                executable_in_tune = false;
                if spec.path.trim().is_empty() {
                    shape_errors.push(format!("step {sid} missing edit path"));
                }
                if spec.operation.trim().is_empty() {
                    shape_errors.push(format!("step {sid} missing edit operation"));
                }
            }
            Step::Reply { .. } => has_reply = true,
        }
    }

    for (id, count) in ids {
        if count > 1 {
            shape_errors.push(format!("duplicate step id {id}"));
        }
    }

    if program.steps.is_empty() {
        shape_errors.push("program has no steps".to_string());
    }
    if !has_reply {
        shape_errors.push("program has no reply step".to_string());
    }

    match scenario.route.as_str() {
        "CHAT" => {
            if has_shell || has_plan || has_masterplan || has_decide || has_edit {
                shape_errors.push("chat route should not execute workflow steps".to_string());
            }
        }
        "SHELL" => {
            if !(has_shell || has_edit) {
                shape_errors.push("shell route missing shell or edit step".to_string());
            }
        }
        "PLAN" => {
            if !has_plan {
                shape_errors.push("plan route missing plan step".to_string());
            }
        }
        "MASTERPLAN" => {
            if !has_masterplan {
                shape_errors.push("masterplan route missing masterplan step".to_string());
            }
        }
        "DECIDE" => {
            if !has_decide {
                shape_errors.push("decide route missing decide step".to_string());
            }
        }
        _ => {}
    }

    if scenario.speech_act == "CAPABILITY_CHECK"
        && (has_shell || has_plan || has_masterplan || has_decide || has_edit)
    {
        shape_errors.push("capability check should not execute or plan".to_string());
    }

    ProgramEvaluation {
        parsed: true,
        parse_error: String::new(),
        shape_ok: shape_errors.is_empty(),
        shape_reason: if shape_errors.is_empty() {
            "program structure matches scenario expectations".to_string()
        } else {
            shape_errors.join("; ")
        },
        policy_ok: policy_errors.is_empty(),
        policy_reason: if policy_errors.is_empty() {
            "program policy is acceptable".to_string()
        } else {
            policy_errors.join("; ")
        },
        executable_in_tune: executable_in_tune && policy_errors.is_empty(),
        signature: program_signature(program),
    }
}

pub(crate) fn capability_guard_threshold(route_decision: &RouteDecision) -> bool {
    route_decision
        .speech_act
        .choice
        .eq_ignore_ascii_case("CAPABILITY_CHECK")
        && probability_of(&route_decision.speech_act.distribution, "CAPABILITY_CHECK") >= 0.65
}

pub(crate) fn apply_capability_guard(
    program: &mut Program,
    route_decision: &RouteDecision,
    guards_enabled: bool,
) -> bool {
    // Hard constraints are disabled - bypass guard but keep code for reference
    if !guards_enabled {
        return false;
    }
    
    if !capability_guard_threshold(route_decision) {
        return false;
    }
    let has_non_reply = program
        .steps
        .iter()
        .any(|s| !matches!(s, Step::Reply { .. }));
    if !has_non_reply {
        return false;
    }

    let existing_reply = program.steps.iter().find_map(|s| match s {
        Step::Reply { instructions, .. } => Some(instructions.clone()),
        _ => None,
    });
    let instructions = existing_reply.unwrap_or_else(|| {
        "Answer the user's capability question in plain text. Do not execute commands. If helpful, say what Elma can do in this workspace and that you can do it if the user asks.".to_string()
    });
    program.steps = vec![Step::Reply {
        id: "r_cap".to_string(),
        instructions,
        common: StepCommon {
            purpose: "answer capability question without executing".to_string(),
            depends_on: Vec::new(),
            success_condition:
                "the user receives a plain-text capability answer with no command execution"
                    .to_string(),
        },
    }];
    true
}

pub(crate) fn is_command_sane(cmd: &str) -> bool {
    // Very small sanity checks to avoid common model glitches.
    let t = cmd.trim();
    if t.is_empty() {
        return false;
    }
    if t == "ls -" || t.ends_with(" ls -") || t.contains(" ls - ") {
        return false;
    }
    true
}

pub(crate) fn should_classify_artifacts(
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> bool {
    formula.primary.eq_ignore_ascii_case("inspect_decide_reply")
        || complexity
            .suggested_pattern
            .eq_ignore_ascii_case("inspect_decide_reply")
}

pub(crate) fn edit_operation_is_supported(op: &str) -> bool {
    matches!(op.trim(), "write_file" | "replace_text" | "append_text")
}

pub(crate) fn resolve_workspace_edit_path(workdir: &Path, raw_path: &str) -> Result<PathBuf> {
    use std::path::Component;

    let raw_path = raw_path.trim();
    if raw_path.is_empty() {
        anyhow::bail!("edit path is empty");
    }
    let relative = Path::new(raw_path);
    if relative.is_absolute() {
        anyhow::bail!("absolute edit paths are not allowed");
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        anyhow::bail!("edit path must stay inside the workspace");
    }
    Ok(workdir.join(relative))
}

pub(crate) fn preview_text(text: &str, max_lines: usize) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn resolve_command_placeholders(
    cmd: &str,
    artifacts: &HashMap<String, String>,
) -> Result<String> {
    let mut out = String::new();
    let mut rest = cmd;

    while let Some(start) = rest.find("{{") {
        let (prefix, after_start) = rest.split_at(start);
        out.push_str(prefix);
        let after_start = &after_start[2..];
        let Some(end) = after_start.find("}}") else {
            anyhow::bail!("unclosed command placeholder");
        };
        let expr = after_start[..end].trim();
        let remainder = &after_start[end + 2..];
        let (id, mode) = expr
            .split_once('|')
            .map(|(id, mode)| (id.trim(), mode.trim()))
            .unwrap_or((expr, "raw"));
        let raw = artifacts
            .get(id)
            .with_context(|| format!("missing workflow artifact for placeholder {id}"))?;
        let value = match mode {
            "" | "raw" => raw.clone(),
            "shell_words" => raw
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|line| line.trim_start_matches("- ").trim())
                .map(shell_quote)
                .collect::<Vec<_>>()
                .join(" "),
            _ => anyhow::bail!("unsupported placeholder mode {mode}"),
        };
        out.push_str(&value);
        rest = remainder;
    }
    out.push_str(rest);
    Ok(out)
}

pub(crate) fn command_placeholder_refs(cmd: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = cmd;
    while let Some(start) = rest.find("{{") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let expr = after_start[..end].trim();
        let id = expr
            .split_once('|')
            .map(|(id, _)| id.trim())
            .unwrap_or(expr);
        if !id.is_empty() && !refs.iter().any(|seen| seen == id) {
            refs.push(id.to_string());
        }
        rest = &after_start[end + 2..];
    }
    refs
}

pub(crate) fn program_step_json(step: &Step) -> serde_json::Value {
    let base = serde_json::json!({
        "id": step_id(step),
        "type": step_kind(step),
        "purpose": step_purpose(step),
        "depends_on": step_depends_on(step),
        "success_condition": step_success_condition(step),
    });
    let mut obj = base.as_object().cloned().unwrap_or_default();
    match step {
        Step::Shell { cmd, .. } => {
            obj.insert("cmd".to_string(), serde_json::json!(cmd));
            obj.insert(
                "placeholder_refs".to_string(),
                serde_json::json!(command_placeholder_refs(cmd)),
            );
        }
        Step::Select { instructions, .. } => {
            obj.insert(
                "instructions".to_string(),
                serde_json::json!(instructions.trim()),
            );
        }
        Step::Plan { goal, .. } | Step::MasterPlan { goal, .. } => {
            obj.insert("goal".to_string(), serde_json::json!(goal.trim()));
        }
        Step::Decide { prompt, .. } => {
            obj.insert("prompt".to_string(), serde_json::json!(prompt.trim()));
        }
        Step::Summarize {
            text,
            instructions,
            ..
        } => {
            obj.insert(
                "instructions".to_string(),
                serde_json::json!(instructions.trim()),
            );
            if !text.trim().is_empty() {
                obj.insert(
                    "text_preview".to_string(),
                    serde_json::json!(preview_text(text, 6)),
                );
            }
        }
        Step::Edit { spec, .. } => {
            obj.insert("path".to_string(), serde_json::json!(spec.path.trim()));
            obj.insert(
                "operation".to_string(),
                serde_json::json!(spec.operation.trim()),
            );
            if !spec.find.trim().is_empty() {
                obj.insert(
                    "find_preview".to_string(),
                    serde_json::json!(preview_text(&spec.find, 3)),
                );
            }
            if !spec.replace.trim().is_empty() {
                obj.insert(
                    "replace_preview".to_string(),
                    serde_json::json!(preview_text(&spec.replace, 3)),
                );
            }
            if !spec.content.trim().is_empty() {
                obj.insert(
                    "content_preview".to_string(),
                    serde_json::json!(preview_text(&spec.content, 6)),
                );
            }
        }
        Step::Reply { instructions, .. } => {
            obj.insert(
                "instructions".to_string(),
                serde_json::json!(instructions.trim()),
            );
        }
    }
    serde_json::Value::Object(obj)
}

pub(crate) fn step_result_json(result: &StepResult) -> serde_json::Value {
    serde_json::json!({
        "id": result.id,
        "type": result.kind,
        "purpose": result.purpose,
        "depends_on": result.depends_on,
        "success_condition": result.success_condition,
        "ok": result.ok,
        "summary": result.summary,
        "command": result.command,
        "raw_output": result.raw_output,
        "exit_code": result.exit_code,
        "output_bytes": result.output_bytes,
        "truncated": result.truncated,
        "timed_out": result.timed_out,
        "artifact_path": result.artifact_path,
        "artifact_kind": result.artifact_kind,
        "outcome_status": result.outcome_status,
        "outcome_reason": result.outcome_reason,
    })
}

pub(crate) fn run_shell_one_liner(
    cmd: &str,
    workdir: &PathBuf,
    artifact_target: Option<(&PathBuf, &str)>,
) -> Result<ShellExecutionResult> {
    const MAX_INLINE_CAPTURE_BYTES: u64 = 128 * 1024;
    const MAX_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024;
    const MAX_WALL_SECS: u64 = 20;

    let (target_path, capture_limit, artifact_path, artifact_kind) = if let Some((path, kind)) =
        artifact_target
    {
        (
            path.clone(),
            MAX_ARTIFACT_BYTES,
            Some(path.clone()),
            Some(kind.to_string()),
        )
    } else {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let out_path = std::env::temp_dir().join(format!(
            "elma_shell_{}_{}_{}.out",
            std::process::id(),
            stamp,
            hash_short(cmd)
        ));
        (out_path, MAX_INLINE_CAPTURE_BYTES, None, None)
    };

    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&target_path)
        .with_context(|| format!("open {}", target_path.display()))?;
    let file_err = file
        .try_clone()
        .with_context(|| format!("clone {}", target_path.display()))?;

    let blocks = capture_limit.div_ceil(512);
    let shell_script = format!("ulimit -f {blocks}; {cmd}");
    let mut child = std::process::Command::new("sh")
        .arg("-lc")
        .arg(&shell_script)
        .current_dir(workdir)
        .stdout(std::process::Stdio::from(file))
        .stderr(std::process::Stdio::from(file_err))
        .spawn()
        .with_context(|| format!("Failed to run shell: {cmd}"))?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(MAX_WALL_SECS);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().context("poll shell child")? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child.wait().context("wait killed shell child")?;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };

    let bytes = std::fs::read(&target_path).unwrap_or_default();
    let bytes_written = bytes.len() as u64;
    let preview_limit = if artifact_path.is_some() {
        MAX_INLINE_CAPTURE_BYTES.min(4 * 1024)
    } else {
        MAX_INLINE_CAPTURE_BYTES
    };
    let mut inline_text =
        String::from_utf8_lossy(&bytes[..bytes.len().min(preview_limit as usize)]).to_string();
    let truncated = bytes_written >= capture_limit.saturating_sub(256);
    if timed_out {
        if !inline_text.is_empty() && !inline_text.ends_with('\n') {
            inline_text.push('\n');
        }
        inline_text.push_str("[output stopped by Elma after time limit]\n");
    } else if truncated {
        if !inline_text.is_empty() && !inline_text.ends_with('\n') {
            inline_text.push('\n');
        }
        if artifact_path.is_some() {
            inline_text.push_str("[artifact output truncated by Elma safety cap]\n");
        } else {
            inline_text.push_str("[output truncated by Elma safety cap]\n");
        }
    }

    if artifact_path.is_none() {
        let _ = std::fs::remove_file(&target_path);
    }

    Ok(ShellExecutionResult {
        exit_code: if timed_out { 124 } else { status.code().unwrap_or(1) },
        inline_text,
        bytes_written,
        truncated,
        timed_out,
        artifact_path,
        artifact_kind,
    })
}

fn hash_short(text: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}
