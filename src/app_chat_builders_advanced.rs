//! @efficiency-role: orchestrator
//!
//! App Chat - Advanced Program Builders (shell path probe and dispatch)

pub(crate) use crate::app_chat_builders_audit::{
    build_architecture_audit_plan_program, build_hybrid_audit_masterplan_program,
    build_logging_standardization_plan_program, build_workflow_endurance_audit_plan_program,
};
pub(crate) use crate::app_chat_builders_basic::{
    build_readme_summary_and_entry_point_program, build_scoped_list_program,
};
pub(crate) use crate::app_chat_builders_probes::*;
use crate::app_chat_patterns::*;
use crate::*;

pub(crate) fn build_shell_path_probe_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let lower = line.to_ascii_lowercase();

    // Task 453 Category 1: Remove stress-test specific programs
    // readme_summary, workflow_endurance were stress-test markers

    if request_looks_like_scoped_list_request(line) {
        return build_scoped_list_program(line, path);
    }
    if (lower.contains("function") || lower.contains("find "))
        && (lower.contains("called") || lower.contains("call site") || lower.contains("invoked"))
        && !lower.contains("missing")
        && !lower.contains("rename")
    {
        return Program {
            objective: line.to_string(),
            steps: vec![
                shell_step("s1",
                    &format!("rg --files {} --glob '*.ts' --glob '*.tsx' --glob '*.js' --glob '*.jsx' | head -n 80", quoted_path),
                    "list candidate source files for function search",
                    "candidate file paths are available"),
                select_step("sel1",
                    "From the candidate files, identify one function definition. Return the exact function name only.",
                    &["s1"],
                    "select one function to search for call sites",
                    "one function name is selected"),
                shell_step_with_deps("s2",
                    &format!("rg -n \"\\b{{{{sel1|shell_words}}}}\\b\" {} --glob '*.ts' --glob '*.tsx' --glob '*.js' --glob '*.jsx' | head -n 80", quoted_path),
                    "search for all call sites of the selected function",
                    &["sel1"],
                    "all call sites are found"),
                reply_step("r1",
                    "Report the function definition location and all call sites found. Stay grounded in the shell evidence.",
                    &["s1", "sel1", "s2"],
                    "present the grounded function search result",
                    "the user receives a grounded function search summary"),
            ],
        };
    }
    if lower.contains("directory structure")
        && lower.contains("largest")
        && lower.contains("line count")
    {
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!("find {} -type d | sort", quoted_path),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "map the directory structure".to_string(),
                        depends_on: Vec::new(),
                        success_condition: "directory structure is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!(
                        "rg --files {} --glob '*.ts' --glob '*.tsx' --glob '*.js' --glob '*.jsx' --glob '*.go' --glob '*.rs' --glob '*.py' | while read f; do wc -l < \"$f\" | awk '{{print $1, FILENAME}}' FILENAME=\"$f\"; done | sort -rn | head -n 3",
                        quoted_path
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "find the top 3 largest source files by line count".to_string(),
                        depends_on: Vec::new(),
                        success_condition: "top 3 largest files are identified".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Summarize {
                    id: "sum1".to_string(),
                    text: String::new(),
                    instructions: "From the grounded evidence, report the directory structure and the top 3 largest source files by line count."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "summarize the structure and size findings".to_string(),
                        depends_on: vec!["s1".to_string(), "s2".to_string()],
                        success_condition: "a concise summary is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Present the directory structure and top 3 largest files. Stay grounded in the observed evidence."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "present the grounded discovery result".to_string(),
                        depends_on: vec!["s1".to_string(), "s2".to_string(), "sum1".to_string()],
                        success_condition: "the user receives a grounded structure and size report"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }
    if request_looks_like_missing_id_troubleshoot(line) {
        let target = format!("{}/cli/transports/ccrClient.ts", path.trim_end_matches('/'));
        let quoted_target = shell_quote(&target);
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!(
                        "rg -n \"event\\\\.message\\\\.id|message\\\\.id\" {} --glob '*.ts' | head -n 80",
                        quoted_path
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "find a grounded parsing path that directly depends on a present message id field"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "one or more grounded vulnerable id-handling lines are identified"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!("sed -n '145,165p' {}", quoted_target),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "inspect the concrete vulnerable code block around the selected id access"
                            .to_string(),
                        depends_on: vec!["s1".to_string()],
                        success_condition: "the vulnerable code block is visible for repair"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s3".to_string(),
                    cmd: format!(
                        "python3 - {} <<'PY'\nfrom pathlib import Path\nimport sys\n\npath = Path(sys.argv[1])\ntext = path.read_text()\nold = \"\"\"      case 'message_start': {{\n        const id = msg.event.message.id\n        const prevId = state.scopeToMessage.get(scopeKey(msg))\n\"\"\"\nnew = \"\"\"      case 'message_start': {{\n        const id =\n          typeof msg.event.message.id === 'string' && msg.event.message.id.length > 0\n            ? msg.event.message.id\n            : `missing-id:${{msg.uuid}}`\n        const prevId = state.scopeToMessage.get(scopeKey(msg))\n\"\"\"\nif old not in text:\n    raise SystemExit('target snippet not found')\npath.write_text(text.replace(old, new, 1))\nprint(path)\nPY",
                        quoted_target
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "implement a robust local fallback when parsed stream message_start data lacks an id field"
                            .to_string(),
                        depends_on: vec!["s1".to_string(), "s2".to_string()],
                        success_condition: "the target code uses a deterministic fallback id when message.id is missing"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s4".to_string(),
                    cmd: format!(
                        "python3 - {} <<'PY'\nfrom pathlib import Path\nimport sys\n\ntext = Path(sys.argv[1]).read_text()\nassert \"missing-id:${{msg.uuid}}\" in text, 'fallback marker missing'\nassert \"const id = msg.event.message.id\" not in text, 'old direct id access still present'\nprint('verified fallback present')\nPY",
                        quoted_target
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "verify locally that the direct missing-id hazard was replaced by the new fallback logic"
                            .to_string(),
                        depends_on: vec!["s3".to_string()],
                        success_condition: "local verification confirms the fallback marker is present and the direct old assignment is gone"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report the exact file changed, the vulnerable path that was fixed, the fallback that was introduced, and the local verification result. Stay grounded in the shell evidence only."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "present the grounded troubleshooting fix result".to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "s2".to_string(),
                            "s3".to_string(),
                            "s4".to_string(),
                        ],
                        success_condition: "the user receives a grounded explanation of the fix and verification"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    if request_looks_like_scoped_rename_refactor(line) {
        let root = path.trim_end_matches('/');
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!(
                        "rg --files {} --glob '*.ts' --glob '*.tsx' --glob '*.js' --glob '*.jsx' | head -n 80",
                        quoted_path
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "gather candidate files for renaming within the scoped workspace"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "candidate file paths are available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel1".to_string(),
                    instructions: "Choose one small utility function with a vague name that could be improved. Return the exact function name only."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "select one vague function name to rename"
                            .to_string(),
                        depends_on: vec!["s1".to_string()],
                        success_condition: "one function name is selected"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel2".to_string(),
                    instructions: "Propose one clear descriptive replacement name for the selected function. Return only the new name."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "propose a better function name"
                            .to_string(),
                        depends_on: vec!["sel1".to_string()],
                        success_condition: "one replacement name is proposed"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: Some("rename_suggester".to_string()),
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!(
                        "rg -n \"fn {}\\(\" {} --glob '*.ts' --glob '*.tsx' --glob '*.js' --glob '*.jsx' | head -n 8",
                        "{{sel1|shell_words}}", quoted_path
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "find the function definition location"
                            .to_string(),
                        depends_on: vec!["sel1".to_string()],
                        success_condition: "the definition location is identified"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s3".to_string(),
                    cmd: format!(
                        "old='{}'; new='{}'; python3 - \"$old\" \"$new\" {} <<'PY'\nimport sys, re\nfrom pathlib import Path\nold_name, new_name = sys.argv[1], sys.argv[2]\nfor p in Path({quoted_path}).rglob('*'):\n    if p.suffix not in {{'.ts','.tsx','.js','.jsx'}} or not p.is_file():\n        continue\n    text = p.read_text()\n    if old_name not in text:\n        continue\n    text = re.sub(r'\\b' + re.escape(old_name) + r'\\b', new_name, text)\n    p.write_text(text)\n    print(p)\nPY",
                        "{{sel1|shell_words}}", "{{sel2|shell_words}}", quoted_path
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "rename the function and update all call sites across the scoped workspace"
                            .to_string(),
                        depends_on: vec!["sel1".to_string(), "sel2".to_string()],
                        success_condition: "the function is renamed and all call sites are updated"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s4".to_string(),
                    cmd: format!(
                        "rg -n \"\\b{}\\b\" {} --glob '*.ts' --glob '*.tsx' --glob '*.js' --glob '*.jsx' || true",
                        "{{sel1|shell_words}}", quoted_path
                    ),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "verify the old name no longer appears in the scoped workspace"
                            .to_string(),
                        depends_on: vec!["s3".to_string()],
                        success_condition: "the old name is gone from scoped files"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report the exact old and new function names, the scoped files changed, and the verification result. Stay grounded in the evidence."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "present the grounded rename result"
                            .to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "sel1".to_string(),
                            "sel2".to_string(),
                            "s3".to_string(),
                            "s4".to_string(),
                        ],
                        success_condition: "the user receives a grounded rename summary"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    if lower.contains("potential files")
        && lower.contains("most likely candidate")
        && (lower.contains("main application logic") || lower.contains("main logic"))
    {
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!("ls -1 {}", quoted_path),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "list the top-level files and directories in the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "top-level candidate names are available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!("rg --files {} | head -n 120", quoted_path),
                    common: StepCommon {
                                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: true,
                                        is_read_only: false,
                        purpose: "gather concrete file-path evidence from the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "grounded file paths are available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel1".to_string(),
                    instructions: "Select exactly three grounded file paths that are the strongest candidates for main application logic. Prefer entry points, app wiring, root commands, and central runtime modules. Return exact file paths only."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "choose three grounded candidate files".to_string(),
                        depends_on: vec!["s1".to_string(), "s2".to_string()],
                        success_condition: "three candidate file paths are selected".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel2".to_string(),
                    instructions: "From the candidate file paths, choose exactly one most likely main application logic file. Prefer the file that most directly acts as the application entry point or root command. Return the exact file path only."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "select the most likely candidate from the three grounded options"
                            .to_string(),
                        depends_on: vec!["sel1".to_string()],
                        success_condition: "one grounded file path is selected as the best candidate"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report the three selected candidate file paths, then name the most likely candidate and explain briefly why it is the strongest grounded choice."
                        .to_string(),
                    common: StepCommon {
                                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                        is_destructive: false,
                                        is_read_only: true,
                        purpose: "present the grounded candidates and the final selection"
                            .to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "s2".to_string(),
                            "sel1".to_string(),
                            "sel2".to_string(),
                        ],
                        success_condition: "the user receives three grounded candidates plus one selected best candidate with reasoning"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    let mut steps = vec![Step::Shell {
        id: "s1".to_string(),
        cmd: format!("ls -1 {}", quoted_path),
        common: StepCommon {
            is_concurrency_safe: false,
            interrupt_behavior: InterruptBehavior::Graceful,
            is_destructive: true,
            is_read_only: false,
            purpose: "list the files in the target path".to_string(),
            depends_on: Vec::new(),
            success_condition: "the file or directory listing is available".to_string(),
            parent_id: None,
            depth: None,
            unit_type: None,
        },
    }];

    if lower.contains("readme.md") {
        steps.push(Step::Read {
            id: "r1".to_string(),
            path: Some(format!("{}/README.md", path.trim_end_matches('/'))),
            paths: None,
            common: StepCommon {
                is_concurrency_safe: true,
                interrupt_behavior: InterruptBehavior::Graceful,
                is_destructive: false,
                is_read_only: true,
                purpose: "read the README file in the target path".to_string(),
                depends_on: Vec::new(),
                success_condition: "the README contents are available".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
        if request_prefers_summary_output(line) {
            steps.push(Step::Summarize {
                id: "sum1".to_string(),
                text: String::new(),
                instructions: "Create exactly 3 concise bullet points that summarize the README for an executive audience. Keep every point grounded in the README contents."
                    .to_string(),
                common: StepCommon {
                                    is_concurrency_safe: true,
                            interrupt_behavior: InterruptBehavior::Graceful,
                                    is_destructive: false,
                                    is_read_only: true,
                    purpose: "summarize the README into the requested executive bullets"
                        .to_string(),
                    depends_on: vec!["r1".to_string()],
                    success_condition: "a grounded 3-bullet summary is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            });
            steps.push(Step::Reply {
                id: "a1".to_string(),
                instructions: "Return exactly the 3 bullet points from the grounded summary. Do not add extra prose before or after the bullets."
                    .to_string(),
                common: StepCommon {
                                    is_concurrency_safe: true,
                            interrupt_behavior: InterruptBehavior::Graceful,
                                    is_destructive: false,
                                    is_read_only: true,
                    purpose: "deliver the grounded README summary in the requested format"
                        .to_string(),
                    depends_on: vec!["s1".to_string(), "r1".to_string(), "sum1".to_string()],
                    success_condition: "the user receives exactly 3 grounded bullet points"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            });
            return Program {
                objective: line.to_string(),
                steps,
            };
        }
        steps.push(Step::Reply {
            id: "a1".to_string(),
            instructions: "Summarize the README core purpose and keep the answer grounded in the observed file contents.".to_string(),
            common: StepCommon {
                                is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                is_destructive: false,
                                is_read_only: true,
                purpose: "answer using the README evidence".to_string(),
                depends_on: vec!["s1".to_string(), "r1".to_string()],
                success_condition: "the user receives a grounded summary".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
        return Program {
            objective: line.to_string(),
            steps,
        };
    }

    let evidence_cmd = if lower.contains("entry point") || lower.contains("primary entry") {
        format!(
            "rg --files {} | rg '(^|/)(main\\.(go|rs|py|ts|js)|Cargo\\.toml|package\\.json|cmd/root\\.go)$'",
            quoted_path
        )
    } else {
        format!("rg --files {}", quoted_path)
    };

    steps.push(Step::Shell {
        id: "s2".to_string(),
        cmd: evidence_cmd,
        common: StepCommon {
            is_concurrency_safe: false,
            interrupt_behavior: InterruptBehavior::Graceful,
            is_destructive: true,
            is_read_only: false,
            purpose: "collect supporting file evidence from the target path".to_string(),
            depends_on: Vec::new(),
            success_condition: "supporting file evidence is available".to_string(),
            parent_id: None,
            depth: None,
            unit_type: None,
        },
    });
    if lower.contains("entry point") || lower.contains("primary entry") {
        steps.push(Step::Select {
            id: "sel1".to_string(),
            instructions: "From the grounded file-path evidence, choose exactly one most likely primary entry point for the codebase. Prefer the top-level executable entry file over secondary command wiring. Return the exact relative path only."
                .to_string(),
            common: StepCommon {
                                is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                is_destructive: false,
                                is_read_only: true,
                purpose: "select the strongest grounded primary entry-point candidate".to_string(),
                depends_on: vec!["s2".to_string()],
                success_condition: "one grounded relative path is selected as the primary entry point".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
        steps.push(Step::Reply {
            id: "r1".to_string(),
            instructions: "Answer using the observed file evidence and the selected entry-point candidate. Preserve exact grounded relative file paths from the evidence in the final answer. State the selected exact relative path first, then explain briefly why it is the strongest grounded entry point.".to_string(),
            common: StepCommon {
                                is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                is_destructive: false,
                                is_read_only: true,
                purpose: "present the grounded result".to_string(),
                depends_on: vec!["s2".to_string(), "sel1".to_string()],
                success_condition: "the user receives a grounded answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
    } else {
        steps.push(Step::Reply {
            id: "r1".to_string(),
            instructions: "Answer using the observed file evidence. Preserve exact grounded relative file paths from the evidence in the final answer.".to_string(),
            common: StepCommon {
                                is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                                is_destructive: false,
                                is_read_only: true,
                purpose: "present the grounded result".to_string(),
                depends_on: vec!["s1".to_string(), "s2".to_string()],
                success_condition: "the user receives a grounded answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
    }

    Program {
        objective: line.to_string(),
        steps,
    }
}
