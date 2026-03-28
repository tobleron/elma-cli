use crate::*;

pub(crate) fn program_safety_check(cmd: &str) -> bool {
    is_command_sane(cmd) && is_command_allowed(cmd)
}

pub(crate) fn step_kind(s: &Step) -> &'static str {
    match s {
        Step::Shell { .. } => "shell",
        Step::Plan { .. } => "plan",
        Step::MasterPlan { .. } => "masterplan",
        Step::Decide { .. } => "decide",
        Step::Summarize { .. } => "summarize",
        Step::Reply { .. } => "reply",
    }
}

pub(crate) fn step_id(s: &Step) -> &str {
    match s {
        Step::Shell { id, .. } => id,
        Step::Plan { id, .. } => id,
        Step::MasterPlan { id, .. } => id,
        Step::Decide { id, .. } => id,
        Step::Summarize { id, .. } => id,
        Step::Reply { id, .. } => id,
    }
}

pub(crate) fn step_common(s: &Step) -> &StepCommon {
    match s {
        Step::Shell { common, .. } => common,
        Step::Plan { common, .. } => common,
        Step::MasterPlan { common, .. } => common,
        Step::Decide { common, .. } => common,
        Step::Summarize { common, .. } => common,
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
        Step::Plan { .. } => "plan".to_string(),
        Step::MasterPlan { .. } => "masterplan".to_string(),
        Step::Decide { .. } => "decide".to_string(),
        Step::Summarize { .. } => "summarize".to_string(),
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
            Step::Plan { .. } => "plan".to_string(),
            Step::MasterPlan { .. } => "masterplan".to_string(),
            Step::Decide { .. } => "decide".to_string(),
            Step::Summarize { .. } => "summarize".to_string(),
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
            Step::Plan { .. } => has_plan = true,
            Step::MasterPlan { .. } => has_masterplan = true,
            Step::Decide { .. } => has_decide = true,
            Step::Summarize { .. } => {}
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
            if has_shell || has_plan || has_masterplan || has_decide {
                shape_errors.push("chat route should not execute workflow steps".to_string());
            }
        }
        "SHELL" => {
            if !has_shell {
                shape_errors.push("shell route missing shell step".to_string());
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
        && (has_shell || has_plan || has_masterplan || has_decide)
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

pub(crate) fn apply_capability_guard(program: &mut Program, route_decision: &RouteDecision) -> bool {
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

pub(crate) fn preview_text(text: &str, max_lines: usize) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn run_shell_one_liner(cmd: &str, workdir: &PathBuf) -> Result<(i32, String)> {
    let out = Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .current_dir(workdir)
        .output()
        .with_context(|| format!("Failed to run shell: {cmd}"))?;
    let code = out.status.code().unwrap_or(1);
    let mut s = String::new();
    if !out.stdout.is_empty() {
        s.push_str(&String::from_utf8_lossy(&out.stdout));
    }
    if !out.stderr.is_empty() {
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    Ok((code, s))
}
