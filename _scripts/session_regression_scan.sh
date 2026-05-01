#!/usr/bin/env bash
#
# Session Regression Miner & Failure Taxonomy Scanner (Task 391)
#
# Scans all saved Elma sessions and classifies failures into a compact taxonomy.
# Read-only, offline, no model calls.
#
# Usage:
#   _scripts/session_regression_scan.sh [sessions_dir] > report.toml
#
# Default sessions_dir: sessions/
#
# Output: TOML report on stdout. Sections for each session that has failures.
#
# Failure families:
#   invalid_action_dsl     - Model produced DSL that failed parsing (catch-all)
#   provider_markup        - Provider-native <command>...</command> markup instead of DSL
#   bare_action_line       - Bare action letter/word without proper DSL format
#   missing_block_terminator - Thinking/reasoning block without proper closing
#   prose_before_action    - Prose or explanation emitted before action line
#   false_success_wording  - Final answer claims success when errors occurred
#   missing_evidence_final - Final answer acknowledges inability due to missing evidence
#   hidden_retry_or_fallback - Fallback or retry occurred without user visibility
#

set -euo pipefail

# ---------- Configuration ----------
SESSIONS_DIR="${1:-sessions}"
SCRIPT_NAME=$(basename "$0")

# ---------- Helper: emit TOML string value ----------
toml_string() {
  local val="$1"
  # Escape backslashes, double quotes, newlines
  val="${val//\\/\\\\}"
  val="${val//\"/\\\"}"
  # Replace actual newlines with \n escape
  printf '"%s"' "$val"
}

toml_multiline() {
  printf '"""\n%s\n"""' "$1"
}

# ---------- Classification Functions ----------

# Classify a raw= value from tool_loop: invalid DSL parse error
classify_raw_action() {
  local raw="$1"
  local class="invalid_action_dsl"

  # provider_markup: <command>...</command>
  if echo "$raw" | grep -qE '<command>' || echo "$raw" | grep -qE '</command>'; then
    echo "provider_markup"
    return
  fi

  # prose_before_action: markdown fences, prose paragraphs, code blocks
  if echo "$raw" | grep -qE '```' || echo "$raw" | grep -qE '^[A-Za-z].*\n\n'; then
    echo "prose_before_action"
    return
  fi

  # bare_action_line: lone single letter (like just 'X' or 'R') with no args at all
  if echo "$raw" | grep -qE '^[A-Z]$'; then
    echo "bare_action_line"
    return
  fi
  # bare lowercase word (like 'read' or 'help') that isn't DSL
  if echo "$raw" | grep -qE '^[a-z]+$'; then
    echo "bare_action_line"
    return
  fi

  # missing_block_terminator: has opening of think/reason block but no proper closing
  if echo "$raw" | grep -qiE '<think>' && ! echo "$raw" | grep -qiE '</think>'; then
    echo "missing_block_terminator"
    return
  fi

  # catch-all
  echo "$class"
}

# Classify final answer text for wording issues
check_final_answer_wording() {
  local text="$1"
  local has_errors="$2"

  # false_success_wording: claims success when errors present
  if [[ "$has_errors" == "true" ]]; then
    if echo "$text" | grep -qiE '(completed successfully|exchange succeeded|action completed|successfully processed|operation completed)'; then
      echo "false_success_wording"
      return
    fi
  fi

  # missing_evidence_final: acknowledges inability
  if echo "$text" | grep -qiE '(could not continue|cannot continue|not have enough|insufficient evidence|cannot determine|unable to (find|determine|complete))'; then
    echo "missing_evidence_final"
    return
  fi

  echo ""
}

# Check trace for hidden retry/fallback patterns
check_hidden_retry() {
  local trace_file="$1"
  local class=""

  # Check for fallback usage
  if grep -qE '\[INTEL_FALLBACK\]' "$trace_file" 2>/dev/null; then
    # Check if final answer mentions fallback
    if ! grep -qiE '(fallback|retry|attempt|could not continue)' "$2" 2>/dev/null; then
      echo "hidden_retry_or_fallback"
      return
    fi
    # But still note it if fallback occurred silently
    local fallback_count
    fallback_count=$(grep -cE '\[INTEL_FALLBACK\]' "$trace_file" 2>/dev/null || echo 0)
    if [[ "$fallback_count" -gt 1 ]]; then
      echo "hidden_retry_or_fallback"
      return
    fi
  fi

  echo ""
}

# ---------- Extract model from trace_debug.log ----------
extract_model() {
  local trace_file="$1"
  grep -m1 '\[HTTP_START\] model=' "$trace_file" 2>/dev/null | sed 's/.*model=//' | sed 's/ .*//' || echo "unknown"
}

# ---------- Extract route near failure (last route before tool_loop or INTEL_FALLBACK) ----------
extract_route_near_failure() {
  local trace_file="$1"
  # Find the line number of first tool_loop or INTEL_FALLBACK
  local fail_line
  fail_line=$(grep -n -m1 -E '(tool_loop|INTEL_FALLBACK)' "$trace_file" 2>/dev/null | head -1 | cut -d: -f1 || echo "")
  if [[ -z "$fail_line" ]]; then
    echo "unknown"
    return
  fi
  # Look backward from failure line for most recent route= line
  local route_line
  route_line=$(head -n "$fail_line" "$trace_file" 2>/dev/null | grep -E '^trace: route=' | tail -1 | sed 's/.*route=//' | sed 's/ .*//' || echo "unknown")
  echo "${route_line:-unknown}"
}

# ---------- Extract failed intel unit ----------
extract_failed_unit() {
  local trace_file="$1"
  grep -m1 '\[INTEL_FALLBACK\]' "$trace_file" 2>/dev/null | sed 's/.*unit=//' | sed 's/ .*//' || echo ""
}

# ---------- Map failure to suggested owning task ----------
map_owning_task() {
  local failure_class="$1"
  case "$failure_class" in
    provider_markup)      echo 392 ;;
    invalid_action_dsl)   echo 387 ;;
    bare_action_line)     echo 387 ;;
    missing_block_terminator) echo 387 ;;
    prose_before_action)  echo 387 ;;
    false_success_wording) echo 398 ;;
    missing_evidence_final) echo 398 ;;
    hidden_retry_or_fallback) echo 395 ;;
    *)                    echo "" ;;
  esac
}

# ---------- Main Scan ----------
scan_start=$(date +%s)

# Collect session directories
shopt -s nullglob
session_dirs=("$SESSIONS_DIR"/s_*/)
shopt -u nullglob

total_sessions=${#session_dirs[@]}
failed_sessions=0

# Emit TOML header
cat <<EOF
# Session Regression Scan Report
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Sessions scanned: $total_sessions
# Scanner: $SCRIPT_NAME

[metadata]
scanned_at = $(date +%s)
session_count = $total_sessions

EOF

# Scan each session
for session_dir in "${session_dirs[@]}"; do
  session_id=$(basename "$session_dir")
  session_json="$session_dir/session.json"
  trace_log="$session_dir/trace_debug.log"
  final_answers=("$session_dir"/artifacts/*final_answer*.txt)
  final_answer_md=("$session_dir"/artifacts/*final_answer*.md)

  # Skip if no session.json
  [[ -f "$session_json" ]] || continue

  # Check for errors in session.json
  has_errors=false
  error_turns=0
  turn_errors=""

  if command -v jq &>/dev/null; then
    # Extract turn summaries with errors using jq
    turn_count=$(jq -r '.turn_summaries | to_entries[] | select(.value.errors // [] | length > 0) | .key' "$session_json" 2>/dev/null || true)
    if [[ -n "$turn_count" ]]; then
      has_errors=true
      error_turns=$(echo "$turn_count" | wc -l | tr -d ' ')
      turn_errors=$(jq -r '.turn_summaries | to_entries[] | select(.value.errors // [] | length > 0) | .value.errors[]' "$session_json" 2>/dev/null | head -5 || true)
    fi
  else
    # Fallback: grep for errors in JSON
    if grep -q '"errors"' "$session_json" 2>/dev/null && ! grep -q '"errors": \[\]' "$session_json" 2>/dev/null; then
      has_errors=true
      error_turns=1
    fi
  fi

  # Skip sessions with no errors at all
  [[ "$has_errors" == "false" ]] && continue

  # Read trace log for failure classification
  first_failure=""
  failure_signatures=()
  model="unknown"
  route="unknown"
  failed_unit=""

  if [[ -f "$trace_log" ]]; then
    model=$(extract_model "$trace_log")
    route=$(extract_route_near_failure "$trace_log")
    failed_unit=$(extract_failed_unit "$trace_log")

    # Extract all invalid DSL parse error lines
    while IFS= read -r line; do
      raw=$(echo "$line" | sed 's/.*raw=//' 2>/dev/null || true)
      if [[ -n "$raw" ]]; then
        class=$(classify_raw_action "$raw")
        if [[ -z "$first_failure" ]]; then
          first_failure="$class"
        fi
        # Deduplicate signatures
        sig_found=false
        for sig in "${failure_signatures[@]}"; do
          [[ "$sig" == "$class" ]] && sig_found=true && break
        done
        [[ "$sig_found" == "false" ]] && failure_signatures+=("$class")
      fi
    done < <(grep -E 'trace: tool_loop: invalid DSL parse' "$trace_log" 2>/dev/null || true)

    # Also check INTEL_DSL_REPAIR lines (intel unit DSL failures)
    if grep -qE '\[INTEL_DSL_REPAIR\]' "$trace_log" 2>/dev/null; then
      local_class="invalid_action_dsl"
      [[ -z "$first_failure" ]] && first_failure="$local_class"
      sig_found=false
      for sig in "${failure_signatures[@]}"; do
        [[ "$sig" == "$local_class" ]] && sig_found=true && break
      done
      [[ "$sig_found" == "false" ]] && failure_signatures+=("$local_class")
    fi

    # Check INTEL_FALLBACK for hidden_retry
    if grep -qE '\[INTEL_FALLBACK\]' "$trace_log" 2>/dev/null; then
      local_class="hidden_retry_or_fallback"
      # Only add if not already present
      sig_found=false
      for sig in "${failure_signatures[@]}"; do
        [[ "$sig" == "$local_class" ]] && sig_found=true && break
      done
      [[ "$sig_found" == "false" ]] && failure_signatures+=("$local_class")
      [[ -z "$first_failure" ]] && first_failure="$local_class"
    fi
  fi

  # Check final answer for wording issues
  wording_class=""
  for fa_file in "${final_answers[@]}"; do
    if [[ -f "$fa_file" ]]; then
      fa_text=$(cat "$fa_file" 2>/dev/null || true)
      wc=$(check_final_answer_wording "$fa_text" "$has_errors")
      if [[ -n "$wc" ]]; then
        wording_class="$wc"
        break
      fi
    fi
  done
  if [[ -z "$wording_class" ]]; then
    for fa_file in "${final_answer_md[@]}"; do
      if [[ -f "$fa_file" ]]; then
        fa_text=$(cat "$fa_file" 2>/dev/null || true)
        wc=$(check_final_answer_wording "$fa_text" "$has_errors")
        if [[ -n "$wc" ]]; then
          wording_class="$wc"
          break
        fi
      fi
    done
  fi
  if [[ -n "$wording_class" ]]; then
    sig_found=false
    for sig in "${failure_signatures[@]}"; do
      [[ "$sig" == "$wording_class" ]] && sig_found=true && break
    done
    [[ "$sig_found" == "false" ]] && failure_signatures+=("$wording_class")
    [[ -z "$first_failure" ]] && first_failure="$wording_class"
  fi

  # Default to invalid_action_dsl if nothing else found
  if [[ ${#failure_signatures[@]} -eq 0 ]]; then
    failure_signatures=("invalid_action_dsl")
    first_failure="invalid_action_dsl"
  fi

  # Always set first_failure
  [[ -z "$first_failure" ]] && first_failure="${failure_signatures[0]}"

  # Map owning task
  owning_task=$(map_owning_task "$first_failure")

  # Build signatures array as TOML
  sigs_toml=""
  for sig in "${failure_signatures[@]}"; do
    if [[ -n "$sigs_toml" ]]; then
      sigs_toml="$sigs_toml, "
    fi
    sigs_toml="${sigs_toml}$(toml_string "$sig")"
  done

  # Emit session TOML block
  cat <<EOF
[sessions.$session_id]
first_failure = $(toml_string "$first_failure")
model = $(toml_string "$model")
route = $(toml_string "$route")
failed_unit = $(toml_string "$failed_unit")
owning_task = $(toml_string "$owning_task")
failure_count = ${#failure_signatures[@]}
error_turns = $error_turns
signatures = [$sigs_toml]
turn_errors = $(toml_multiline "$turn_errors")

EOF

  failed_sessions=$((failed_sessions + 1))
done

scan_end=$(date +%s)
elapsed=$((scan_end - scan_start))

cat <<EOF
[summary]
total_sessions = $total_sessions
failed_sessions = $failed_sessions
elapsed_seconds = $elapsed
clean_sessions = $((total_sessions - failed_sessions))

EOF
