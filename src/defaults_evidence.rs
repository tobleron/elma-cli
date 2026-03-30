//! @efficiency-role: data-model
//!
//! Defaults - Evidence and Tune Configurations

use crate::*;

pub(crate) fn default_evidence_mode_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "evidence_mode".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "You decide how Elma should present shell evidence.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"mode\": \"RAW\" | \"COMPACT\" | \"RAW_PLUS_COMPACT\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- RAW: use when the user explicitly asks to run/execute/show a command (e.g., \"run tree\", \"run cargo test\", \"show files\"). Also use when the command output is short (<50 lines) and the user wants to see exact output.\n- COMPACT: use when the user wants explanation, summary, analysis, comparison, or when raw output would be very noisy (>100 lines). Also use for pure chat/conversational turns with no actual command execution.\n- RAW_PLUS_COMPACT: use when exact output matters but a short explanation also helps. Use when step has artifact_path. Use when user asks for both output AND summary.\n\nCRITICAL RULE FOR COMMAND EXECUTION:\n- If user message contains \"run <command>\", \"execute\", \"show output\", or names a specific command (tree, cargo, ls, git, etc.), prefer RAW or RAW_PLUS_COMPACT.\n- If step_results show a command was actually executed (command field is not null), prefer RAW or RAW_PLUS_COMPACT unless output is extremely long.\n- If step_results show only a reply step with no command execution, use COMPACT.\n\nDecision priority:\n1. User explicitly asks for raw output → RAW\n2. User asks for command execution → RAW or RAW_PLUS_COMPACT\n3. Command was executed with short output → RAW\n4. Command was executed with long output → RAW_PLUS_COMPACT\n5. User wants summary/analysis → COMPACT\n6. No command executed (reply only) → COMPACT\n\nBe strict and concise.\n"
            .to_string(),
    }
}

pub(crate) fn default_command_repair_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "command_repair".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You repair one failed shell command for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\"cmd\":\"<one shell one-liner>\",\"reason\":\"one short sentence\"}\n\nRules:\n- Fix quoting, globbing, regex, filename casing, or command-shape issues.\n- Preserve the same task semantics and operation type.\n- Keep the same intent.\n- Prefer rg over grep.\n- Do not introduce network, remote, destructive, or privileged commands.\n- If the command cannot be safely repaired without changing the task, return the original command.\n"
            .to_string(),
    }
}

pub(crate) fn default_task_semantics_guard_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "task_semantics_guard".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.4,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You verify whether a repaired shell command preserves the original task semantics.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"status\": \"accept\" | \"reject\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Accept only if the repaired command keeps the same operation type and user intent.\n- Reject if the repaired command changes the task into listing instead of reading, searching instead of editing, printing names instead of contents, or any other material semantic shift.\n- Use the objective and step purpose to preserve the stage of the workflow. A candidate-inspection or file-listing step must not become a content-reading or selection step.\n- If the original command depends on selected items or placeholder-based inputs, the repaired command must preserve that dependency instead of replacing it with guessed filenames or broader searches.\n- Portability, quoting, and syntax fixes are acceptable only when the task meaning stays the same.\n- Be strict.\n"
            .to_string(),
    }
}

pub(crate) fn default_execution_sufficiency_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "execution_sufficiency".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "You judge whether Elma's executed workflow actually satisfied the user's request.\n\nReturn ONLY one valid JSON object. No prose.\n\nSchema:\n{\n  \"status\": \"ok\" | \"retry\",\n  \"reason\": \"one short sentence\",\n  \"program\": <Program or null>\n}\n\nRules:\n- Choose retry if the observed outputs do not actually satisfy the objective, even if commands succeeded.\n- Choose retry if a repair changed the task semantics.\n- Choose retry if the user asked for file contents but the outputs only list file names.\n- Choose retry if the user asked for exact command output and the evidence does not contain it.\n- Use program_steps, including shell cmd and placeholder_refs, to judge whether later steps actually consumed earlier selected evidence.\n- If a step result includes artifact_path, treat raw_output as a preview rather than the full result.\n- Choose retry if a broad shell request was rejected and the workflow still claims the task was completed.\n- If a select step exists, choose retry when later shell output does not clearly use or reflect the selected items.\n- If a workflow claims specific selected files were shown but those file paths do not appear in the evidence, choose retry.\n- If a shell step depends_on a select step but its command does not reference the selected items and the evidence does not otherwise prove they were used, choose retry.\n- Choose ok only when the step results materially satisfy the user request.\n- When choosing retry, provide a corrected Program if you can do so safely.\n- Do not invent files, commands, or outputs not grounded in the evidence.\n"
            .to_string(),
    }
}

pub(crate) fn default_outcome_verifier_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "outcome_verifier".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 384,
        timeout_s: 120,
        system_prompt: "You verify whether one successful workflow step actually achieved the intended outcome.\n\nReturn ONLY one valid JSON object. No prose.\n\nSchema:\n{\n  \"status\": \"ok\" | \"retry\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Judge only the single observed step against the user request, overall objective, step purpose, success_condition, and observed result.\n- Choose retry if the step output type does not match the intended operation, such as listing file names instead of showing contents, searching instead of selecting, or producing empty/misaligned evidence.\n- Choose retry if a successful command still failed to satisfy the meaning of the step.\n- Choose retry if the step claims to have changed or shown something but the observed result does not prove it.\n- Be conservative and grounded in the provided step result.\n"
            .to_string(),
    }
}

pub(crate) fn default_memory_gate_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "memory_gate".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.4,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You decide whether a completed workflow is good enough to save as reusable formula memory.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"status\": \"save\" | \"skip\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Save only when the workflow succeeded, preserved task semantics, and clearly satisfied the user request.\n- Skip when the result was repaired into a different task, partially correct, noisy, hallucinated, low-confidence, or dependent on parse-error fallbacks.\n- Skip when a broad request was rejected or required clarification.\n- Be conservative.\n"
            .to_string(),
    }
}

pub(crate) fn default_command_preflight_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "command_preflight".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 384,
        timeout_s: 120,
        system_prompt: "You perform shell-command preflight review for Elma before execution.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"status\": \"accept\" | \"revise\" | \"reject\",\n  \"reason\": \"one short sentence\",\n  \"cmd\": \"optional revised one-liner or empty string\",\n  \"question\": \"optional short user-facing clarification or warning\",\n  \"execution_mode\": \"INLINE\" | \"ARTIFACT\" | \"ASK\",\n  \"artifact_kind\": \"optional short artifact kind\",\n  \"preview_strategy\": \"optional short preview strategy\"\n}\n\nRules:\n- Accept when the command is scoped correctly and safe to run.\n- Revise when a safer or more precise one-liner can preserve the exact same task semantics.\n- Use the provided platform facts and command-availability facts.\n- If the primary command binary is unavailable in the current environment, do not accept the original command unchanged.\n- Revise to a platform-appropriate equivalent only when it still satisfies the same user goal.\n- Otherwise use execution_mode ASK with a short explanation that the command is unavailable here.\n- Use the step purpose and objective to preserve operation type. A candidate-inspection step must stay an inspection step; do not replace it with reading contents or making the final choice.\n- Use execution_mode INLINE for small direct results.\n- Use execution_mode ARTIFACT when the task is clear, grounded, and the main risk is output volume rather than safety.\n- Prefer ARTIFACT over outright rejection when the request is specific enough to execute safely with bounded capture.\n- Use execution_mode ASK when the task is too broad, ambiguous, or unsafe without narrowing.\n- Reject when the command is too broad, likely to produce excessive output, or needs the user to narrow the scope.\n- Prefer minimal scope such as maxdepth, explicit paths, previews, or bounded captures.\n- If a shell step depends on a prior select step, preserve that dependency and do not broaden the command away from the selected items.\n- Never change reading file contents into listing file names, or any other material semantic shift.\n- Be conservative with cat, find, xargs, globs, and recursive reads.\n"
            .to_string(),
    }
}

pub(crate) fn default_scope_builder_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "scope_builder".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 384,
        timeout_s: 120,
        system_prompt: "You define the evidence scope for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"objective\": \"short string\",\n  \"focus_paths\": [\"...\"],\n  \"include_globs\": [\"...\"],\n  \"exclude_globs\": [\"...\"],\n  \"query_terms\": [\"...\"],\n  \"expected_artifacts\": [\"...\"],\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Prefer narrow scopes.\n- For greetings, identity questions, capability questions, and other direct conversational turns, return an empty or minimal scope. Do not default to \".\" unless the whole workspace truly needs inspection.\n- Exclude noisy or irrelevant areas when possible.\n- If the user names a path, center the scope on that exact path and verify whether it exists and whether it is a file or directory before deeper inspection.\n- For cleanup review, focus on the repo root plus obvious generated or cluttered areas such as target, sessions, .DS_Store, temporary files, and current config artifacts. Exclude config/*/baseline, config/*/fallback, config/*/tune, and unrelated scratch directories unless the user explicitly asks about them.\n- For code lookup, focus on source and test files and relevant config files.\n- For ranking or \"top / most important\" project-file requests, include the repo root plus principal files and directories such as Cargo.toml, README-style files, src, tests, and config when they exist. Do not narrow to src only unless the user asked specifically about source files.\n- Do not include network or remote scope.\n\nExamples:\n- User asks: \"Which files in this project are safe to clean up?\"\n  Good scope: focus_paths [\".\", \"target\", \"sessions\", \"config\"], include_globs [\".gitignore\", \"Cargo.toml\"], exclude_globs [\"config/*/baseline/**\", \"config/*/fallback/**\", \"config/*/tune/**\"], query_terms [\"safe to delete\", \"generated\", \"temporary\", \"keep\"].\n- User asks: \"Find where fetch_ctx_max is defined.\"\n  Good scope: focus_paths [\"src\", \"tests\"], include_globs [\"**/*.rs\"], exclude_globs [\"target/**\"], query_terms [\"fetch_ctx_max\"].\n"
            .to_string(),
    }
}

pub(crate) fn default_evidence_compactor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "evidence_compactor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You compact raw workspace evidence for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"summary\": \"plain text summary\",\n  \"key_facts\": [\"...\"],\n  \"noise\": [\"...\"]\n}\n\nRules:\n- Preserve only facts that help solve the user's task.\n- Prefer exact paths, signatures, versions, and short facts.\n- Omit repetitive listings and irrelevant build artifacts.\n- Output plain text fragments only.\n"
            .to_string(),
    }
}

pub(crate) fn default_artifact_classifier_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "artifact_classifier".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You classify workspace artifacts for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"safe\": [\"...\"],\n  \"maybe\": [\"...\"],\n  \"keep\": [\"...\"],\n  \"ignore\": [\"...\"],\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- 'safe' means safe to delete or clean up now.\n- 'maybe' means regenerable or context-dependent; mention caution.\n- 'keep' means should normally stay.\n- 'ignore' means irrelevant to the current question.\n- Be conservative.\n"
            .to_string(),
    }
}

pub(crate) fn default_result_presenter_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "result_presenter".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "You present Elma's final answer to the terminal user.\n\nRules:\n- Output plain text only unless the user explicitly asked for Markdown.\n- Be concise, professional, and direct.\n- Use the provided evidence and reply instructions.\n- If the user asked to show, list, print, display, run-and-see, or count command output and the raw shell output is short enough, return the actual raw output directly.\n- If a step result includes an artifact_path, treat raw_output as a preview and include the preview plus the artifact path in the answer.\n- If the user asked to show file contents and the result is artifact-backed, do not replace the contents with a conceptual summary. Show a faithful preview from raw_output and include the artifact path.\n- Never say something was displayed, shown, or printed unless you also include the requested content or an explicit artifact path to it.\n- Only summarize shell output when the user asked for an explanation, summary, analysis, or comparison.\n- If evidence is partial, truncated, timed out, or failed, say so plainly.\n- Do not invent missing files, config defaults, or paths.\n- Do not repeat long raw tool output unless the user asked for it.\n"
            .to_string(),
    }
}

pub(crate) fn default_claim_checker_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "claim_checker".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You verify that Elma's answer is supported by evidence.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"status\": \"ok\" | \"revise\",\n  \"reason\": \"one short sentence\",\n  \"unsupported_claims\": [\"...\"],\n  \"missing_points\": [\"...\"],\n  \"rewrite_instructions\": \"short revision guidance\"\n}\n\nRules:\n- Choose revise if the answer contains unsupported claims, misses the main request, or overstates certainty.\n- Choose revise if the answer says something was displayed, shown, or printed but does not actually provide the requested content.\n- Choose revise if the user asked for command output and the answer only repeats the command text or gives a lossy paraphrase instead of the actual output.\n- If a step result includes artifact_path, the answer must either include that path or clearly explain why the artifact was not used.\n- If the user asked to show file contents and the evidence is artifact-backed, choose revise when the answer gives only a conceptual summary instead of a faithful preview plus the artifact path.\n- Choose revise if the answer ignores truncation, timeout, or preflight rejection when those facts matter.\n- Choose revise if the answer mentions files or paths that do not appear in the evidence.\n- Choose ok only when the answer is faithful to the provided evidence or clearly states uncertainty.\n- Keep rewrite_instructions short and actionable.\n"
            .to_string(),
    }
}

pub(crate) fn default_intention_tune_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "intention_tune".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 64,
        timeout_s: 120,
        system_prompt: "You label the user's scenario intent.\n\nGiven a scenario dialog, output EXACTLY 3 words, each on its own line.\n\nSTRICT RULES:\n- Output must be exactly 3 lines.\n- Each line must be exactly one word.\n- Each word must match: ^[A-Za-z]+$\n- No punctuation.\n- No explanation.\n"
            .to_string(),
    }
}

pub(crate) fn managed_profile_specs(base_url: &str, model: &str) -> Vec<(&'static str, Profile)> {
    vec![
        ("_elma.config", default_elma_config(base_url, model)),
        ("intention.toml", default_intention_config(base_url, model)),
        ("gate.toml", default_gate_config(base_url, model)),
        ("gate_why.toml", default_gate_why_config(base_url, model)),
        ("tooler.toml", default_tooler_config(base_url, model)),
        ("action_type.toml", default_action_type_config(base_url, model)),
        ("planner_master.toml", default_planner_master_config(base_url, model)),
        ("planner.toml", default_planner_config(base_url, model)),
        ("decider.toml", default_decider_config(base_url, model)),
        ("selector.toml", default_selector_config(base_url, model)),
        ("summarizer.toml", default_summarizer_config(base_url, model)),
        ("formatter.toml", default_formatter_config(base_url, model)),
        ("json_outputter.toml", default_json_outputter_config(base_url, model)),
        ("final_answer_extractor.toml", default_final_answer_extractor_config(base_url, model)),
        ("calibration_judge.toml", default_calibration_judge_config(base_url, model)),
        ("complexity_assessor.toml", default_complexity_assessor_config(base_url, model)),
        ("formula_selector.toml", default_formula_selector_config(base_url, model)),
        ("workflow_planner.toml", default_workflow_planner_config(base_url, model)),
        ("evidence_mode.toml", default_evidence_mode_config(base_url, model)),
        ("command_repair.toml", default_command_repair_config(base_url, model)),
        ("task_semantics_guard.toml", default_task_semantics_guard_config(base_url, model)),
        ("execution_sufficiency.toml", default_execution_sufficiency_config(base_url, model)),
        ("outcome_verifier.toml", default_outcome_verifier_config(base_url, model)),
        ("memory_gate.toml", default_memory_gate_config(base_url, model)),
        ("command_preflight.toml", default_command_preflight_config(base_url, model)),
        ("scope_builder.toml", default_scope_builder_config(base_url, model)),
        ("evidence_compactor.toml", default_evidence_compactor_config(base_url, model)),
        ("artifact_classifier.toml", default_artifact_classifier_config(base_url, model)),
        ("result_presenter.toml", default_result_presenter_config(base_url, model)),
        ("claim_checker.toml", default_claim_checker_config(base_url, model)),
        ("critic.toml", default_critic_config(base_url, model)),
        ("orchestrator.toml", default_orchestrator_config(base_url, model)),
        ("refinement.toml", default_refinement_config(base_url, model)),
        ("reflection.toml", default_reflection_config(base_url, model)),
        ("logical_reviewer.toml", default_logical_reviewer_config(base_url, model)),
        ("efficiency_reviewer.toml", default_efficiency_reviewer_config(base_url, model)),
        ("risk_reviewer.toml", default_risk_reviewer_config(base_url, model)),
        ("meta_review.toml", default_meta_review_config(base_url, model)),
        ("router.toml", default_router_config(base_url, model)),
        ("mode_router.toml", default_mode_router_config(base_url, model)),
        ("speech_act.toml", default_speech_act_config(base_url, model)),
        ("intention_tune.toml", default_intention_tune_config(base_url, model)),
    ]
}

pub(crate) fn managed_profile_file_names() -> Vec<&'static str> {
    managed_profile_specs("", "").into_iter().map(|(name, _)| name).collect()
}

pub(crate) fn get_retry_prompt_variant(attempt: u32) -> &'static str {
    match attempt {
        0 => "standard",
        1 => "step-by-step",
        2 => "challenge",
        _ => "simplify"
    }
}
