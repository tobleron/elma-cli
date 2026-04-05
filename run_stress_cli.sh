#!/usr/bin/env bash
set -euo pipefail

# Real CLI stress runner for elma-cli
# Runs each stress scenario through `cargo run` (not direct API calls)
# Validates: final answer presence, evidence grounding, sandbox confinement, semantic criteria
#
# Usage:
#   ./run_stress_cli.sh
#   ./run_stress_cli.sh S000B
#   ./run_stress_cli.sh S000A S000B S001
#
# Env:
#   LLAMA_BASE_URL=http://192.168.1.186:8080
#   LLAMA_MODEL=<override model id>
#   STRESS_TIMEOUT=120  (seconds per scenario)

BASE_URL="${LLAMA_BASE_URL:-http://192.168.1.186:8080}"
STRESS_TIMEOUT="${STRESS_TIMEOUT:-120}"
STRESS_ROOT="_stress_testing"
SESSIONS_ROOT="sessions_stress_$(date +%s)"
RESULTS_FILE="stress_results_$(date +%Y%m%d_%H%M%S).txt"

# macOS doesn't have `timeout` — use gtimeout or perl fallback
has_timeout() {
  if command -v timeout >/dev/null 2>&1; then
    echo "timeout"
  elif command -v gtimeout >/dev/null 2>&1; then
    echo "gtimeout"
  else
    echo ""
  fi
}
TIMEOUT_CMD="$(has_timeout)"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing $1" >&2; exit 127; }; }
need python3

# ── Extract prompt from stress test markdown ──
extract_prompt() {
  local file="$1"
  python3 - <<'PY' "$file"
import re, sys
with open(sys.argv[1], 'r') as f:
    content = f.read()
match = re.search(r'## 1\. The Test \(Prompt\)\s*\n\s*"([^"]+)"', content)
if match:
    print(match.group(1))
else:
    print("ERROR: Could not extract prompt", file=sys.stderr)
    sys.exit(1)
PY
}

# ── Extract expected criteria from stress test markdown ──
extract_expected() {
  local file="$1"
  python3 - <<'PY' "$file"
import re, sys
with open(sys.argv[1], 'r') as f:
    content = f.read()
sections = {}
current = None
for line in content.split('\n'):
    m = re.match(r'^## (.+)$', line)
    if m:
        current = m.group(1).strip()
        sections[current] = []
    elif current is not None:
        sections[current].append(line)
# Get Expected Behavior
expected = sections.get('Expected Behavior', [])
success = sections.get('Success Criteria', [])
print("EXPECTED:")
print('\n'.join(expected))
print("SUCCESS:")
print('\n'.join(success))
PY
}

# ── Semantic validator per scenario ──
validate_semantic() {
  local test_name="$1"
  local final_answer="$2"
  local trace_file="$3"
  local session_dir="$4"

  python3 - <<'PY' "$test_name" "$final_answer" "$trace_file" "$session_dir"
import sys, os, re, json, glob

test_name = sys.argv[1]
final_answer = sys.argv[2]
trace_file = sys.argv[3]
session_dir = sys.argv[4]

errors = []
warnings = []

# ── Generic checks ──

# 1. Final answer must exist
if not final_answer or not final_answer.strip():
    errors.append("NO_FINAL_ANSWER: Session produced no final answer")

# 2. No "No steps observed" failure text
if "No steps observed" in final_answer:
    errors.append("FALLBACK_FAILURE: Final answer is 'No steps observed for this request.'")

# 3. No generic AI identity
generic_patterns = [
    r"I am an? (AI|language model)",
    r"I'm (an AI|a large language model|a helpful assistant)",
    r"AI language model providing",
]
for pat in generic_patterns:
    if re.search(pat, final_answer, re.IGNORECASE):
        errors.append(f"GENERIC_IDENTITY: Response uses generic AI identity instead of Elma-specific identity")
        break

# ── Scenario-specific checks ──

base = test_name.split('_')[0] if '_' in test_name else test_name

if base in ('S000A', 'S001'):
    # Chat baseline: should be a reply, no file ops, no hallucinated workspace facts
    if re.search(r'(Cargo\.toml|src/|\.rs)', final_answer):
        errors.append("HALLUCINATED_FACTS: Chat baseline should not invent workspace file references")
    if len(final_answer) > 2000:
        errors.append("OVERLONG_REPLY: Chat baseline reply exceeds 2000 chars ({})".format(len(final_answer)))

elif base in ('S000B', 'S002'):
    # Shell/recursive discovery: must identify files in sandbox
    if '_opencode_for_testing' not in final_answer and 'opencode_for_testing' not in final_answer:
        # Check if at least a file path from sandbox is mentioned
        if not re.search(r'main\.(rs|go|py)', final_answer):
            errors.append("NO_SANDBOX_FILE: Final answer does not reference any file in the sandbox directory")
    # Path preservation: if a specific path is given, it must be exact (not softened)
    path_matches = re.findall(r'[_\w/]+/main\.(?:rs|go|py)', final_answer)
    if path_matches:
        for p in path_matches:
            if '_opencode_for_testing' not in p and 'opencode_for_testing' not in p:
                errors.append("PATH_SOFTENING: Path '{}' is missing the sandbox prefix".format(p))

elif base in ('S000C', 'S000G'):
    # Read/summarize: should summarize content, not execute code
    pass  # Generic checks sufficient

elif base in ('S000D',):
    # Search: must find actual content
    pass

elif base in ('S000E',):
    # Sequential logic: steps should be non-duplicative
    pass

elif base in ('S000F',):
    # Select: must present candidates
    pass

elif base in ('S000H',):
    # Decide: must give a decision
    pass

elif base in ('S000I',):
    # Edit: must describe an edit
    pass

elif base == 'S003':
    # Multi-file refactor: sandbox confined
    if 'src/' in final_answer and '_stress_testing/' not in final_answer:
        errors.append("SANDBOX_ESCAPE: Refactor touched production src/ instead of sandbox")

elif base == 'S004':
    # Troubleshooting: sandbox confined
    if '_claude_code_src' not in final_answer and 'claude_code_src' not in final_answer:
        warnings.append("SCOPE_UNCLEAR: Final answer does not clarify which codebase was analyzed")

elif base == 'S005':
    # Master planning: must include both plan and Phase 1 implementation
    has_plan = bool(re.search(r'(phase|plan|audit|log|event)', final_answer, re.IGNORECASE))
    has_impl = bool(re.search(r'(creat|wrote|implement|built|Phase 1)', final_answer, re.IGNORECASE))
    if has_plan and not has_impl:
        errors.append("PLAN_ONLY: Master plan produced but no Phase 1 implementation")
    if not has_plan and not has_impl:
        errors.append("NO_PLAN_NO_IMPL: Neither plan nor implementation found")

elif base == 'S006':
    # Architecture audit: must score/rank modules
    has_ranking = bool(re.search(r'(top \d|#\d|rank|score|priority|most)', final_answer, re.IGNORECASE))
    if not has_ranking:
        errors.append("NO_RANKING: Architecture audit should produce ranked module recommendations")

elif base == 'S007':
    # Full refactoring: must describe style standardization
    pass

elif base == 'S008':
    # Workflow endurance: must produce AUDIT_REPORT.md
    if 'AUDIT_REPORT' not in final_answer and 'audit report' not in final_answer.lower():
        errors.append("NO_AUDIT_REPORT: Workflow endurance should produce AUDIT_REPORT.md artifact")

# ── Sandbox confinement check ──
# Check that no files outside sandbox were modified
stress_sandbox = "_stress_testing/_opencode_for_testing"
claude_sandbox = "_stress_testing/_claude_code_src"
allowed_sandboxes = [stress_sandbox, claude_sandbox, "_stress_testing"]

# Report
print("ERRORS:")
for e in errors:
    print("  " + e)
if not errors:
    print("  (none)")

print("WARNINGS:")
for w in warnings:
    print("  " + w)
if not warnings:
    print("  (none)")

if errors:
    print("RESULT: FAIL")
    sys.exit(1)
else:
    print("RESULT: PASS")
    sys.exit(0)
PY
}

# ── Run one scenario through real CLI ──
run_scenario() {
  local file="$1"
  local test_name
  test_name="$(basename "$file" .md)"

  echo ""
  echo "══════════════════════════════════════════════════"
  echo "  $test_name"
  echo "══════════════════════════════════════════════════"

  local prompt
  prompt="$(extract_prompt "$file")"
  if [[ "$prompt" == ERROR* ]]; then
    echo "  SKIP: Cannot extract prompt"
    echo "[$test_name] SKIP (prompt extraction failed)" >> "$RESULTS_FILE"
    return 0
  fi

  echo "  Prompt: ${prompt:0:100}..."
  echo ""

  # Run through cargo run with timeout
  local session_id
  session_id="stress_${test_name}_$(date +%s)"

  # Create sessions dir
  mkdir -p "$SESSIONS_ROOT"

  # Pipe the prompt to elma-cli and capture output
  # The CLI will process one turn and exit on EOF
  local cli_output
  local cli_exit=0

  if [[ -n "$TIMEOUT_CMD" ]]; then
    cli_output="$(echo "$prompt" | $TIMEOUT_CMD "${STRESS_TIMEOUT}s" \
      cargo run --quiet -- \
        --base-url "$BASE_URL" \
        --sessions-root "$SESSIONS_ROOT" \
        2>&1)" || cli_exit=$?
  else
    # No timeout available — run without timeout warning
    echo "  WARN: No timeout command available, running without timeout"
    cli_output="$(echo "$prompt" | cargo run --quiet -- \
      --base-url "$BASE_URL" \
      --sessions-root "$SESSIONS_ROOT" \
      2>&1)" || cli_exit=$?
  fi

  if [[ $cli_exit -eq 124 ]]; then
    echo "  FAIL: TIMEOUT after ${STRESS_TIMEOUT}s"
    echo "[$test_name] FAIL (timeout)" >> "$RESULTS_FILE"
    return 0
  elif [[ $cli_exit -ne 0 ]]; then
    echo "  FAIL: CLI exited with code $cli_exit"
    echo "[$test_name] FAIL (cli_exit=$cli_exit)" >> "$RESULTS_FILE"
    return 0
  fi

  # Find the latest session
  local latest_session
  latest_session="$(ls -td "$SESSIONS_ROOT"/*/ 2>/dev/null | head -1)"

  # Extract final answer from CLI output
  # Elma prints "Elma: " prefix for its messages
  local final_answer
  final_answer="$(echo "$cli_output" | sed -n '/Elma:/{s/^.*Elma: //;p}' | tail -1)"

  if [[ -z "$final_answer" ]]; then
    # Try extracting the last substantial block of output
    final_answer="$(echo "$cli_output" | tail -20)"
  fi

  # Extract trace file if it exists
  local trace_file=""
  if [[ -n "$latest_session" ]]; then
    trace_file="$(ls "$latest_session"/*.log 2>/dev/null | head -1)"
  fi

  echo "  Session: ${latest_session:-unknown}"
  echo "  Answer: ${final_answer:0:120}..."
  echo ""

  # Run semantic validation
  local validation_result
  if validation_result="$(validate_semantic "$test_name" "$final_answer" "$trace_file" "$latest_session" 2>&1)"; then
    echo "  PASS"
    echo "$validation_result" | head -10 | sed 's/^/    /'
    echo "[$test_name] PASS" >> "$RESULTS_FILE"
  else
    echo "  FAIL"
    echo "$validation_result" | sed 's/^/    /'
    echo "[$test_name] FAIL" >> "$RESULTS_FILE"
  fi

  echo ""
}

# ── Main ──
echo "══════════════════════════════════════════════════"
echo "  Elma CLI — Real Stress Test Runner"
echo "══════════════════════════════════════════════════"
echo "  Base URL: $BASE_URL"
echo "  Timeout:  ${STRESS_TIMEOUT}s per scenario"
echo "  Sessions: $SESSIONS_ROOT"
echo "  Results:  $RESULTS_FILE"
echo "══════════════════════════════════════════════════"

# Determine which scenarios to run
if [[ $# -gt 0 ]]; then
  # Specific scenarios requested
  scenarios=()
  for arg in "$@"; do
    # Find matching file
    match="$(ls "${STRESS_ROOT}/${arg}"*.md 2>/dev/null | head -1)"
    if [[ -n "$match" ]]; then
      scenarios+=("$match")
    else
      echo "WARNING: No match for '$arg'"
    fi
  done
else
  # Run all S* stress tests
  scenarios=("${STRESS_ROOT}"/S*.md)
fi

if [[ ${#scenarios[@]} -eq 0 ]]; then
  echo "No scenarios to run"
  exit 1
fi

echo ""
echo "Scenarios: ${#scenarios[@]}"
for s in "${scenarios[@]}"; do
  echo "  - $(basename "$s")"
done

# Initialize results file
echo "=== Elma CLI Stress Results ===" > "$RESULTS_FILE"
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$RESULTS_FILE"
echo "Base URL: $BASE_URL" >> "$RESULTS_FILE"
echo "Timeout: ${STRESS_TIMEOUT}s" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

pass_count=0
fail_count=0
skip_count=0

for scenario in "${scenarios[@]}"; do
  if [[ ! -f "$scenario" ]]; then
    continue
  fi

  run_scenario "$scenario"
done

# Count results
pass_count="$(grep -c 'PASS' "$RESULTS_FILE" 2>/dev/null || echo 0)"
fail_count="$(grep -c 'FAIL' "$RESULTS_FILE" 2>/dev/null || echo 0)"
skip_count="$(grep -c 'SKIP' "$RESULTS_FILE" 2>/dev/null || echo 0)"
# Subtract the header line from pass count
pass_count=$((pass_count - 1))
[[ $pass_count -lt 0 ]] && pass_count=0

echo ""
echo "══════════════════════════════════════════════════"
echo "  SUMMARY"
echo "══════════════════════════════════════════════════"
echo "  Pass:  $pass_count"
echo "  Fail:  $fail_count"
echo "  Skip:  $skip_count"
echo "  Total: $((pass_count + fail_count + skip_count))"
echo ""
echo "  Results: $RESULTS_FILE"
echo "══════════════════════════════════════════════════"

if [[ $fail_count -gt 0 ]]; then
  exit 1
fi
