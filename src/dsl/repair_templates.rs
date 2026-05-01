//! Concrete repair templates for DSL parse failures.
//!
//! When a model produces invalid DSL, the repair feedback must include the
//! exact expected format — not a generic "valid DSL" placeholder. This module
//! maps from the first token (or error context) to a concrete format template
//! that appears in the repair message.
//!
//! Templates are derived from the canonical DSL contracts. This file is the
//! single source of truth for repair format hints.

/// Given the raw model output, detect which DSL format was intended and return
/// a concrete expected-format hint for the repair message.
pub fn detect_expected_format(raw_output: &str) -> String {
    let cleaned = strip_for_format_detection(raw_output);
    let first_token = first_non_empty_token(&cleaned);

    match first_token {
        // ── Classifier DSLs ──
        "ACT" => "ACT choice=N label=CHAT|INSTRUCT|INQUIRE reason=\"justification\" entropy=N.N"
            .to_string(),
        "ROUTE" => "ROUTE choice=N label=CHAT|WORKFLOW reason=\"justification\" entropy=N.N"
            .to_string(),
        "MODE" => {
            // MODE is used by both workflow-mode and evidence-mode classifiers.
            // The valid labels differ — provide a useful diagnostic.
            "MODE choice=N label=LABEL reason=\"justification\" entropy=N.N".to_string()
        }

        // ── Assessor DSLs ──
        "ASSESS" => {
            // ASSESS is reused across complexity, evidence-needs, action-needs,
            // evidence-quality, evidence-sufficiency, evidence-staleness, and
            // pattern-suggester units.  The fields vary — provide the most
            // common shape.
            "ASSESS key=value key2=\"val\"   (one line, fields vary by unit)".to_string()
        }
        "TOOLS" => "TOOLS needs_tools=true|false".to_string(),

        // ── Formula / Workflow / Scope ──
        "FORMULA" => "FORMULA primary=formula_name alt1=... alt2=... reason=\"justification\""
            .to_string(),
        "WORKFLOW" => "WORKFLOW objective=\"one sentence\" complexity=DIRECT risk=LOW ... END"
            .to_string(),
        "SCOPE" => "SCOPE objective=\"text\"\nF path=\"relative/path\"\nQ text=\"query\"\nEND"
            .to_string(),

        // ── Pyramid / Decomposition ──
        "OBJECTIVE" => "OBJECTIVE text=\"one line\" risk=low|medium|high\nGOAL ...\nTASK ...\nEND"
            .to_string(),
        "GOAL" => "GOAL text=\"description\" evidence_needed=true|false".to_string(),
        "TASK" => "TASK id=N text=\"description\" status=ready|pending".to_string(),
        "NEXT" => {
            "NEXT task_id=N action=read|list|search|shell|edit|ask|done reason=\"justification\""
                .to_string()
        }

        // ── Turn Summarizer ──
        "TURN" => {
            "TURN summary_narrative=\"compact narrative\" status_category=completed|partial|failed"
                .to_string()
        }

        // ── Claim / Verdict / Critic ──
        "CLAIM" => {
            "CLAIM statement=\"...\" evidence_ids=\"id\" status=GROUNDED|UNGROUNDED\n...\nREASON text=\"one sentence\"\nEND".to_string()
        }
        "VERDICT" => {
            "VERDICT status=ok|revise reason=\"justification\" unsupported_claims=\"...\"".to_string()
        }
        "OK" | "RETRY" | "REVISE" => format!(
            "{token} reason=\"justification\"",
            token = first_token
        ),
        "CAUTION" => "CAUTION reason=\"justification\"".to_string(),

        // ── Evidence Compactor / Artifact ──
        "RESULT" => {
            "RESULT summary=\"concise summary\" key_facts=\"fact1,fact2\" noise=\"noise\""
                .to_string()
        }
        "ARTIFACT" => {
            "ARTIFACT safe=\"a,b\" maybe=\"c\" keep=\"d\" ignore=\"e\" reason=\"one sentence\""
                .to_string()
        }

        // ── Selector ──
        "ITEM" => "ITEM value=\"exact item\"\n...\nREASON text=\"one sentence\"\nEND".to_string(),

        // ── Other intel units ──
        "RENAME" => "RENAME identifier=\"newId\" reason=\"justification\"".to_string(),
        "MEMORY" | "MATCH" => "MEMORY memory_id=\"id_or_empty\"".to_string(),
        "REASON" => "REASON text=\"one short sentence\"".to_string(),
        "GATE" => "GATE status=save|skip reason=\"justification\"".to_string(),
        "REFLECT" => "REFLECT confidence=N.NN justification=\"justification\"".to_string(),
        "REPAIR" => "REPAIR cmd=\"shell one-liner\" reason=\"justification\"".to_string(),
        "SEMANTICS" => "SEMANTICS status=accept|reject reason=\"justification\"".to_string(),
        "PREFLIGHT" => "PREFLIGHT status=accept|revise|reject reason=\"...\" cmd=\"...\"".to_string(),
        "STATUS" => "STATUS status=\"Processing...\"".to_string(),
        "EXPERT" => "EXPERT advisor=\"direct|explanatory|cautious: reason\"".to_string(),
        "STEP" => "STEP num=N instruction=\"plain English\" ... END".to_string(),
        "JUDGE" => {
            "JUDGE status=pass|fail reason=\"...\" answered_request=true|false faithful_to_evidence=true|false".to_string()
        }

        // ── Action DSL ──
        "R" => "R path=\"relative/path\"".to_string(),
        "L" => "L path=\"src\" depth=1".to_string(),
        "S" => "S q=\"search text\" path=\"src\"".to_string(),
        "Y" => "Y q=\"symbol\" path=\"src\"".to_string(),
        "E" => "E path=\"file\"\n---OLD\nold text\n---NEW\nnew text\n---END".to_string(),
        "X" => "X\n<shell_command>\n---END".to_string(),
        "ASK" => "ASK\n<question>\n---END".to_string(),
        "DONE" => "DONE summary=\"one-line summary\"".to_string(),
        "UTIL" => "UTIL action=read|search|shell path=\"...\"".to_string(),
        // ── Action Selector ──
        "SELECT" => "SELECT action=R reason=\"short justification\"".to_string(),

        // ── Fallback: no recognizable token ──
        _ => {
            "one DSL line: COMMAND key=value key2=\"val\" (uppercase command, explicit field names)"
                .to_string()
        }
    }
}

/// Extract the first non-empty token from the cleaned model output.
fn first_non_empty_token(text: &str) -> &str {
    // Skip common prefixes that are not DSL
    let trimmed = text.trim();
    // Strip leading punctuation/fencing that might appear
    let trimmed = trimmed.trim_start_matches('`').trim_start();
    for word in trimmed.split_whitespace() {
        let w = word.trim().trim_end_matches(':');
        // Skip words that look like prose (lowercase sentence starters)
        if w.chars().all(|c| c.is_uppercase() || c.is_ascii_digit()) {
            return w;
        }
        // Words like "choice=1" suggest missing command token in classifier
        if w.contains('=') && w.contains(|c| c == 'c' || c == 'C') {
            // Could be a partial classifier — return empty to trigger fallback
        }
    }
    ""
}

/// Strip thinking blocks, XML tool-call wrappers, and Markdown fences so we
/// can extract the first real DSL token.
fn strip_for_format_detection(text: &str) -> String {
    crate::text_utils::strip_thinking_blocks(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_mode_format() {
        let raw = "MODE choice=1 label=DECIDE";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("MODE choice=N label="));
    }

    #[test]
    fn detect_act_format() {
        let raw = "ACT choice=2 label=INSTRUCT reason=\"user wants action\" entropy=0.2";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("ACT choice=N label=CHAT|INSTRUCT|INQUIRE"));
    }

    #[test]
    fn detect_assess_format() {
        let raw = "ASSESS needs_evidence=true";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("ASSESS key=value"));
    }

    #[test]
    fn detect_action_r_format() {
        let raw = "R path=/Users/r2/elma-cli";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("R path=\"relative/path\""));
    }

    #[test]
    fn detect_action_done_format() {
        let raw = "DONE summary=\"task completed\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("DONE summary=\"one-line summary\""));
    }

    #[test]
    fn detect_turn_format() {
        let raw = "TURN summary_narrative=\"done\" status_category=completed";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("TURN summary_narrative="));
    }

    #[test]
    fn detect_formula_format() {
        let raw = "FORMULA primary=reply_only alt1=inspect_reply reason=\"simple query\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("FORMULA primary="));
    }

    #[test]
    fn detect_next_format() {
        let raw = "NEXT task_id=2 action=edit reason=\"user requested\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("NEXT task_id=N action="));
    }

    #[test]
    fn detect_verdict_format() {
        let raw = "VERDICT status=ok reason=\"evidence sufficient\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("VERDICT status=ok|revise"));
    }

    #[test]
    fn detect_gate_format() {
        let raw = "GATE status=save reason=\"session relevant\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("GATE status=save|skip"));
    }

    #[test]
    fn fallback_for_prose_only() {
        let raw = "I'm repeating the same action type without progress";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("one DSL line: COMMAND key=value"));
    }

    #[test]
    fn fallback_for_thinking_only() {
        let raw = "<think>let me analyze this...</think>";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("one DSL line: COMMAND key=value"));
    }

    #[test]
    fn detect_claim_format() {
        let raw = "CLAIM statement=\"the file exists\" evidence_ids=\"e_001\" status=GROUNDED";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("CLAIM statement="));
    }

    #[test]
    fn detect_ok_verdict() {
        let raw = "OK reason=\"all tests pass\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("OK reason=\"justification\""));
    }

    #[test]
    fn detect_retry_verdict() {
        let raw = "RETRY reason=\"evidence missing\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("RETRY reason=\"justification\""));
    }

    #[test]
    fn detect_objective_format() {
        let raw = "OBJECTIVE text=\"fix bug\" risk=low";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("OBJECTIVE text="));
    }

    #[test]
    fn detect_action_s_format() {
        let raw = "S q=\"needle\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("S q=\"search text\""));
    }

    #[test]
    fn detect_preflight_format() {
        let raw = "PREFLIGHT status=accept reason=\"safe\" cmd=\"ls\"";
        let fmt = detect_expected_format(raw);
        assert!(fmt.contains("PREFLIGHT status=accept|revise|reject"));
    }

    #[test]
    fn detect_target_format() {
        let raw = "F path=\"src/main.rs\"";
        let fmt = detect_expected_format(raw);
        // F alone is not a recognized top-level DSL command — should fall back
        assert!(fmt.contains("one DSL line: COMMAND key=value"));
    }
}
