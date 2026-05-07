//! @efficiency-role: test
//!
//! Session Regression Test Harness — Task 769.
//!
//! Reproduces the latest failure sequence as deterministic unit tests
//! that exercise the governance, context, artifact, and finalization
//! fixes implemented in Tasks 761-768. These tests verify that the
//! behavioral failures from the latest session are caught and prevented.
//!
//! Test sequence covers:
//! 1. Current-turn context separation (761)
//! 2. Per-turn deliverable tracking (762)
//! 3. Objective state and approach supervision (763)
//! 4. Scope coverage (764)
//! 5. Path resolution (765)
//! 6. Finalization hardening (766)
//! 7. Tool loop summaries (767)
//! 8. Relevance expiry (768)

use crate::*;

/// Helper: create a session root directory.
fn session_root() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    (dir, root)
}

// ── Test 1: Current-turn context isolates raw user request ──
#[test]
fn test_regression_current_turn_context_isolates_raw_request() {
    let mut ctx =
        crate::turn_context_packet::CurrentTurnContext::new("root path has no AGENTS.md ?");
    ctx.prior_relevant_evidence =
        Some("Required artifact: _testing_prompts/01_prompt.txt".to_string());
    // The raw request must not contain prior evidence
    let raw = ctx.raw_request();
    assert_eq!(raw, "root path has no AGENTS.md ?");
    assert!(!raw.contains("_testing_prompts"));
    assert!(!raw.contains("Required artifact"));

    // The model message includes the evidence but labeled as prior attempt
    let model_msg = ctx.build_model_message();
    assert!(model_msg.contains("Previously gathered"));
    assert!(model_msg.contains("_testing_prompts"));
    assert!(model_msg.contains("root path has no AGENTS.md"));
}

// ── Test 3: Pre-existing files not reported as created without mutation ──
#[test]
fn test_regression_pre_existing_not_reported_created() {
    let (dir, root) = session_root();
    std::fs::write(root.join("AGENTS.md"), "existing content").unwrap();

    let mut contract = crate::artifact_verifier::DeliverableContract::new("turn_001");
    contract.require("AGENTS.md", "user_request", &root);
    contract.verify_all(&root);

    // Pre-existing file NOT touched this turn should not be "completed this turn"
    assert!(!contract.has_current_turn_completions());
    assert!(
        contract.entries[0].pre_existed,
        "Should record pre-existence"
    );
    // But should be in a valid terminal state
    assert!(contract.all_terminal());
    let _ = dir.close();
}

// ── Test 4: Artifact contract requires turn-scope — stale deliverables don't leak ──
#[test]
fn test_regression_stale_artifact_does_not_leak() {
    let (dir, root) = session_root();
    let mut contract = crate::artifact_verifier::DeliverableContract::new("turn_002");
    contract.require("current_report.md", "user_request", &root);
    std::fs::write(root.join("current_report.md"), "report").unwrap();
    contract.mark_touched("current_report.md");
    contract.verify_all(&root);

    // Only current_request artifacts are tracked
    assert_eq!(contract.required_paths().len(), 1);
    assert!(contract.has_current_turn_completions());

    // Simulate: the answer should NOT reference stale paths
    let answer = "Created or updated: `_testing_prompts/01_prompt.txt`";
    let hardener = crate::finalization_hardener::check_stale_artifact_references(
        answer,
        &contract.required_paths(),
    );
    assert!(
        matches!(
            hardener,
            crate::finalization_hardener::HardenerVerdict::StaleArtifactDetected { .. }
        ),
        "Stale artifact references must be detected"
    );
    let _ = dir.close();
}

// ── Test 5: Finalization hardener catches completion claims with unresolved objectives ──
#[test]
fn test_regression_finalization_hardener_catches_unresolved() {
    let answer = "Completed the requested artifact work.";
    let verdict = crate::finalization_hardener::check_unresolved_objective(
        answer, true, // unresolved objectives
        true, // budget exhausted
    );
    assert!(
        matches!(
            verdict,
            crate::finalization_hardener::HardenerVerdict::PartialCompletion { .. }
        ),
        "Hardener must flag completion claim with unresolved objectives"
    );
}

// ── Test 6: Path resolver can find _tasks/completed from tasks/completed ──
#[test]
fn test_regression_path_resolver_finds_prefixed_path() {
    let (_dir, root) = {
        let d = tempfile::tempdir().unwrap();
        let r = d.path().to_path_buf();
        std::fs::create_dir_all(r.join("_tasks").join("completed")).unwrap();
        std::fs::write(
            r.join("_tasks").join("completed").join("001_done.md"),
            "done",
        )
        .unwrap();
        (d, r)
    };
    let candidates = crate::workspace_path_resolver::resolve_missing_path("tasks/completed", &root);
    let has_match = candidates
        .iter()
        .any(|c| c.resolved.contains("_tasks/completed"));
    assert!(
        has_match,
        "Path resolver should find _tasks/completed from tasks/completed query"
    );
}

// ── Test 7: Scope coverage ledger tracks required items ──
#[test]
fn test_regression_scope_coverage_tracks_prompts() {
    let mut ledger = crate::scope_coverage::ScopeCoverageLedger::new();
    let prompt_files: Vec<String> = (1..=8)
        .map(|i| format!("_testing_prompts/{:02}_prompt.txt", i))
        .collect();
    let refs: Vec<&str> = prompt_files.iter().map(|s| s.as_str()).collect();
    ledger.register_items(
        &refs.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "file",
    );

    // Initially all pending
    assert_eq!(
        ledger.count_by_status(crate::scope_coverage::CoverageStatus::Pending),
        8
    );

    // Mark some covered
    for i in 0..5 {
        ledger.mark_covered(&prompt_files[i]);
    }
    assert_eq!(
        ledger.count_by_status(crate::scope_coverage::CoverageStatus::Covered),
        5
    );
    assert!(ledger.has_pending());

    // Mark all covered
    for i in 5..8 {
        ledger.mark_covered(&prompt_files[i]);
    }
    assert!(ledger.all_terminal());
}

// ── Test 8: Tool loop summary has structured metadata ──
#[test]
fn test_regression_tool_loop_summary_structure() {
    let summary = crate::tool_loop::ToolLoopSummary {
        tool_calls_made: 5,
        tool_call_ids: vec!["c1".to_string(), "c2".to_string()],
        successful_reads: vec!["AGENTS.md".to_string()],
        successful_searches: vec!["src/".to_string()],
        failed_operations: vec![("read".to_string(), "not found".to_string())],
        duplicate_suppressions: 2,
        coverage: Some((2, 5)),
        stop_reason: "iteration_limit".to_string(),
        stop_iteration: 8,
    };
    assert_eq!(summary.tool_calls_made, 5);
    assert_eq!(summary.successful_reads.len(), 1);
    assert_eq!(summary.failed_operations.len(), 1);
    assert_eq!(summary.duplicate_suppressions, 2);
}

// ── Test 9: Relevance filtering expires irrelevant tool output ──
#[test]
fn test_regression_relevance_expires_irrelevant_context() {
    let messages = vec![
        ChatMessage::simple("user", "find AGENTS.md"),
        ChatMessage::simple("assistant", "searching..."),
        ChatMessage {
            role: "tool".to_string(),
            content: "_testing_prompts/01_prompt.txt exists".to_string(),
            name: Some("search".to_string()),
            tool_calls: None,
            tool_call_id: Some("t1".to_string()),
            reasoning_content: None,
            summarized: false,
        },
    ];
    let prior_artifacts = vec!["_tasks/completed".to_string()];
    let filtered = crate::effective_history::compute_effective_history_with_relevance(
        &messages,
        "find AGENTS.md",
        &prior_artifacts,
    );
    // Tool message about _testing_prompts should be expired (not relevant to AGENTS.md,
    // not matching prior artifacts, and not an error)
    assert_eq!(
        filtered.len(),
        2,
        "Irrelevant tool message should be expired"
    );
}

// ── Test 10: Objective state fork creates sibling approach ──
#[test]
fn test_regression_objective_state_fork_on_stagnation() {
    let mut state = crate::objective_state::ObjectiveState::new("find tasks/completed");
    state.stagnation_count = 3;
    assert!(state.needs_approach_fork(3));

    let original = state.active_approach_id.clone();
    let new_id = state.fork_approach("path not resolved after 3 attempts");
    assert_ne!(new_id, original);
    assert_eq!(state.total_approaches, 2);
    assert_eq!(state.stagnation_count, 0);
}
