use crate::*;

pub(crate) fn calibration_metric(correct: usize, total: usize) -> CalibrationMetric {
    CalibrationMetric {
        total,
        correct,
        accuracy: if total == 0 {
            0.0
        } else {
            correct as f64 / total as f64
        },
    }
}

pub(crate) fn build_confusions(pairs: &[(String, String)]) -> Vec<CalibrationConfusion> {
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for (expected, predicted) in pairs {
        *counts
            .entry((expected.clone(), predicted.clone()))
            .or_insert(0usize) += 1;
    }
    let mut out: Vec<CalibrationConfusion> = counts
        .into_iter()
        .map(|((expected, predicted), count)| CalibrationConfusion {
            expected,
            predicted,
            count,
        })
        .collect();
    out.sort_by(|a, b| {
        a.expected
            .cmp(&b.expected)
            .then(a.predicted.cmp(&b.predicted))
    });
    out
}

pub(crate) fn metric_accuracy_or_neutral(correct: usize, total: usize) -> f64 {
    if total == 0 {
        1.0
    } else {
        correct as f64 / total as f64
    }
}

pub(crate) fn is_workflow_calibration_scenario(scenario: &CalibrationScenario) -> bool {
    scenario.workflow.eq_ignore_ascii_case("WORKFLOW")
}

pub(crate) fn is_response_calibration_scenario(scenario: &CalibrationScenario) -> bool {
    if scenario.route.eq_ignore_ascii_case("CHAT") {
        return true;
    }
    matches!(
        scenario.mode.as_deref(),
        Some("INSPECT") | Some("PLAN") | Some("MASTERPLAN") | Some("DECIDE")
    ) || matches!(scenario.route.as_str(), "PLAN" | "MASTERPLAN" | "DECIDE")
}

/// Filter scenarios based on tuning mode.
/// Quick mode: Only 5 critical scenarios for fast startup tuning.
/// Full mode: All scenarios for comprehensive calibration.
pub(crate) fn filter_scenarios_by_mode(
    scenarios: Vec<CalibrationScenario>,
    tune_mode: &str,
) -> Vec<CalibrationScenario> {
    if tune_mode == "quick" {
        // Quick mode: Only critical scenarios for core classification
        // These scenarios test: speech_act, workflow, mode, route, program_parse
        scenarios
            .into_iter()
            .filter(|s| {
                // Prioritize scenarios that test core classification
                matches!(s.speech_act.as_str(), "INFO_REQUEST" | "ACTION_REQUEST")
                    && s.route.eq_ignore_ascii_case("WORKFLOW")
            })
            .take(5) // Limit to 5 scenarios for quick tuning
            .collect()
    } else {
        // Full mode: All scenarios
        scenarios
    }
}

pub(crate) fn summarize_evidence_compact(compact: &EvidenceCompact) -> String {
    let mut lines = Vec::new();
    if !compact.summary.trim().is_empty() {
        lines.push(compact.summary.trim().to_string());
    }
    for fact in compact.key_facts.iter().take(6) {
        let fact = fact.trim();
        if !fact.is_empty() {
            lines.push(format!("- {fact}"));
        }
    }
    lines.join("\n")
}

pub(crate) fn summarize_artifact_classification(classification: &ArtifactClassification) -> String {
    let fmt = |label: &str, items: &[String]| -> Option<String> {
        let values: Vec<String> = items
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .take(6)
            .collect();
        if values.is_empty() {
            None
        } else {
            Some(format!("{label}: {}", values.join(", ")))
        }
    };
    [
        fmt("safe", &classification.safe),
        fmt("maybe", &classification.maybe),
        fmt("keep", &classification.keep),
        fmt("ignore", &classification.ignore),
        if classification.reason.trim().is_empty() {
            None
        } else {
            Some(format!("reason: {}", classification.reason.trim()))
        },
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn scope_contains_expected_terms(scope: &ScopePlan, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!(
        "{}\n{}\n{}\n{}",
        scope.objective,
        scope.focus_paths.join("\n"),
        scope.include_globs.join("\n"),
        scope.query_terms.join("\n")
    )
    .to_lowercase();
    terms
        .iter()
        .all(|term| haystack.contains(&term.to_lowercase()))
}

pub(crate) fn scope_avoids_forbidden_terms(scope: &ScopePlan, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!(
        "{}\n{}\n{}\n{}",
        scope.objective,
        scope.focus_paths.join("\n"),
        scope.include_globs.join("\n"),
        scope.exclude_globs.join("\n")
    )
    .to_lowercase();
    !terms.iter().any(|term| {
        haystack.contains(&term.to_lowercase())
            && !scope
                .exclude_globs
                .iter()
                .any(|g| g.to_lowercase().contains(&term.to_lowercase()))
    })
}

pub(crate) fn text_contains_keywords(text: &str, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    let lower = text.to_lowercase();
    keywords.iter().all(|kw| lower.contains(&kw.to_lowercase()))
}

pub(crate) fn text_avoids_keywords(text: &str, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    let lower = text.to_lowercase();
    !keywords.iter().any(|kw| lower.contains(&kw.to_lowercase()))
}

pub(crate) fn classification_has_categories(
    classification: &ArtifactClassification,
    categories: &[String],
) -> bool {
    if categories.is_empty() {
        return true;
    }
    let mut present = Vec::new();
    if !classification.safe.is_empty() {
        present.push("safe");
    }
    if !classification.maybe.is_empty() {
        present.push("maybe");
    }
    if !classification.keep.is_empty() {
        present.push("keep");
    }
    if !classification.ignore.is_empty() {
        present.push("ignore");
    }
    categories
        .iter()
        .all(|category| present.iter().any(|p| p.eq_ignore_ascii_case(category)))
}

pub(crate) fn tool_economy_score(
    step_count: usize,
    min_steps: Option<usize>,
    max_steps: Option<usize>,
) -> f64 {
    let min_steps = min_steps.unwrap_or(step_count.max(1));
    let max_steps = max_steps.unwrap_or(step_count.max(min_steps));
    if step_count < min_steps {
        (step_count as f64 / min_steps as f64).clamp(0.0, 1.0)
    } else if step_count <= max_steps {
        1.0
    } else {
        (max_steps as f64 / step_count as f64).clamp(0.0, 1.0)
    }
}

pub(crate) fn save_stage_score_note(dir: &Path, stage: &str, note: &str) -> Result<()> {
    let path = dir.join(format!("{stage}_score.txt"));
    std::fs::write(&path, note).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub(crate) fn save_calibration_report(path: &PathBuf, report: &CalibrationReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s =
        serde_json::to_string_pretty(report).context("Failed to serialize calibration report")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn load_calibration_report(path: &PathBuf) -> Result<CalibrationReport> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read calibration report at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("calibration report is not valid UTF-8")?;
    serde_json::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_efficiency_report(path: &PathBuf, report: &EfficiencyReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s =
        serde_json::to_string_pretty(report).context("Failed to serialize efficiency report")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn load_efficiency_report(path: &PathBuf) -> Result<EfficiencyReport> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read efficiency report at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("efficiency report is not valid UTF-8")?;
    serde_json::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn efficiency_metric_from_score(score_sum: f64, total: usize) -> EfficiencyMetric {
    EfficiencyMetric {
        total,
        score: if total == 0 {
            0.0
        } else {
            score_sum / total as f64
        },
    }
}
