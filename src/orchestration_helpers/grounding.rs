//! @efficiency-role: domain-logic

use crate::*;

fn response_requires_exact_relative_path(reply_instructions: &str) -> bool {
    let lower = reply_instructions.to_ascii_lowercase();
    lower.contains("exact relative path")
        || lower.contains("exact grounded relative file paths")
        || lower.contains("preserve exact grounded relative file paths")
}

fn response_requires_two_bullets_and_entry_point(reply_instructions: &str) -> bool {
    let lower = reply_instructions.to_ascii_lowercase();
    (lower.contains("exactly two bullet points") || lower.contains("exactly 2 bullet points"))
        && lower.contains("entry point:")
}

fn summarized_bullets(step_results: &[StepResult]) -> Option<String> {
    let summarize = step_results
        .iter()
        .find(|result| result.kind == "summarize" && result.ok)
        .map(|result| result.summary.trim())
        .filter(|summary| !summary.is_empty())?;

    let mut bullets = summarize
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            if line.starts_with("- ") || line.starts_with("* ") {
                format!("- {}", line[2..].trim())
            } else {
                format!("- {line}")
            }
        })
        .take(2)
        .collect::<Vec<_>>();

    if bullets.len() == 2 {
        Some(bullets.join("\n"))
    } else {
        None
    }
}

fn selected_exact_grounded_path(step_results: &[StepResult]) -> Option<String> {
    let mut candidates = step_results
        .iter()
        .filter(|result| result.kind == "select" && result.ok)
        .filter_map(|result| result.raw_output.as_deref())
        .flat_map(|raw| raw.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty() && line.contains('/'))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    if candidates.len() == 1 {
        Some(candidates.remove(0))
    } else {
        None
    }
}

fn derive_exact_grounded_path_from_evidence(
    step_results: &[StepResult],
    final_text: &str,
) -> Option<String> {
    let basename_candidates = step_results
        .iter()
        .filter(|result| result.kind == "select" && result.ok)
        .filter_map(|result| result.raw_output.as_deref())
        .flat_map(|raw| raw.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.contains('/'))
        .filter(|line| final_text.contains(*line))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    for basename in basename_candidates {
        let mut matches = step_results
            .iter()
            .filter(|result| result.kind == "shell" && result.ok)
            .filter_map(|result| result.raw_output.as_deref())
            .flat_map(|raw| raw.lines())
            .map(str::trim)
            .filter(|line| {
                !line.is_empty()
                    && line.contains('/')
                    && line.ends_with(&basename)
                    && line.len() > basename.len()
                    && line
                        .as_bytes()
                        .get(line.len().saturating_sub(basename.len() + 1))
                        == Some(&b'/')
            })
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        matches.sort();
        matches.dedup();
        if matches.len() == 1 {
            return Some(matches.remove(0));
        }
    }

    None
}

pub(crate) fn preserve_exact_grounded_path(
    final_text: String,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> String {
    if !response_requires_exact_relative_path(reply_instructions) {
        return final_text;
    }
    let Some(path) = selected_exact_grounded_path(step_results)
        .or_else(|| derive_exact_grounded_path_from_evidence(step_results, &final_text))
    else {
        return final_text;
    };
    if final_text.contains(&path) {
        return final_text;
    }

    let basename = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    if basename.is_empty() || !final_text.contains(basename) {
        return format!("{path}\n{final_text}");
    }

    // Task 023: Find the longest suffix of 'path' that exists in 'final_text'
    // and ends with 'basename'. Replace that suffix with the full 'path'.
    // This prevents doubled prefixes (e.g. if 'sub/main.go' is there, replace it with 'root/sub/main.go').

    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut longest_suffix_match: Option<String> = None;

    // Check suffixes from longest to shortest (basename)
    for i in 0..path_segments.len() {
        let suffix = path_segments[i..].join("/");
        if final_text.contains(&suffix) {
            longest_suffix_match = Some(suffix);
            break;
        }
    }

    if let Some(suffix) = longest_suffix_match {
        // If the longest suffix is the whole path, we're done.
        if suffix == path {
            return final_text;
        }
        // Otherwise, replace the suffix with the full path.
        // We only replace the FIRST occurrence to stay safe.
        final_text.replacen(&suffix, &path, 1)
    } else {
        // Fallback: if even basename isn't found (shouldn't happen due to check above)
        // just prepend.
        format!("{path}\n{final_text}")
    }
}

pub(crate) fn preserve_requested_summary_and_entry_point(
    final_text: String,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> String {
    if !response_requires_two_bullets_and_entry_point(reply_instructions) {
        return final_text;
    }

    let bullets = summarized_bullets(step_results);
    let path = selected_exact_grounded_path(step_results)
        .or_else(|| derive_exact_grounded_path_from_evidence(step_results, &final_text));

    if let (Some(bullets), Some(path)) = (bullets, path) {
        return format!("{bullets}\nEntry point: {path}");
    }

    final_text
}

pub(crate) async fn request_judge_verdict(
    client: &reqwest::Client,
    chat_url: &Url,
    judge_cfg: &Profile,
    scenario: &CalibrationScenario,
    user_message: &str,
    step_results: &[StepResult],
    final_text: &str,
) -> Result<CalibrationJudgeVerdict> {
    let req = chat_request_system_user(
        judge_cfg,
        &judge_cfg.system_prompt,
        &serde_json::json!({
            "scenario_notes": scenario.notes,
            "expected_route": scenario.route,
            "expected_speech_act": scenario.speech_act,
            "user_message": user_message,
            "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
            "final_answer": final_text,
            "markdown_requested": user_requested_markdown(user_message).to_string()
        })
        .to_string(),
        ChatRequestOptions::default(),
    );
    chat_json_with_repair(client, chat_url, &req).await
}

#[cfg(test)]
mod tests {
    use super::{preserve_exact_grounded_path, preserve_requested_summary_and_entry_point};
    use crate::StepResult;

    #[test]
    fn preserve_exact_grounded_path_replaces_shortened_basename() {
        let step_results = vec![StepResult {
            id: "sel1".to_string(),
            kind: "select".to_string(),
            ok: true,
            raw_output: Some("_stress_testing/_opencode_for_testing/main.go".to_string()),
            ..StepResult::default()
        }];

        let final_text = preserve_exact_grounded_path(
            "main.go is the primary entry point.".to_string(),
            &step_results,
            "State the selected exact relative path first, then explain briefly why it is the strongest grounded entry point.",
        );

        assert!(final_text.starts_with("_stress_testing/_opencode_for_testing/main.go"));
    }

    #[test]
    fn preserve_exact_grounded_path_prepends_when_basename_missing() {
        let step_results = vec![StepResult {
            id: "sel1".to_string(),
            kind: "select".to_string(),
            ok: true,
            raw_output: Some("_stress_testing/_opencode_for_testing/main.go".to_string()),
            ..StepResult::default()
        }];

        let final_text = preserve_exact_grounded_path(
            "main.go is the primary entry point.".to_string(),
            &step_results,
            "State the selected exact relative path first, then explain briefly why it is the strongest grounded entry point.",
        );

        assert!(final_text.starts_with("_stress_testing/_opencode_for_testing/main.go"));
    }

    #[test]
    fn preserve_exact_grounded_path_can_recover_path_from_shell_evidence() {
        let step_results = vec![
            StepResult {
                id: "s2".to_string(),
                kind: "shell".to_string(),
                ok: true,
                raw_output: Some(
                    "_stress_testing/_opencode_for_testing/main.go\n_stress_testing/_opencode_for_testing/cmd/root.go"
                        .to_string(),
                ),
                ..StepResult::default()
            },
            StepResult {
                id: "sel1".to_string(),
                kind: "select".to_string(),
                ok: true,
                raw_output: Some("main.go".to_string()),
                ..StepResult::default()
            },
        ];

        let final_text = preserve_exact_grounded_path(
            "main.go is the strongest grounded primary entry point.".to_string(),
            &step_results,
            "State the selected exact relative path first, then explain briefly why it is the strongest grounded entry point.",
        );

        assert!(final_text.starts_with("_stress_testing/_opencode_for_testing/main.go"));
    }

    #[test]
    fn preserve_requested_summary_and_entry_point_restores_missing_bullets_and_path_line() {
        let step_results = vec![
            StepResult {
                id: "sum1".to_string(),
                kind: "summarize".to_string(),
                ok: true,
                summary: "- First repo purpose\n- Second repo purpose".to_string(),
                ..StepResult::default()
            },
            StepResult {
                id: "sel1".to_string(),
                kind: "select".to_string(),
                ok: true,
                raw_output: Some("_stress_testing/_opencode_for_testing/main.go".to_string()),
                ..StepResult::default()
            },
        ];

        let final_text = preserve_requested_summary_and_entry_point(
            "Primary entry point: _stress_testing/_opencode_for_testing/main.go".to_string(),
            &step_results,
            "Return exactly two bullet points from the grounded README summary first. Then add one final line that starts with `Entry point:` followed by the selected exact relative path. Preserve exact grounded relative file paths from the evidence and do not mention files that were not observed.",
        );

        assert!(final_text.contains("- First repo purpose"));
        assert!(final_text.contains("- Second repo purpose"));
        assert!(final_text.contains("Entry point: _stress_testing/_opencode_for_testing/main.go"));
    }
}
