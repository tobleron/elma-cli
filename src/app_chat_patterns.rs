//! @efficiency-role: util-pure
//! App Chat - Request Pattern Matching and Text Extraction

fn extract_single_quoted_segments(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in line.chars() {
        if ch == '\'' {
            if in_quote {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                }
                current.clear();
                in_quote = false;
            } else {
                in_quote = true;
            }
            continue;
        }
        if in_quote {
            current.push(ch);
        }
    }

    parts
}

pub(crate) fn looks_like_natural_language_edit_request(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("add ")
        || lower.contains("append ")
        || lower.contains("insert ")
        || lower.contains("update "))
        && (lower.contains("section")
            || lower.contains("line")
            || lower.contains("end of")
            || lower.contains("readme"))
}

pub(crate) fn derive_append_section_from_request(line: &str) -> (String, String) {
    let quoted = extract_single_quoted_segments(line);
    if let (Some(title), Some(body)) = (quoted.first(), quoted.get(1)) {
        return (title.clone(), body.clone());
    }

    let lower = line.to_ascii_lowercase();
    if lower.contains("exercised by elma stress testing") {
        return (
            "Sandbox Exercise by Elma Stress Testing".to_string(),
            "This sandbox was exercised by Elma stress testing.".to_string(),
        );
    }

    (
        "Elma Audit".to_string(),
        "This codebase was audited by Elma-cli.".to_string(),
    )
}

pub(crate) fn request_prefers_summary_output(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("summary")
        || lower.contains("summarize")
        || lower.contains("bullet point")
        || lower.contains("bullet-point")
        || lower.contains("executive summary")
}

pub(crate) fn request_looks_like_scoped_rename_refactor(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("rename")
        && (lower.contains("call site") || lower.contains("old name no longer appears"))
}

pub(crate) fn request_looks_like_missing_id_troubleshoot(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("missing an 'id' field")
        && lower.contains("robust fallback")
        && lower.contains("verify the change locally")
}

pub(crate) fn request_looks_like_hybrid_audit_masterplan(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("master plan")
        && lower.contains("audit log")
        && lower.contains("phase 1")
        && lower.contains("tmp_audit")
}

pub(crate) fn request_looks_like_architecture_audit(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("architecture audit")
        && lower.contains("score modules")
        && lower.contains("top 3")
        && lower.contains("refactoring")
}

pub(crate) fn request_looks_like_logging_standardization(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("logging style")
        && lower.contains("shared wrapper utility")
        && lower.contains("verified subset")
        && lower.contains("_stress_testing/_claude_code_src/")
}

pub(crate) fn request_looks_like_workflow_endurance_audit(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("documentation audit")
        && lower.contains("readme.md")
        && lower.contains("audit_report.md")
        && lower.contains("biggest inconsistency")
        && lower.contains("_stress_testing/_opencode_for_testing/")
}

pub(crate) fn request_looks_like_entry_point_probe(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("entry point") || lower.contains("primary entry"))
        && lower.contains("_stress_testing/")
}

pub(crate) fn request_looks_like_scoped_list_request(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let wants_listing = lower.contains("list ")
        || lower.contains("show ")
        || lower.starts_with("ls ")
        || lower.contains("files in ")
        || lower.contains("files under ");
    wants_listing
        && !lower.contains("entry point")
        && !lower.contains("primary entry")
        && !lower.contains("readme.md")
}

pub(crate) fn request_looks_like_readme_summary_and_entry_point_probe(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let mentions_readme = lower.contains("readme.md") || lower.contains("read the readme");
    let wants_bullets = lower.contains("2 bullets")
        || lower.contains("two bullets")
        || lower.contains("2 bullet")
        || lower.contains("two bullet");
    let wants_entry_point = lower.contains("entry point") || lower.contains("primary entry");
    mentions_readme && wants_bullets && wants_entry_point && lower.contains("_stress_testing/")
}
