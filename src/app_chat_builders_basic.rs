//! @efficiency-role: domain-logic
//! App Chat - Basic Program Builders

use crate::app_chat_patterns::*;
use crate::*;

pub(crate) fn build_edit_path_probe_program(line: &str, path: &str) -> Program {
    let (section_title, section_line) = derive_append_section_from_request(line);
    let append_content = format!("\n\n## {section_title}\n\n{section_line}\n");

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Read {
                id: "r1".to_string(),
                path: Some(path.to_string()),
                paths: None,
                common: StepCommon {
                    purpose: "read the target file before making the requested append edit"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the target file contents are available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Edit {
                id: "e1".to_string(),
                spec: EditSpec {
                    path: path.to_string(),
                    operation: "append_text".to_string(),
                    content: append_content,
                    find: String::new(),
                    replace: String::new(),
                },
                common: StepCommon {
                    purpose: "append the requested section to the end of the target file"
                        .to_string(),
                    depends_on: vec!["r1".to_string()],
                    success_condition: "the requested section is appended exactly once"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Read {
                id: "r2".to_string(),
                path: Some(path.to_string()),
                paths: None,
                common: StepCommon {
                    purpose: "verify the file now includes the appended audit section"
                        .to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the appended section is visible in the file contents"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Reply {
                id: "r3".to_string(),
                instructions:
                    "Confirm the edit briefly, mention the target file path, and stay grounded in the verified file contents."
                        .to_string(),
                common: StepCommon {
                    purpose: "report the successful edit to the user".to_string(),
                    depends_on: vec!["r1".to_string(), "e1".to_string(), "r2".to_string()],
                    success_condition: "the user receives a grounded confirmation of the edit"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
        ],
    }
}

pub(crate) fn build_scoped_list_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: format!("ls -1 {} | head -n 80", quoted_path),
                common: StepCommon {
                    purpose: "list the scoped path contents concisely from grounded filesystem evidence"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a concise grounded listing of the scoped path is available"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "Return a concise plain-text listing of the observed items only. Do not add commentary before the list. If the listing was truncated, say that briefly after the list."
                    .to_string(),
                common: StepCommon {
                    purpose: "present the concise grounded listing to the user".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the user receives a concise grounded listing"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
        ],
    }
}

pub(crate) fn build_readme_summary_and_entry_point_program(line: &str, path: &str) -> Program {
    let root = path.trim_end_matches('/');
    let quoted_path = shell_quote(path);
    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: format!("ls -1 {}", quoted_path),
                common: StepCommon {
                    purpose: "list the scoped files and directories before reading the README and identifying the entry point".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the top-level scoped listing is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Read {
                id: "r1".to_string(),
                path: Some(format!("{root}/README.md")),
                paths: None,
                common: StepCommon {
                    purpose: "read the scoped README so the repo purpose can be summarized from grounded evidence".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the README contents are available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Summarize {
                id: "sum1".to_string(),
                text: String::new(),
                instructions: "Create exactly 2 concise bullet points that explain what this repo is for. Keep both bullets grounded only in the README contents."
                    .to_string(),
                common: StepCommon {
                    purpose: "compress the README into the requested two grounded bullets".to_string(),
                    depends_on: vec!["r1".to_string()],
                    success_condition: "an exact 2-bullet grounded README summary is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: format!(
                    "rg --files {} | rg '(^|/)(main\\.(go|rs|py|ts|js)|Cargo\\.toml|package\\.json|cmd/root\\.go)$'",
                    quoted_path
                ),
                common: StepCommon {
                    purpose: "gather grounded entry-point candidate files from the scoped workspace".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "grounded entry-point candidate file paths are available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Select {
                id: "sel1".to_string(),
                instructions: "From the grounded file-path evidence, choose exactly one most likely primary entry point for the codebase. Prefer the top-level executable entry file over secondary command wiring. Return the exact relative path only."
                    .to_string(),
                common: StepCommon {
                    purpose: "select the strongest grounded primary entry-point candidate".to_string(),
                    depends_on: vec!["s2".to_string()],
                    success_condition: "one grounded relative path is selected as the primary entry point".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Reply {
                id: "r2".to_string(),
                instructions: "Return exactly two bullet points from the grounded README summary first. Then add one final line that starts with `Entry point:` followed by the selected exact relative path. Preserve exact grounded relative file paths from the evidence and do not mention files that were not observed."
                    .to_string(),
                common: StepCommon {
                    purpose: "present the grounded README summary and exact entry-point path together".to_string(),
                    depends_on: vec![
                        "s1".to_string(),
                        "r1".to_string(),
                        "sum1".to_string(),
                        "s2".to_string(),
                        "sel1".to_string(),
                    ],
                    success_condition: "the user receives exactly two grounded bullets plus the exact grounded entry-point path".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
        ],
    }
}

pub(crate) fn build_decide_path_probe_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let lower = line.to_ascii_lowercase();
    let is_db_storage_decision = lower.contains("database")
        || lower.contains("schema")
        || lower.contains("state")
        || lower.contains("stored")
        || lower.contains("persist");

    if is_db_storage_decision {
        let root = path.trim_end_matches('/');
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!(
                        "printf 'FILES\\n'; rg --files {} | rg '(^|/)(sqlc\\.yaml|internal/db/|internal/session/|internal/config/)' | head -n 160",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "gather concrete database-related file evidence from the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "grounded database-related file paths are available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                        is_read_only: false,
                        is_destructive: true,
                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                    },
                },
                Step::Read {
                    id: "r_sqlc".to_string(),
                    path: Some(format!("{root}/sqlc.yaml")),
                    paths: None,
                    common: StepCommon {
                        purpose: "read the sqlc configuration to see whether a database schema directory is configured"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "the sqlc configuration is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                        is_read_only: true,
                        is_destructive: false,
                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                    },
                },
                Step::Read {
                    id: "r_connect".to_string(),
                    path: Some(format!("{root}/internal/db/connect.go")),
                    paths: None,
                    common: StepCommon {
                        purpose: "read the database connection code to verify whether the project opens a real database and where it stores it"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "the database connection code is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                        is_read_only: true,
                        is_destructive: false,
                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                    },
                },
                Step::Read {
                    id: "r_migration".to_string(),
                    path: Some(format!("{root}/internal/db/migrations/20250424200609_initial.sql")),
                    paths: None,
                    common: StepCommon {
                        purpose: "read one concrete migration file to verify that the schema is defined in SQL migrations"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "a concrete schema migration file is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                        is_read_only: true,
                        is_destructive: false,
                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                    },
                },
                Step::Decide {
                    id: "d1".to_string(),
                    prompt: "Using only the observed evidence, decide whether the project uses a real database. If yes, identify the schema location precisely. If not, identify where state is stored. Prefer the strongest direct evidence from configuration, connection code, and schema files."
                        .to_string(),
                    common: StepCommon {
                        purpose: "make the requested storage decision from directly read evidence"
                            .to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "r_sqlc".to_string(),
                            "r_connect".to_string(),
                            "r_migration".to_string(),
                        ],
                        success_condition: "the storage decision is grounded in directly read evidence"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                        is_read_only: false,
                        is_destructive: true,
                        is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Answer directly. If the project uses a database, say which database is used and identify the schema location precisely, preferring the configured migration directory and one concrete migration file as support. If it does not use a database, identify where state is stored from the observed evidence."
                        .to_string(),
                    common: StepCommon {
                        purpose: "present the grounded storage answer to the user".to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "r_sqlc".to_string(),
                            "r_connect".to_string(),
                            "r_migration".to_string(),
                            "d1".to_string(),
                        ],
                        success_condition: "the user receives a direct grounded storage answer"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                        is_read_only: true,
                        is_destructive: false,
                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                    },
                },
            ],
        };
    }

    let evidence_cmd = format!("rg --files {} | head -n 160", quoted_path);

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: evidence_cmd,
                common: StepCommon {
                    purpose: "gather concrete workspace evidence from the target path"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "grounded file and content evidence is available"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Decide {
                id: "d1".to_string(),
                prompt: format!(
                    "Using only the observed workspace evidence, answer this request: {}. If the evidence is insufficient, say that plainly instead of guessing.",
                    line
                ),
                common: StepCommon {
                    purpose: "make the requested judgment from grounded evidence".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the decision is grounded in the observed evidence"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "Answer the user's question explicitly and ground it in the observed evidence. If the evidence was insufficient, say that plainly and mention the strongest observed clue."
                    .to_string(),
                common: StepCommon {
                    purpose: "present the grounded decision to the user".to_string(),
                    depends_on: vec!["s1".to_string(), "d1".to_string()],
                    success_condition: "the user receives a grounded decision with concise support"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
        ],
    }
}
