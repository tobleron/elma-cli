#!/usr/bin/env bash
#
# DSL Protocol Self-Test Harness (Task 365)
#
# Runs a suite of DSL action prompts against a sandbox workspace, captures
# sessions/transcripts, classifies failures, and generates a report.
#
# Usage:
#   $0 [--dry-run] [--manual] [--workspace PATH] [--prompts-dir PATH] [--report FILE] [--verbose]
#
# Environment:
#   ELMA_BIN           - Path to elma-cli binary (default: elma-cli)
#   ELMA_SELF_TEST_BASE_URL - Base URL for LLM provider (default: http://localhost:8080)
#   ELMA_SELF_TEST_MODEL - Model name to use (default: test-model)
#   ELMA_DRY_RUN       - If set, same as --dry-run
#
set -euo pipefail

# ---------- Configuration ----------
DEFAULT_WORKSPACE="_stress_testing/dsl_protocol_lab"
DEFAULT_PROMPTS_DIR="tests/dsl/prompts"
DEFAULT_REPORT_FILE="dsl_protocol_report.md"
DEFAULT_SESSIONS_SUBDIR="sessions"

WORKSPACE="${ELMA_TEST_WORKSPACE:-$DEFAULT_WORKSPACE}"
PROMPTS_DIR="${ELMA_TEST_PROMPTS_DIR:-$DEFAULT_PROMPTS_DIR}"
REPORT_FILE="${ELMA_TEST_REPORT:-$DEFAULT_REPORT_FILE}"
SESSIONS_SUBDIR="${ELMA_TEST_SESSIONS_SUBDIR:-$DEFAULT_SESSIONS_SUBDIR}"

DRY_RUN="${ELMA_DRY_RUN:-false}"
MANUAL_MODE=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --manual)  MANUAL_MODE=true; shift ;;
    --workspace) WORKSPACE="$2"; shift 2 ;;
    --prompts-dir) PROMPTS_DIR="$2"; shift 2 ;;
    --report) REPORT_FILE="$2"; shift 2 ;;
    --verbose) VERBOSE=true; shift ;;
    -h|--help)
      echo "Usage: $0 [--dry-run] [--manual] [--workspace PATH] [--prompts-dir PATH] [--report FILE] [--verbose]"
      exit 0
      ;;
    *)
      echo "ERROR: Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# Resolve to absolute paths relative to repo root
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  REPO_ROOT=$(git rev-parse --show-toplevel)
else
  REPO_ROOT=$(pwd)
fi

WORKSPACE_ABS="$REPO_ROOT/$WORKSPACE"
PROMPTS_DIR_ABS="$REPO_ROOT/$PROMPTS_DIR"
REPORT_ABS="$REPO_ROOT/$REPORT_FILE"
SESSIONS_ROOT="$WORKSPACE_ABS/$SESSIONS_SUBDIR"

# Elma CLI binary
ELMA_BIN="${ELMA_BIN:-elma-cli}"

# Ensure workspace exists
if [[ ! -d "$WORKSPACE_ABS" ]]; then
  echo "ERROR: Workspace directory not found: $WORKSPACE_ABS" >&2
  exit 1
fi

# Ensure prompts dir exists and has files
shopt -s nullglob
prompt_files=($(ls -1 "$PROMPTS_DIR_ABS"/*.md 2>/dev/null | sort))
shopt -u nullglob
if [[ ${#prompt_files[@]} -eq 0 ]]; then
  echo "ERROR: No prompt files (*.md) found in $PROMPTS_DIR_ABS" >&2
  exit 1
fi

# Create sessions root if needed
mkdir -p "$SESSIONS_ROOT"

# Detect existing sessions to identify new ones per run
existing_sessions=()
if [[ -d "$SESSIONS_ROOT" ]]; then
  mapfile -t existing_sessions < <(ls -1 "$SESSIONS_ROOT" 2>/dev/null || true)
fi

# Initialize report
cat > "$REPORT_ABS" <<'EOF'
# DSL Protocol Self-Test Report

**Generated:** <generated timestamp>
**Workspace:** <workspace path>
**Prompts dir:** <prompts dir>
**Mode:** <mode>

## Summary

| # | Prompt | Status | Session | Duration (s) | Failure Class |
|---|---|---|---|---|---|
EOF

# Replace placeholders with actual values using in-place edit (sed -i but we'll just truncate and rewrite later). We'll just append rows and then post-process.
# Actually simpler: generate the entire report incrementally, then replace placeholders at end.

# Write header with actual meta
# We'll create the file with proper header, then append table rows later.
: > "$REPORT_ABS"
cat >> "$REPORT_ABS" <<EOF
# DSL Protocol Self-Test Report

**Generated:** $(date)
**Workspace:** $WORKSPACE_ABS
**Prompts dir:** $PROMPTS_DIR_ABS
**Mode:** $(if $DRY_RUN; then echo dry-run; elif $MANUAL_MODE; then echo manual; else echo auto; fi)

## Summary

| # | Prompt | Status | Session | Duration (s) | Failure Class |
|---|---|---|---|---|---|
EOF

total=${#prompt_files[@]}
passed=0
failed=0
idx=1

# Function: classify failure based on transcript and exit code
classify_failure() {
  local transcript="$1"
  local exit_code="$2"
  local class="unknown"

  if [[ ! -f "$transcript" ]]; then
    echo "transcript_missing"
    return
  fi

  # Read transcript content (first 200 lines should be enough)
  local content
  content=$(head -n 200 "$transcript")

  # Check for specific error patterns
  if echo "$content" | grep -qiE 'unknown command|unrecognized command|invalid command'; then
    class="invalid_dsl"
  elif echo "$content" | grep -qiE 'parse error|syntax error|dsl error|validation failed'; then
    class="invalid_dsl"
  elif echo "$content" | grep -qiE 'executor not found|no executor|not implemented|unimplemented'; then
    class="action_not_executed"
  elif echo "$content" | grep -qiE 'permission denied|safe mode|approval required|non-interactive denied'; then
    class="permission_failure"
  elif echo "$content" | grep -qiE 'stale|file changed|external modification'; then
    class="stale_evidence"
  elif echo "$content" | grep -qiE 'timeout|context overflow|too large|size limit'; then
    class="executor_failure"
  elif echo "$content" | grep -qiE 'loop|stagnation|retry exhausted|max retries'; then
    class="loop_stagnation"
  elif echo "$content" | grep -qiE 'session persist|transcript persist|write failed'; then
    class="persistence_failure"
  elif [[ $exit_code -ne 0 ]]; then
    class="executor_failure"
  else
    # If exit code 0 but transcript suggests lack of evidence
    if echo "$content" | grep -qiE 'i don.t have enough|insufficient evidence|cannot determine|unable to find'; then
      class="missing_evidence"
    else
      class="final_answer_unsupported"
    fi
  fi

  echo "$class"
}

# Loop over prompts
for prompt_file in "${prompt_files[@]}"; do
  prompt_name=$(basename "$prompt_file")
  echo "--------------------------------------------------"
  echo "Running [$idx/$total]: $prompt_name"

  prompt_content=$(<"$prompt_file")
  start=$(date +%s)

  status=""
  session_id=""
  transcript_path=""
  failure_class=""

  if $DRY_RUN; then
    echo "[DRY-RUN] Command: echo \"$prompt_content\" | $ELMA_BIN --sessions-root $SESSIONS_SUBDIR"
    status="SKIPPED"
    duration=0
  elif $MANUAL_MODE; then
    echo "[MANUAL] Run the following:"
    echo "  cd $WORKSPACE_ABS"
    echo "  $ELMA_BIN --sessions-root $SESSIONS_SUBDIR"
    echo "Then paste:"
    echo "---"
    echo "$prompt_content"
    echo "---"
    status="MANUAL"
    duration=0
  else
    # Auto: run non-interactive
    log_file="/tmp/elma_test_${idx}_$(date +%s).log"
    set +e
    echo "$prompt_content" | "$ELMA_BIN" --sessions-root "$SESSIONS_SUBDIR" 2>&1 | tee "$log_file"
    exit_code=${PIPESTATUS[0]}
    set -e

    # Determine new session ID
    current_sessions=()
    if [[ -d "$SESSIONS_ROOT" ]]; then
      mapfile -t current_sessions < <(ls -1 "$SESSIONS_ROOT" 2>/dev/null || true)
    fi

    new_session_id=""
    for sid in "${current_sessions[@]}"; do
      if ! printf '%s\n' "${existing_sessions[@]}" | grep -qx "$sid"; then
        new_session_id="$sid"
        break
      fi
    done

    if [[ -z "$new_session_id" ]]; then
      status="FAIL (no session)"
      transcript_path=""
      failure_class="session_persistence_failure"
      ((failed++))
    else
      session_id="$new_session_id"
      transcript_path="$SESSIONS_ROOT/$session_id/session.md"
      if [[ ! -f "$transcript_path" ]]; then
        status="FAIL (no transcript)"
        failure_class="transcript_persistence_failure"
        ((failed++))
      else
        if [[ $exit_code -ne 0 ]]; then
          status="FAIL (exit $exit_code)"
          failure_class=$(classify_failure "$transcript_path" "$exit_code")
          ((failed++))
        else
          status="PASS"
          ((passed++))
          failure_class=""
        fi
      fi
      existing_sessions+=("$new_session_id")
    fi
    # Keep logs for debugging if verbose
    $VERBOSE && echo "Log: $log_file"
  fi

  duration=$(( $(date +%s) - start ))

  # Append table row
  printf '| %d | %s | %s | %s | %d | %s |\n' \
    "$idx" "$prompt_name" "$status" "${session_id:-N/A}" "$duration" "$failure_class" \
    >> "$REPORT_ABS"

  ((idx++))
done

# Append final summary
cat >> "$REPORT_ABS" <<EOF

## Detailed Results

- **Total prompts:** $total
- **Passed:** $passed
- **Failed:** $failed
EOF

if [[ $failed -gt 0 ]]; then
  echo "" >> "$REPORT_ABS"
  echo "### Failure Classes Count" >> "$REPORT_ABS"
  echo "" >> "$REPORT_ABS"
  echo '| Failure Class | Count |' >> "$REPORT_ABS"
  echo '|---|---|' >> "$REPORT_ABS"
  # Aggregate counts from the table we built. Extract 6th column (skip header lines starting with | but include data). Use tail -n +4 to skip header + separator? Let's do a quick awk:
  awk -F'|' 'NR>3 {gsub(/^[ \t]+|[ \t]+$/, "", $6); if($6!="") count[$6]++} END {for(c in count) printf "| %s | %d |\n", c, count[c]}' "$REPORT_ABS" >> "$REPORT_ABS"
fi

cat >> "$REPORT_ABS" <<EOF

---
*Report generated by $(basename "$0") on $(date)*
EOF

echo ""
echo "============================================"
echo "DSL Protocol Self-Test Complete"
echo "  Total:  $total"
echo "  Passed: $passed"
echo "  Failed: $failed"
echo "  Report: $REPORT_ABS"
echo "============================================"

# Exit with appropriate code
if $DRY_RUN || $MANUAL_MODE; then
  exit 0
fi

if [[ $failed -gt 0 ]]; then
  exit 1
fi
exit 0
