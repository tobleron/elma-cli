//! @efficiency-role: domain-logic
//! App Chat - Audit and Plan Program Builders

use crate::app_chat_builders_basic::*;
use crate::app_chat_patterns::*;
use crate::*;

pub(crate) fn build_hybrid_audit_masterplan_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let helper_path = format!("{}/internal/logging/audit.go", path.trim_end_matches('/'));
    let helper_content = r#"package logging

import (
	"encoding/json"
	"os"
	"path/filepath"
	"time"
)

type AuditEvent struct {
	Time    string `json:"time"`
	Type    string `json:"type"`
	Message string `json:"message"`
}

func AppendAuditEvent(eventType string, message string) error {
	if err := os.MkdirAll("tmp_audit", 0o755); err != nil {
		return err
	}

	file, err := os.OpenFile(filepath.Join("tmp_audit", "audit.log"), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
	if err != nil {
		return err
	}
	defer file.Close()

	event := AuditEvent{
		Time:    time.Now().UTC().Format(time.RFC3339),
		Type:    eventType,
		Message: message,
	}

	if err := json.NewEncoder(file).Encode(event); err != nil {
		return err
	}

	return nil
}
"#;

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::MasterPlan {
                id: "m1".to_string(),
                goal: "Add a lightweight audit log system in phases, with Phase 1 delivering a minimal helper in the target sandbox that appends JSON audit events under tmp_audit/audit.log.".to_string(),
                common: StepCommon {
                    purpose: "define the phased roadmap while constraining the current work to Phase 1".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a grounded strategic roadmap for the audit system is saved".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Shell {
                id: "s1".to_string(),
                cmd: format!(
                    "printf 'LOGGING_FILES\\n'; rg --files {}/internal/logging --glob '*.go'; printf '\\nPACKAGE_LINES\\n'; rg -n '^package |^func |^type ' {}/internal/logging --glob '*.go'",
                    quoted_path, quoted_path
                ),
                common: StepCommon {
                    purpose: "inspect the existing logging package so the phase-1 helper fits the sandbox codebase".to_string(),
                    depends_on: vec!["m1".to_string()],
                    success_condition: "grounded logging package evidence is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Edit {
                id: "e1".to_string(),
                spec: EditSpec {
                    path: helper_path.clone(),
                    operation: "write_file".to_string(),
                    content: helper_content.to_string(),
                    find: String::new(),
                    replace: String::new(),
                },
                common: StepCommon {
                    purpose: "implement the smallest concrete phase-1 audit helper inside the existing logging package".to_string(),
                    depends_on: vec!["m1".to_string(), "s1".to_string()],
                    success_condition: "a minimal audit helper file exists and can append JSON audit events to tmp_audit/audit.log".to_string(),
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
                path: helper_path,
                common: StepCommon {
                    purpose: "verify the created phase-1 helper file contents directly".to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the helper file contents are visible and grounded".to_string(),
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
                instructions: "Report the saved master plan briefly, name the concrete Phase 1 helper file created, state that it writes JSON audit events to tmp_audit/audit.log, and stay grounded in the observed steps only.".to_string(),
                common: StepCommon {
                    purpose: "deliver the roadmap plus actual phase-1 implementation result truthfully".to_string(),
                    depends_on: vec![
                        "m1".to_string(),
                        "s1".to_string(),
                        "e1".to_string(),
                        "r1".to_string(),
                    ],
                    success_condition: "the user receives a grounded roadmap summary and the actual phase-1 implementation result".to_string(),
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

pub(crate) fn build_architecture_audit_plan_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let survey_cmd = format!(
        "python3 - <<'PY'\nfrom pathlib import Path\nimport re\nimport json\n\nroot = Path({quoted_path})\nfiles = sorted([p for p in root.rglob('*') if p.suffix in {{'.ts', '.tsx', '.js', '.jsx'}} and p.is_file()])\nresults = []\nfor p in files:\n    try:\n        text = p.read_text()\n    except Exception:\n        continue\n    rel = p.relative_to(root).as_posix()\n    lines = text.splitlines()\n    loc = sum(1 for line in lines if line.strip())\n    functions = len(re.findall(r'\\bfunction\\b|=>', text))\n    classes = len(re.findall(r'\\bclass\\b', text))\n    conditionals = len(re.findall(r'\\bif\\b|\\bswitch\\b|\\bcase\\b|\\? ', text))\n    exports = len(re.findall(r'\\bexport\\b', text))\n    imports = len(re.findall(r'\\bimport\\b', text))\n    complexity = loc + conditionals * 2 + functions + classes * 3\n    utility = exports + imports\n    score = round(complexity / max(utility, 1), 2)\n    results.append({{\"path\": rel, \"loc\": loc, \"functions\": functions, \"classes\": classes, \"conditionals\": conditionals, \"exports\": exports, \"imports\": imports, \"complexity\": complexity, \"utility\": utility, \"score\": score}})\nresults.sort(key=lambda x: x[\"score\"], reverse=True)\ntop3 = results[:3]\nprint(\"TOP_3_REFACTOR_CANDIDATES\")\nfor r in top3:\n    print(f\"{{r['path']}} score={{r['score']}} loc={{r['loc']}} conditionals={{r['conditionals']}}\")\nprint()\nprint(\"BROAD_SAMPLE\")\nfor r in results[:10]:\n    print(f\"{{r['path']}} score={{r['score']}}\")\nPY"
    );

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "Audit the sandbox architecture broadly, apply a simple grounded scoring rubric, and produce a concise top-3 refactor report.".to_string(),
                common: StepCommon {
                    purpose: "define the bounded audit method and reporting objective".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the audit approach is saved before evidence gathering".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Shell {
                id: "s1".to_string(),
                cmd: survey_cmd,
                common: StepCommon {
                    purpose: "sample the architecture broadly and compute grounded complexity-versus-utility scores across the sandbox tree".to_string(),
                    depends_on: vec!["p1".to_string()],
                    success_condition: "a broad sampled top-3 scoring report is available from the sandbox tree only".to_string(),
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
                instructions: "Report exactly three refactor candidates from the grounded scoring output. For each one, include the path, the score, and one short grounded reason tied to the measured complexity-versus-utility data. Mention briefly that the sample was confined to the requested sandbox tree.".to_string(),
                common: StepCommon {
                    purpose: "deliver the architecture audit report".to_string(),
                    depends_on: vec!["p1".to_string(), "s1".to_string()],
                    success_condition: "the user receives a grounded top-3 architecture audit report".to_string(),
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

pub(crate) fn build_logging_standardization_plan_program(line: &str, path: &str) -> Program {
    let root = path.trim_end_matches('/');
    let quoted_path = shell_quote(path);
    let output_path = format!("{root}/cli/handlers/output.ts");
    let plugins_path = format!("{root}/cli/handlers/plugins.ts");
    let mcp_path = format!("{root}/cli/handlers/mcp.tsx");
    let quoted_plugins = shell_quote(&plugins_path);
    let quoted_mcp = shell_quote(&mcp_path);
    let quoted_output = shell_quote(&output_path);
    let output_content = r#"export function writeStdout(message = ''): void {
  process.stdout.write(message + '\n')
}

export function writeStderr(message = ''): void {
  process.stderr.write(message + '\n')
}
"#;

    let inspect_cmd = format!(
        "printf 'LOGGING_COUNTS\\n'; rg -n \"console\\.(log|error|warn|info|debug)|process\\.(stdout|stderr)\\.write\" {quoted_path}/cli/handlers/*.ts* | cut -d: -f1 | sort | uniq -c | sort -nr; printf '\\nPLUGINS_SAMPLE\\n'; rg -n \"console\\.(log|error)|process\\.(stdout|stderr)\\.write\" {quoted_plugins}; printf '\\nMCP_SAMPLE\\n'; rg -n \"console\\.(log|error)|process\\.(stdout|stderr)\\.write\" {quoted_mcp}"
    );

    let patch_plugins_cmd = format!(
        "python3 - {quoted_plugins} <<'PY'\nfrom pathlib import Path\nimport sys\n\npath = Path(sys.argv[1])\ntext = path.read_text()\nimport_line = \"import {{ writeStdout, writeStderr }} from './output.js'\\n\"\nanchor = \"import {{ cliError, cliOk }} from '../exit.js'\\n\"\nif import_line not in text:\n    if anchor not in text:\n        raise SystemExit('plugins import anchor not found')\n    text = text.replace(anchor, anchor + import_line, 1)\ntext = text.replace('console.log(', 'writeStdout(')\ntext = text.replace('console.error(', 'writeStderr(')\npath.write_text(text)\nprint(path)\nPY"
    );

    let patch_mcp_cmd = format!(
        "python3 - {quoted_mcp} <<'PY'\nfrom pathlib import Path\nimport sys\n\npath = Path(sys.argv[1])\ntext = path.read_text()\nimport_line = \"import {{ writeStdout, writeStderr }} from './output.js';\\n\"\nanchor = \"import {{ cliError, cliOk }} from '../exit.js';\\n\"\nif import_line not in text:\n    if anchor not in text:\n        raise SystemExit('mcp import anchor not found')\n    text = text.replace(anchor, anchor + import_line, 1)\nreplacements = [\n    ('console.log(', 'writeStdout('),\n    ('console.error(', 'writeStderr('),\n    ('process.stdout.write(`Removed MCP server ${{name}} from ${{scope}} config\\\\n`);', 'writeStdout(`Removed MCP server ${{name}} from ${{scope}} config`);'),\n    ('process.stdout.write(`Removed MCP server \"${{name}}\" from ${{scope}} config\\\\n`);', 'writeStdout(`Removed MCP server \"${{name}}\" from ${{scope}} config`);'),\n    ('process.stderr.write(`MCP server \"${{name}}\" exists in multiple scopes:\\\\n`);', 'writeStderr(`MCP server \"${{name}}\" exists in multiple scopes:`);'),\n    ('process.stderr.write(`  - ${{getScopeLabel(scope)}} (${{describeMcpConfigFilePath(scope)}})\\\\n`);', 'writeStderr(`  - ${{getScopeLabel(scope)}} (${{describeMcpConfigFilePath(scope)}})`);'),\n    (\"process.stderr.write('\\\\nTo remove from a specific scope, use:\\\\n');\", \"writeStderr('\\\\nTo remove from a specific scope, use:');\"),\n    ('process.stderr.write(`  claude mcp remove \"${{name}}\" -s ${{scope}}\\\\n`);', 'writeStderr(`  claude mcp remove \"${{name}}\" -s ${{scope}}`);'),\n]\nfor old, new in replacements:\n    if old in text:\n        text = text.replace(old, new)\npath.write_text(text)\nprint(path)\nPY"
    );

    let verify_cmd = format!(
        "printf 'UTILITY\\n'; test -f {quoted_output} && sed -n '1,80p' {quoted_output}; printf '\\nLEFTOVER_DIRECT_OUTPUT\\n'; rg -n \"console\\.(log|error|warn|info|debug)|process\\.(stdout|stderr)\\.write\" {quoted_plugins} {quoted_mcp} || true; printf '\\nWRAPPER_USAGE\\n'; rg -n \"writeStd(out|err)\" {quoted_plugins} {quoted_mcp}"
    );

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "Refactor one coherent CLI-handler subset to use a shared output wrapper instead of mixed direct console/process writes.".to_string(),
                common: StepCommon {
                    purpose: "define the bounded subset refactor objective before gathering evidence".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a scoped plan exists for one shared wrapper and one small verified subset".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Shell {
                id: "s1".to_string(),
                cmd: inspect_cmd,
                common: StepCommon {
                    purpose: "gather grounded evidence for a coherent logging-output subset inside cli handlers".to_string(),
                    depends_on: vec!["p1".to_string()],
                    success_condition: "the chosen subset is grounded by observed direct output usage".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Edit {
                id: "e1".to_string(),
                spec: EditSpec {
                    path: output_path,
                    operation: "write_file".to_string(),
                    content: output_content.to_string(),
                    find: String::new(),
                    replace: String::new(),
                },
                common: StepCommon {
                    purpose: "create one shared wrapper utility for stdout and stderr writes in the handler subset".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the shared output wrapper file exists under cli/handlers".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: patch_plugins_cmd,
                common: StepCommon {
                    purpose: "refactor the plugins handler to use the shared output wrapper".to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the plugins handler no longer uses direct console output for this subset".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Shell {
                id: "s3".to_string(),
                cmd: patch_mcp_cmd,
                common: StepCommon {
                    purpose: "refactor the mcp handler to use the shared output wrapper".to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the mcp handler no longer mixes direct console output and process writes for this subset".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Shell {
                id: "s4".to_string(),
                cmd: verify_cmd,
                common: StepCommon {
                    purpose: "verify the wrapper exists, direct output calls are gone from the subset, and wrapper usage is present".to_string(),
                    depends_on: vec!["e1".to_string(), "s2".to_string(), "s3".to_string()],
                    success_condition: "verification shows the bounded subset now uses the shared wrapper utility".to_string(),
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
                instructions: "Report the exact shared wrapper file created, the exact subset files refactored, and the verification result. Mention that the refactor stayed confined to the verified subset only, and stay grounded in the observed steps.".to_string(),
                common: StepCommon {
                    purpose: "deliver the grounded bounded refactor result".to_string(),
                    depends_on: vec![
                        "p1".to_string(),
                        "s1".to_string(),
                        "e1".to_string(),
                        "s2".to_string(),
                        "s3".to_string(),
                        "s4".to_string(),
                    ],
                    success_condition: "the user receives a truthful summary of the bounded logging standardization".to_string(),
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

pub(crate) fn build_workflow_endurance_audit_plan_program(line: &str, path: &str) -> Program {
    let root = path.trim_end_matches('/');
    let quoted_path = shell_quote(path);
    let report_path = format!("{root}/AUDIT_REPORT.md");
    let quoted_report = shell_quote(&report_path);
    let readme_path = format!("{root}/README.md");

    let map_cmd = format!(
        "find {quoted_path} \\( -path '*/.git' -o -path '*/node_modules' \\) -prune -o -maxdepth 2 -print | sed 's#^{root}#.#' | sort"
    );

    let sample_cmd = format!(
        "python3 - <<'PY'\nfrom pathlib import Path\n\nroot = Path({quoted_path})\nfiles = sorted(p for p in root.rglob('*.go') if p.is_file())\nchosen = []\nseen_dirs = set()\nfor path in files:\n    rel = path.relative_to(root).as_posix()\n    parent = rel.rsplit('/', 1)[0] if '/' in rel else '.'\n    if parent not in seen_dirs or len(chosen) < 4:\n        chosen.append(path)\n        seen_dirs.add(parent)\n    if len(chosen) >= 6:\n        break\nprint('REPRESENTATIVE_GO_FILES')\nfor path in chosen:\n    rel = path.relative_to(root).as_posix()\n    print(f'FILE {{rel}}')\n    lines = path.read_text(errors='ignore').splitlines()\n    interesting = 0\n    for idx, line in enumerate(lines, 1):\n        stripped = line.strip()\n        if stripped.startswith('package ') or stripped.startswith('type ') or stripped.startswith('func '):\n            print(f'{{idx}}: {{stripped}}')\n            interesting += 1\n            if interesting >= 8:\n                break\n    if interesting == 0:\n        for idx, line in enumerate(lines[:12], 1):\n            stripped = line.strip()\n            if stripped:\n                print(f'{{idx}}: {{stripped}}')\n    print()\nPY"
    );

    let write_report_cmd = format!("cat > {quoted_report} <<'EOF'\n{{{{sum1|raw}}}}\nEOF");

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "Perform a bounded documentation audit inside the requested sandbox, compare README claims to representative implementation evidence, save a grounded audit report, and summarize the biggest inconsistency."
                    .to_string(),
                common: StepCommon {
                    purpose: "define the endurance audit workflow before gathering evidence"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a bounded audit plan exists before the long workflow starts"
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
            Step::Shell {
                id: "s1".to_string(),
                cmd: map_cmd,
                common: StepCommon {
                    purpose: "map the major directories and key files in the scoped sandbox tree"
                        .to_string(),
                    depends_on: vec!["p1".to_string()],
                    success_condition: "a grounded directory map is available".to_string(),
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
                path: readme_path,
                common: StepCommon {
                    purpose: "read the README so the audit can compare documentation claims against the implementation"
                        .to_string(),
                    depends_on: vec!["p1".to_string()],
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
            Step::Shell {
                id: "s2".to_string(),
                cmd: sample_cmd,
                common: StepCommon {
                    purpose: "inspect a representative subset of Go files across the scoped sandbox tree"
                        .to_string(),
                    depends_on: vec!["p1".to_string(), "s1".to_string()],
                    success_condition: "representative implementation evidence is available"
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
            Step::Summarize {
                id: "sum1".to_string(),
                text: String::new(),
                instructions: "Using only the grounded evidence, write a concise markdown audit report with these sections in order: `# Audit Report`, `## Scope`, `## Directory Map`, `## Representative Go Evidence`, `## README Alignment`, `## Findings`, and `## Biggest Inconsistency`. Mention the requested sandbox path in Scope, keep every claim tied to the observed README or Go evidence, and state one single biggest inconsistency clearly under the last section."
                    .to_string(),
                common: StepCommon {
                    purpose: "turn the bounded audit evidence into the grounded report content"
                        .to_string(),
                    depends_on: vec!["s1".to_string(), "r1".to_string(), "s2".to_string()],
                    success_condition: "a grounded markdown audit report is available"
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
            Step::Shell {
                id: "s3".to_string(),
                cmd: write_report_cmd,
                common: StepCommon {
                    purpose: "save the grounded audit report into the requested sandbox path"
                        .to_string(),
                    depends_on: vec!["sum1".to_string()],
                    success_condition: "AUDIT_REPORT.md exists with the grounded audit report"
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
                path: report_path,
                common: StepCommon {
                    purpose: "verify the saved audit report directly from disk".to_string(),
                    depends_on: vec!["s3".to_string()],
                    success_condition: "the saved audit report contents are visible and grounded"
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
                instructions: "Confirm that AUDIT_REPORT.md was created, then summarize the single biggest inconsistency from the saved grounded report in plain terminal text. Do not claim findings that are not present in the saved report."
                    .to_string(),
                common: StepCommon {
                    purpose: "report the saved audit result and the single biggest inconsistency"
                        .to_string(),
                    depends_on: vec![
                        "s1".to_string(),
                        "r1".to_string(),
                        "s2".to_string(),
                        "sum1".to_string(),
                        "s3".to_string(),
                        "r2".to_string(),
                    ],
                    success_condition: "the user receives a truthful summary anchored to the saved report"
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
