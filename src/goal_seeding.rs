//! @efficiency-role: domain-logic
//!
//! Goal Seeding from Multi-Step Requests (T305)
//!
//! Lightweight heuristic that extracts independent action clauses from the
//! first user message and seeds them as pending subgoals.  Uses word-boundary
//! matching, imperative verb detection, and clause splitting — no LLM call,
//! no external dependency.

use crate::types_core::GoalState;

/// Canonical (base-form) action verbs that signal an independent sub-task.
/// Each entry includes common inflections for robust matching.
const ACTION_VERB_GROUPS: &[&[&str]] = &[
    // File/workspace operations
    &["scan", "scans", "scanning", "scanned"],
    &["extract", "extracts", "extracting", "extracted"],
    &["identify", "identifies", "identifying", "identified"],
    &["find", "finds", "finding", "found"],
    &["list", "lists", "listing", "listed"],
    &["search", "searches", "searching", "searched"],
    &["locate", "locates", "locating", "located"],
    // Content/text operations
    &["summarize", "summarizes", "summarizing", "summarized"],
    &["count", "counts", "counting", "counted"],
    &["analyze", "analyzes", "analyzing", "analyzed"],
    &["parse", "parses", "parsing", "parsed"],
    &["check", "checks", "checking", "checked"],
    &["validate", "validates", "validating", "validated"],
    &["verify", "verifies", "verifying", "verified"],
    &["inspect", "inspects", "inspecting", "inspected"],
    // Creation/output operations
    &["write", "writes", "writing", "written", "wrote"],
    &["generate", "generates", "generating", "generated"],
    &["create", "creates", "creating", "created"],
    &["build", "builds", "building", "built"],
    &["compile", "compiles", "compiling", "compiled"],
    &["render", "renders", "rendering", "rendered"],
    &["produce", "produces", "producing", "produced"],
    &["output", "outputs", "outputting"],
    &["format", "formats", "formatting", "formatted"],
    &["convert", "converts", "converting", "converted"],
    // File system operations
    &["read", "reads", "reading"],
    &["copy", "copies", "copying", "copied"],
    &["move", "moves", "moving", "moved"],
    &["delete", "deletes", "deleting", "deleted"],
    &["remove", "removes", "removing", "removed"],
    &["rename", "renames", "renaming", "renamed"],
    &["save", "saves", "saving", "saved"],
    &["store", "stores", "storing", "stored"],
    &["download", "downloads", "downloading", "downloaded"],
    &["upload", "uploads", "uploading", "uploaded"],
    // Code/development operations
    &["test", "tests", "testing", "tested"],
    &["run", "runs", "running", "ran"],
    &["execute", "executes", "executing", "executed"],
    &["install", "installs", "installing", "installed"],
    &["configure", "configures", "configuring", "configured"],
    &["setup", "setups", "setting up", "set up"],
    &["deploy", "deploys", "deploying", "deployed"],
    &["implement", "implements", "implementing", "implemented"],
    &["refactor", "refactors", "refactoring", "refactored"],
    &["debug", "debugs", "debugging", "debugged"],
    &["fix", "fixes", "fixing", "fixed"],
    &["update", "updates", "updating", "updated"],
    &["upgrade", "upgrades", "upgrading", "upgraded"],
    // Data operations
    &["filter", "filters", "filtering", "filtered"],
    &["sort", "sorts", "sorting", "sorted"],
    &["group", "groups", "grouping", "grouped"],
    &["aggregate", "aggregates", "aggregating", "aggregated"],
    &["transform", "transforms", "transforming", "transformed"],
    &["merge", "merges", "merging", "merged"],
    &["split", "splits", "splitting"],
    &["map", "maps", "mapping", "mapped"],
    // Communication/reporting
    &["report", "reports", "reporting", "reported"],
    &["log", "logs", "logging", "logged"],
    &["display", "displays", "displaying", "displayed"],
    &["show", "shows", "showing", "showed", "shown"],
    &["print", "prints", "printing", "printed"],
    &["visualize", "visualizes", "visualizing", "visualized"],
    &["notify", "notifies", "notifying", "notified"],
    &["alert", "alerts", "alerting", "alerted"],
    &["send", "sends", "sending", "sent"],
    &["receive", "receives", "receiving", "received"],
    // Planning/organizing
    &["plan", "plans", "planning", "planned"],
    &["organize", "organizes", "organizing", "organized"],
    &["categorize", "categorizes", "categorizing", "categorized"],
    &["classify", "classifies", "classifying", "classified"],
    &["prioritize", "prioritizes", "prioritizing", "prioritized"],
    &["schedule", "schedules", "scheduling", "scheduled"],
    &["estimate", "estimates", "estimating", "estimated"],
    // Comparison/evaluation
    &["compare", "compares", "comparing", "compared"],
    &["evaluate", "evaluates", "evaluating", "evaluated"],
    &["assess", "assesses", "assessing", "assessed"],
    &["review", "reviews", "reviewing", "reviewed"],
    &["audit", "audits", "auditing", "audited"],
    &["benchmark", "benchmarks", "benchmarking", "benchmarked"],
    &["measure", "measures", "measuring", "measured"],
    // Cleanup/maintenance
    &["clean", "cleans", "cleaning", "cleaned"],
    &["purge", "purges", "purging", "purged"],
    &["archive", "archives", "archiving", "archived"],
    &["compress", "compresses", "compressing", "compressed"],
    &["decompress", "decompresses", "decompressing", "decompressed"],
    &["optimize", "optimizes", "optimizing", "optimized"],
];

/// Conjunctions that separate independent clauses.
const CLAUSE_SPLITTERS: &[&str] = &[
    ", and then ", ", and also ", ", and finally ", ", but also ",
    ", then ", ", finally ", ", also ", ", additionally ",
    ", and ", ", but ", ", or ",
    " and then ", " and also ", " and finally ",
    ". Then ", ". Finally ", ". Also ", ". Additionally ",
    ";\nthen ", ";\nfinally ", "; then ", "; finally ",
];

/// Prefix patterns that introduce action clauses (imperative mood).
/// After these words, the next verb is likely an action to perform.
const ACTION_INTRODUCERS: &[&str] = &[
    "please ", "also ", "then ", "additionally ", "furthermore ",
    "next, ", "first, ", "second, ", "third, ", "lastly, ",
];

/// Verbs that are action words but NOT imperative requests (descriptive).
/// If the sentence is "can you list..." or "I want to find...", the verb
/// is still a real action — we just need to handle the framing.
const REQUEST_FRAMES: &[&str] = &[
    "can you ", "could you ", "would you ", "please ",
    "i want to ", "i need to ", "i'd like to ", "i would like to ",
    "i want you to ", "i need you to ",
];

/// If the user message contains multiple independent actions (separated by
/// conjunctions or containing ≥2 distinct action verbs), seed them as
/// pending subgoals and set the overall objective.
pub(crate) fn seed_goals_if_multi_step(line: &str, goal_state: &mut GoalState) {
    if goal_state.has_active_goal() || line.trim().len() < 15 {
        return;
    }

    let normalized = normalize_for_matching(line);
    let clauses = split_into_clauses(line);

    // For each clause, extract the action phrase
    let action_clauses: Vec<String> = clauses
        .iter()
        .filter_map(|c| extract_action_phrase(c, &normalized))
        .collect();

    // Only seed if we found ≥2 independent action clauses
    if action_clauses.len() < 2 {
        // Still seed the objective even for single-action requests
        seed_objective_only(line, goal_state);
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    goal_state.active_objective = Some(line.trim().to_string());
    goal_state.pending_subgoals = action_clauses;
    goal_state.created_at = now;
    goal_state.last_updated = now;
}

/// Seed just the objective (no subgoals) for single-action requests.
fn seed_objective_only(line: &str, goal_state: &mut GoalState) {
    let trimmed = line.trim();
    if trimmed.len() < 15 {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    goal_state.active_objective = Some(trimmed.to_string());
    goal_state.created_at = now;
    goal_state.last_updated = now;
}

/// Strip request framing to get to the core action.
/// "Can you list all files..." → "list all files..."
fn strip_request_frame(clause: &str) -> &str {
    let lower = clause.to_lowercase();
    for frame in REQUEST_FRAMES {
        if lower.starts_with(frame) {
            let start = frame.len();
            let remaining = &clause[start..];
            // Capitalize first letter
            let mut chars = remaining.chars();
            if let Some(first) = chars.next() {
                let mut result = first.to_uppercase().to_string();
                result.push_str(chars.as_str());
                // Return a static-friendly version by using the original string
                return remaining.trim_start();
            }
        }
    }
    clause.trim_start()
}

/// Extract the action phrase from a clause if it contains an action verb.
/// Returns the clause with request framing stripped, or None if no action.
fn extract_action_phrase(clause: &str, _full_normalized: &str) -> Option<String> {
    let stripped = strip_request_frame(clause);
    if stripped.len() < 5 {
        return None;
    }

    let lower = stripped.to_lowercase();
    let has_action = contains_action_verb_with_boundaries(&lower);
    if !has_action {
        return None;
    }

    // Clean up: remove trailing punctuation, normalize whitespace
    let cleaned = stripped
        .trim_end_matches(|c: char| c == '.' || c == ',' || c == ';' || c == '!')
        .trim()
        .to_string();

    if cleaned.is_empty() {
        return None;
    }

    Some(cleaned)
}

/// Check if text contains an action verb, using word-boundary matching.
/// "write" matches but "rewrite" and "overwrite" do NOT.
/// Handles inflections: write, writes, writing, wrote, written.
fn contains_action_verb_with_boundaries(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();

    for group in ACTION_VERB_GROUPS {
        for verb_form in *group {
            if word_exists_with_boundaries(&words, verb_form) {
                return true;
            }
        }
    }
    false
}

/// Check if a word exists in the word list with proper boundaries.
/// Handles cases where the verb is followed by punctuation: "scan," "files."
fn word_exists_with_boundaries(words: &[&str], target: &str) -> bool {
    for word in words {
        // Strip trailing punctuation from the word for comparison
        let clean_word = word.trim_end_matches(|c: char| {
            c == '.' || c == ',' || c == ';' || c == ':' || c == '!' || c == '?'
                || c == '"' || c == '\'' || c == ')' || c == ']'
        });

        if clean_word.eq_ignore_ascii_case(target) {
            return true;
        }
    }
    false
}

/// Split a user message into candidate action clauses.
fn split_into_clauses(line: &str) -> Vec<String> {
    // Try conjunction-based splitting first
    for splitter in CLAUSE_SPLITTERS {
        let parts: Vec<&str> = line.split(splitter).collect();
        if parts.len() >= 2 {
            return parts
                .iter()
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
        }
    }

    // Try semicolon splitting
    if line.contains(';') {
        return line
            .split(';')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
    }

    // Try numbered list splitting (1. ... 2. ... or - ...)
    let numbered: Vec<&str> = line.split(&['\n', '\r']).collect();
    if numbered.len() >= 2
        && numbered.iter().any(|l| {
            let t = l.trim();
            t.starts_with(|c: char| c.is_ascii_digit()) && t.contains(". ")
                || t.starts_with("- ")
                || t.starts_with("* ")
        })
    {
        return numbered
            .iter()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
    }

    // No splitting possible — return as single clause
    vec![line.to_string()]
}

/// Normalize text for matching: lowercase, collapse whitespace.
fn normalize_for_matching(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_goal() -> GoalState {
        GoalState::default()
    }

    #[test]
    fn test_seeds_multi_step_request() {
        let line = "Scan the workspace for source files, extract TODO comments, and identify files larger than 1MB";
        let mut gs = empty_goal();
        seed_goals_if_multi_step(line, &mut gs);

        assert!(gs.has_active_goal());
        assert!(gs.pending_subgoals.len() >= 2);
    }

    #[test]
    fn test_seeds_multi_step_with_periods() {
        let line = "Write a script to scan files. Then generate a report. Finally clean up temp files.";
        let mut gs = empty_goal();
        seed_goals_if_multi_step(line, &mut gs);

        assert!(gs.has_active_goal());
        assert!(gs.pending_subgoals.len() >= 2);
    }

    #[test]
    fn test_seeds_request_framing() {
        let line = "Can you find all Python files and then count how many there are?";
        let mut gs = empty_goal();
        seed_goals_if_multi_step(line, &mut gs);

        assert!(gs.has_active_goal());
        assert!(gs.pending_subgoals.len() >= 2);
    }

    #[test]
    fn test_no_false_positive_rewrite() {
        // "rewrite" should NOT match as action verb (it's not in our list)
        assert!(!contains_action_verb_with_boundaries("i need to rewrite this file"));
    }

    #[test]
    fn test_word_boundary_matching() {
        assert!(contains_action_verb_with_boundaries("scan all files"));
        assert!(contains_action_verb_with_boundaries("scanning files"));
        assert!(contains_action_verb_with_boundaries("scanned the directory"));
        assert!(contains_action_verb_with_boundaries("write a script"));
        assert!(contains_action_verb_with_boundaries("writing a script"));
        assert!(contains_action_verb_with_boundaries("wrote the script"));
        assert!(contains_action_verb_with_boundaries("find the bug"));
        assert!(contains_action_verb_with_boundaries("found the bug"));
    }

    #[test]
    fn test_no_false_positive_descriptive() {
        // "I found that the scanner is broken, can you fix it and write a report?"
        // Single clause (no splitter matches " and " without comma), so objective
        // is seeded but no subgoals. This is correct behavior — subgoal extraction
        // requires clear clause boundaries.
        let line = "I found that the scanner is broken, can you fix it and write a report?";
        let mut gs = empty_goal();
        seed_goals_if_multi_step(line, &mut gs);

        assert!(gs.has_active_goal());
        // Single clause → objective seeded, no subgoals
        assert!(gs.pending_subgoals.is_empty());
    }

    #[test]
    fn test_no_seed_for_simple_question() {
        let line = "What is the weather today?";
        let mut gs = empty_goal();
        seed_goals_if_multi_step(line, &mut gs);

        // Single-clause, no action verb → no subgoals, but objective seeded
        assert!(gs.has_active_goal());
        assert!(gs.pending_subgoals.is_empty());
    }

    #[test]
    fn test_no_seed_for_short_input() {
        let line = "hello";
        let mut gs = empty_goal();
        seed_goals_if_multi_step(line, &mut gs);

        assert!(!gs.has_active_goal());
    }

    #[test]
    fn test_no_seed_when_already_has_goal() {
        let line = "Scan files and extract TODOs";
        let mut gs = GoalState::new("existing goal".to_string());
        seed_goals_if_multi_step(line, &mut gs);

        assert_eq!(gs.active_objective, Some("existing goal".to_string()));
        assert!(gs.pending_subgoals.is_empty());
    }

    #[test]
    fn test_splits_on_and_conjunction() {
        let line = "Write a script to scan files, and then generate a report";
        let clauses = split_into_clauses(line);
        assert!(clauses.len() >= 2);
    }

    #[test]
    fn test_splits_on_numbered_list() {
        let line = "1. Scan all files\n2. Extract TODOs\n3. Generate report";
        let clauses = split_into_clauses(line);
        assert!(clauses.len() >= 2);
    }

    #[test]
    fn test_exact_prompt_01_pattern() {
        let line = "Write a shell-based automation that scans the workspace for source files (e.g., .py, .js, .sh), summarizes counts by type, extracts TODO/FIXME comments, and identifies files larger than 1MB. Write all outputs (a compact summary report and a concise log) to a `project_tmp` directory. Ensure idempotency, clear status messages, and graceful error handling.";
        let mut gs = empty_goal();
        seed_goals_if_multi_step(line, &mut gs);

        assert!(gs.has_active_goal());
        assert!(gs.pending_subgoals.len() >= 2);
    }

    #[test]
    fn test_strip_request_frame() {
        assert_eq!(strip_request_frame("Can you list all files"), "list all files");
        assert_eq!(strip_request_frame("I want to find bugs"), "find bugs");
        assert_eq!(strip_request_frame("Please check the logs"), "check the logs");
    }
}
