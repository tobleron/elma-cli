#!/usr/bin/env bash
#
# Live DSL Runtime Smoke Runner (Task 385)
#
# Runs basic smoke prompts from _testing_prompts/ against the live elma-cli
# TUI, captures session/transcript paths, and classifies results.
#
# Usage:
#   _scripts/live_smoke_runner.sh [--manual] [--dry-run] [--prompts-dir DIR] [--report FILE]
#
# Modes:
#   --manual     (default) Print each prompt, wait for user to run it and enter session ID
#   --dry-run    Print prompts and expected actions without running anything
#
# Environment:
#   ELMA_SESSIONS_ROOT  - Override default sessions directory path
#
# Prompts (from _testing_prompts/):
#   01_chat_greeting.txt           - "hi" → natural chat, no workflow-speak
#   02_list_current_directory.txt   - list dir → visible tool row, evidence
#   03_shell_visibility_smoke.txt   - shell + list → visibility, evidence
#   04_search_and_read_smoke.txt    - search + read → evidence chain

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "$BASH_SOURCE" || echo "$0")")" && pwd 2>/dev/null || pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ---------- Configuration ----------
PROMPTS_DIR="${ELMA_TEST_PROMPTS_DIR:-$REPO_ROOT/_testing_prompts}"
REPORT_FILE="${ELMA_TEST_REPORT:-$REPO_ROOT/_testing_reports/live_smoke_$(date +%Y%m%d_%H%M%S).md}"
SESSIONS_DIR="${ELMA_SESSIONS_ROOT:-$REPO_ROOT/sessions}"
MANUAL_MODE=true
DRY_RUN=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --manual)   MANUAL_MODE=true; shift ;;
    --dry-run)  DRY_RUN=true; shift ;;
    --prompts-dir) PROMPTS_DIR="$2"; shift 2 ;;
    --report)   REPORT_FILE="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,30p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *) echo "ERROR: Unknown option: $1" >&2; exit 1 ;;
  esac
done

# Collect prompts (exclude deferred/ and hidden files)
shopt -s nullglob
prompt_files=()
while IFS= read -r -d '' f; do
  prompt_files+=("$f")
done < <(find "$PROMPTS_DIR" -maxdepth 1 -name '*.txt' -type f -print0 | sort -z)
shopt -u nullglob

if [[ ${#prompt_files[@]} -eq 0 ]]; then
  echo "ERROR: No prompt files found in $PROMPTS_DIR" >&2
  exit 1
fi

total=${#prompt_files[@]}

# ---------- Ensure report directory ----------
mkdir -p "$(dirname "$REPORT_FILE")"

# ---------- Classification ----------
classify_transcript() {
  local session_dir="$1"
  local class="not_captured"

  local transcript_file="$session_dir/artifacts/terminal_transcript.txt"
  [[ -f "$transcript_file" ]] || transcript_file="$session_dir/artifacts/transcript.txt"
  [[ -f "$transcript_file" ]] || { echo "transcript_missing"; return; }

  local content
  content=$(head -n 300 "$transcript_file" 2>/dev/null || true)

  # Check for evidence-grounded final answer (non-chat routes)
  if echo "$content" | grep -qiE 'evidence|files found|found.*matches|directory (contents|listing)'; then
    echo "evidence_grounded"
    return
  fi

  # Check for chat/greeting (natural response, no evidence)
  if echo "$content" | grep -qiE 'hi there|hello|greetings'; then
    echo "chat_natural"
    return
  fi

  # Check for tool visibility rows
  if echo "$content" | grep -qiE 'TOOL TRACE RUNNING|ToolStarted|ToolFinished'; then
    echo "tool_rows_visible"
    return
  fi

  # Check for invalid DSL
  if echo "$content" | grep -qiE 'invalid action|unknown command|parse error|DSL error'; then
    echo "invalid_dsl"
    return
  fi

  echo "uncertain"
}

# ---------- Check session for valid artifacts ----------
check_session_artifacts() {
  local session_dir="$1"
  local result="ok"
  local details=()

  # Check session.json
  [[ -f "$session_dir/session.json" ]] || { result="missing_session_json"; details+=("session.json"); }

  # Check artifacts
  local fa_count=0
  fa_count=$(ls "$session_dir"/artifacts/*final_answer*.txt "$session_dir"/artifacts/*final_answer*.md 2>/dev/null | wc -l | tr -d ' ')

  local has_transcript=false
  [[ -f "$session_dir/artifacts/terminal_transcript.txt" ]] || [[ -f "$session_dir/artifacts/transcript.txt" ]] || has_transcript=true
  if [[ "$has_transcript" == "false" ]]; then
    # Only flag if we're checking — terminal_transcript might not exist for very short sessions
    :
  fi

  if [[ "$fa_count" -eq 0 ]]; then
    [[ "$result" == "ok" ]] && result="missing_final_answer"
    details+=("final_answer")
  fi

  echo "$result"
}

# ---------- Run ----------
echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║  Live DSL Runtime Smoke Runner  (Task 385)               ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "  Prompts dir:  $PROMPTS_DIR"
echo "  Report:       $REPORT_FILE"
echo "  Mode:         $( $DRY_RUN && echo 'dry-run' || echo 'manual' )"
echo "  Prompts:      $total"
echo ""

# ---------- Capture pre-run session list ----------
pre_sessions=()
while IFS= read -r -d '' d; do
  pre_sessions+=("$(basename "$d")")
done < <(find "$SESSIONS_DIR" -maxdepth 1 -type d -name 's_*' -print0 2>/dev/null || true)

# Initialize report
{
  echo "# Live Smoke Report"
  echo ""
  echo "**Generated:** $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "**Mode:** $( $DRY_RUN && echo 'dry-run' || echo 'manual' )"
  echo "**Prompts dir:** $PROMPTS_DIR"
  echo ""
  echo "## Summary"
  echo ""
  echo "| # | Prompt | Status | Session | Artifacts | Failure Class |"
  echo "|---|---|---|---|---|---|"
} > "$REPORT_FILE"

passed=0
failed=0
idx=1

for prompt_file in "${prompt_files[@]}"; do
  prompt_name=$(basename "$prompt_file" .txt)
  prompt_content=$(<"$prompt_file")

  echo "──────────────────────────────────────────────────────────"
  echo "[$idx/$total] $prompt_name"
  echo "──────────────────────────────────────────────────────────"

  if $DRY_RUN; then
    echo "  [DRY-RUN] Would prompt user to run:"
    echo "    cargo run -- --sessions-root $SESSIONS_DIR"
    echo "  With input: \"$prompt_content\""
    echo ""
    status="SKIPPED"
    session_id="N/A"
    artifacts_status="N/A"
    failure_class=""
  else
    # Manual mode: show prompt, wait for user
    echo ""
    echo "  ┌─ Prompt ──────────────────────────────────────────"
    echo "  │  $prompt_content"
    echo "  └────────────────────────────────────────────────────"
    echo ""
    echo "  Run elma-cli with:"
    echo "    cargo run -- --sessions-root $SESSIONS_DIR"
    echo ""
    echo "  Then paste the prompt above. After the session ends,"
    echo "  enter the session ID (e.g., s_1777579706_702762000):"
    echo ""
    read -r -p "  Session ID (or 'skip' to skip, 'q' to quit): " session_id

    if [[ "$session_id" == "q" ]]; then
      echo "  Quitting early."
      break
    fi

    if [[ "$session_id" == "skip" ]]; then
      status="SKIPPED"
      session_id="N/A"
      artifacts_status="N/A"
      failure_class=""
    else
      session_dir="$SESSIONS_DIR/$session_id"
      if [[ -d "$session_dir" ]]; then
        artifacts_status=$(check_session_artifacts "$session_dir")
        failure_class=$(classify_transcript "$session_dir")

        if [[ "$artifacts_status" == "ok" && ( "$failure_class" == "evidence_grounded" || "$failure_class" == "chat_natural" || "$failure_class" == "tool_rows_visible" ) ]]; then
          status="PASS"
          ((passed++))
        else
          status="FAIL"
          ((failed++))
        fi
      else
        status="FAIL (no session dir)"
        artifacts_status="missing"
        failure_class="session_not_found"
        ((failed++))
      fi
    fi
  fi

  # Append report row
  printf '| %d | %s | %s | %s | %s | %s |\n' \
    "$idx" "$prompt_name" "$status" "${session_id:-N/A}" "$artifacts_status" "$failure_class" \
    >> "$REPORT_FILE"

  ((idx++))
done

# Append final summary to report
{
  echo ""
  echo "## Results"
  echo ""
  echo "- **Total prompts:** $((idx - 1))"
  echo "- **Passed:** $passed"
  echo "- **Failed:** $failed"
  echo ""
  if [[ $failed -gt 0 ]]; then
    echo "### Failure Classes"
    echo ""
    echo '| Failure Class | Count |'
    echo '|---|---|'
    awk -F'|' 'NR>3 {gsub(/^[ \t]+|[ \t]+$/, "", $6); if($6!="" && $6!="Failure Class") count[$6]++} END {for(c in count) printf "| %s | %d |\n", c, count[c]}' "$REPORT_FILE" >> "$REPORT_FILE"
  fi
} >> "$REPORT_FILE"

echo ""
echo "============================================"
echo "Live Smoke Run Complete"
echo "  Total:  $((idx - 1))"
echo "  Passed: $passed"
echo "  Failed: $failed"
echo "  Report: $REPORT_FILE"
echo "============================================"

if [[ $failed -gt 0 ]]; then
  exit 1
fi
exit 0
