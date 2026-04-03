#!/usr/bin/env bash
set -euo pipefail

# Run stress tests through Elma CLI directly (not raw API)
# This tests the FULL orchestration pipeline including:
# - Formula selection
# - Step limit validation
# - Output truncation
# - Progress monitoring (prevents endless loops)
#
# Timeout: 30 minutes per test with progress monitoring
# If no output progress for 10 minutes, test is terminated
#
# Usage:
#   ./run_stress_tests_cli.sh
#
# Env:
#   LLAMA_BASE_URL=http://192.168.1.186:8080
#   LLAMA_MODEL=<override model id>

BASE_URL="${LLAMA_BASE_URL:-http://192.168.1.186:8080}"
export LLAMA_BASE_URL

echo "=========================================="
echo "Stress Test Runner (CLI Mode)"
echo "=========================================="
echo "Base URL: $BASE_URL"
echo ""

STRESS_SANDBOX_ROOT="_stress_testing"
export ELMA_STRESS_SANDBOX_ROOT="$STRESS_SANDBOX_ROOT"

last_session_id=""

extract_session_id() {
  local file="$1"
  python3 - <<'PY' "$file"
import re, sys

text = open(sys.argv[1], "r", encoding="utf-8", errors="ignore").read()
match = re.search(r"session\s+\x1b\[[0-9;]*m\s*(s_[0-9_]+)", text)
if not match:
    match = re.search(r"session\s+(s_[0-9_]+)", text)
if match:
    print(match.group(1))
PY
}

extract_final_answer() {
  local file="$1"
  python3 - <<'PY' "$file"
import re, sys

text = open(sys.argv[1], "r", encoding="utf-8", errors="ignore").read()
matches = re.findall(r"Elma:\s*(.+)", text)
if matches:
    print(matches[-1].strip())
PY
}

validate_semantic_answer() {
  local test_name="$1"
  local prompt="$2"
  local output_file="$3"

  python3 - <<'PY' "$test_name" "$prompt" "$output_file"
import os
import re
import sys
from pathlib import Path

test_name, prompt, output_file = sys.argv[1:4]
text = Path(output_file).read_text(encoding="utf-8", errors="ignore")
answers = re.findall(r"Elma:\s*(.+)", text)
answer = answers[-1].strip() if answers else ""
repo = Path.cwd()

def fail(msg: str) -> None:
    print(msg)
    raise SystemExit(1)

def extract_existing_files(candidate_text: str):
    tokens = re.findall(r"[_A-Za-z0-9./-]+\.(?:go|rs|py|ts|tsx|js|jsx|md|toml|json|yaml|yml|sql)", candidate_text)
    seen = []
    for token in tokens:
        normalized = token.rstrip(".,:;)")
        path = repo / normalized
        if path.exists() and normalized not in seen:
            seen.append(normalized)
    return seen

if not answer:
    fail("FAILED: semantic validation could not find a final answer")

lower_prompt = prompt.lower()

if "three potential files" in lower_prompt:
    existing = extract_existing_files(answer)
    if len(existing) < 3:
        fail(f"FAILED: expected at least 3 grounded file candidates, got {len(existing)}")

if "3-bullet point executive summary" in lower_prompt or "3 bullet point executive summary" in lower_prompt:
    bullet_lines = [
        line for line in answer.splitlines()
        if line.strip().startswith("- ") or line.strip().startswith("* ") or re.match(r"^\d+\.\s", line.strip())
    ]
    if len(bullet_lines) != 3:
        fail(f"FAILED: expected exactly 3 bullet lines, got {len(bullet_lines)}")

if "identify the primary entry point" in lower_prompt:
    existing = extract_existing_files(answer)
    if not existing:
        fail("FAILED: expected a grounded file path in final answer")

PY
}

print_failure_context() {
  local session_id="$1"
  if [[ -z "$session_id" ]]; then
    return 0
  fi

  local trace_file="sessions/${session_id}/trace_debug.log"
  echo ""
  echo "Failure context:"
  echo "  session: $session_id"
  if [[ -f "$trace_file" ]]; then
    echo "  trace: $trace_file"
    echo "  trace tail:"
    tail -n 20 "$trace_file"
  else
    echo "  trace: missing"
  fi
}

validate_cli_run() {
  local test_name="$1"
  local prompt="$2"
  local output_file="$3"
  local terminated="$4"
  local session_id
  session_id="$(extract_session_id "$output_file")"
  last_session_id="$session_id"

  if [[ "$terminated" -eq 1 ]]; then
    echo "FAILED: run terminated early (timeout or no progress)"
    print_failure_context "$session_id"
    return 1
  fi

  if [[ -z "$session_id" ]]; then
    echo "FAILED: could not detect session id from CLI output"
    return 1
  fi

  local final_answer
  final_answer="$(extract_final_answer "$output_file")"
  if [[ -z "$final_answer" ]]; then
    echo "FAILED: no final Elma answer detected"
    print_failure_context "$session_id"
    return 1
  fi

  local trace_file="sessions/${session_id}/trace_debug.log"
  if [[ -f "$trace_file" ]]; then
    if grep -q "workflow_recovery=failed" "$trace_file" && ! grep -q "note: Retry .* succeeded" "$output_file"; then
      echo "FAILED: workflow recovery failed without a successful retry"
      print_failure_context "$session_id"
      return 1
    fi
  fi

  validate_semantic_answer "$test_name" "$prompt" "$output_file" || {
    print_failure_context "$session_id"
    return 1
  }

  return 0
}

# Extract prompt from stress test file
extract_prompt() {
  local file="$1"
  python3 - <<PY "$file"
import re, sys

with open(sys.argv[1], 'r') as f:
    content = f.read()

# Find the prompt section
match = re.search(r'## 1\. The Test \(Prompt\)\s*\n\s*"([^"]+)"', content)
if match:
    print(match.group(1))
else:
    print("ERROR: Could not extract prompt", file=sys.stderr)
    sys.exit(1)
PY
}

validate_prompt_sandbox() {
  local file="$1"
  local prompt="$2"

  if [[ "$prompt" != *"$STRESS_SANDBOX_ROOT/"* ]] && [[ "$prompt" != *"$STRESS_SANDBOX_ROOT"* ]]; then
    echo "FAILED: $file does not keep the prompt inside $STRESS_SANDBOX_ROOT" >&2
    return 1
  fi
}

# Run single stress test through CLI
run_test() {
  local file="$1"
  local test_name="$(basename "$file" .md)"
  local terminated=0
  local output_file="/tmp/elma_stress_test_$$.txt"
  
  echo "=========================================="
  echo "Test: $test_name"
  echo "=========================================="
  
  local prompt="$(extract_prompt "$file")"
  
  if [[ "$prompt" == ERROR* ]]; then
    echo "FAILED: $prompt"
    return 1
  fi

  validate_prompt_sandbox "$file" "$prompt" || return 1
  
  echo "Prompt: $prompt"
  echo ""
  echo "Sandbox root: $STRESS_SANDBOX_ROOT"
  echo ""
  echo "Response:"
  echo "------------------------------------------"
  
  # Run through elma-cli (send prompt via echo pipe)
  # Timeout set to 180s (3 minutes) per test for normal tasks
  # Complex tasks may need more time
  echo "Starting test with 3-minute timeout (progress monitored)..."
  
  if command -v timeout &> /dev/null; then
    if ! timeout 180 bash -c "echo '$prompt' | cargo run --quiet 2>&1" > "$output_file"; then
      echo ""
      echo "⚠️ TIMEOUT (3-minute limit)"
      terminated=1
    fi
    cat "$output_file" | head -300
  else
    # macOS fallback: use background process with progress monitoring
    # Create a temp file to capture output
    (echo "$prompt" | cargo run --quiet 2>&1) > "$output_file" &
    PID=$!
    
    # Monitor for progress (check if output is still growing)
    ELAPSED=0
    LAST_SIZE=0
    NO_PROGRESS_COUNT=0
    MAX_NO_PROGRESS=6  # 6 checks with no progress = stuck (6 minutes max)
    MAX_ELAPSED=180  # Hard limit 3 minutes
    
    while kill -0 $PID 2>/dev/null; do
      sleep 30  # Check every 30 seconds
      ELAPSED=$((ELAPSED + 30))
      
      # Check if test is still producing output (progress check)
      if [[ -f "$output_file" ]]; then
        CURRENT_SIZE=$(wc -c < "$output_file")
        if [[ "$CURRENT_SIZE" -eq "$LAST_SIZE" ]]; then
          NO_PROGRESS_COUNT=$((NO_PROGRESS_COUNT + 1))
          echo "⚠️ No progress detected ($NO_PROGRESS_COUNT/$MAX_NO_PROGRESS checks)"
          
          if [[ $NO_PROGRESS_COUNT -ge $MAX_NO_PROGRESS ]]; then
            echo "❌ Elma appears stuck - no progress for 3 minutes"
            echo "Terminating test..."
            kill $PID 2>/dev/null
            terminated=1
            break
          fi
        else
          NO_PROGRESS_COUNT=0
          LAST_SIZE=$CURRENT_SIZE
          echo "✓ Progress detected (${CURRENT_SIZE} bytes so far)"
        fi
      fi
      
      # Hard timeout at 3 minutes
      if [[ $ELAPSED -ge $MAX_ELAPSED ]]; then
        echo "⚠️ 3-minute timeout reached"
        kill $PID 2>/dev/null
        terminated=1
        break
      fi
    done
    
    # Show output
    cat "$output_file" | head -300
    
    wait $PID 2>/dev/null || {
      echo ""
      echo "⚠️ Test terminated (timeout or no progress)"
      terminated=1
    }
  fi

  validate_cli_run "$test_name" "$prompt" "$output_file" "$terminated" || {
    rm -f "$output_file"
    return 1
  }
  rm -f "$output_file"
  
  echo ""
  echo "------------------------------------------"
  echo "✅ Test PASSED: $test_name"
  echo ""
}

# Run stress tests until first failure
for file in _stress_testing/S*.md; do
  if [[ -f "$file" ]]; then
    if ! run_test "$file"; then
      echo ""
      echo "=========================================="
      echo "❌ FIRST FAILURE: $file"
      echo "=========================================="
      echo "Stopping stress tests at first failure."
      echo "Troubleshoot this test before continuing."
      exit 1
    fi
  fi
done

echo "=========================================="
echo "All stress tests PASSED!"
echo "=========================================="
