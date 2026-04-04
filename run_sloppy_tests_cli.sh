#!/usr/bin/env bash
set -euo pipefail

# Run SLOPPY HUMAN stress tests through Elma CLI directly
# Focuses on H001, H002, H003

BASE_URL="${LLAMA_BASE_URL:-http://192.168.1.186:8080}"
export LLAMA_BASE_URL

echo "=========================================="
echo "Sloppy Human Test Runner (CLI Mode)"
echo "=========================================="
echo "Base URL: $BASE_URL"
echo ""

STRESS_SANDBOX_ROOT="_stress_testing"
export ELMA_STRESS_SANDBOX_ROOT="$STRESS_SANDBOX_ROOT"

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
# Extract the final Elma response block
# Looking for "Elma:" followed by text until the end or another header
parts = text.split("Elma:")
if len(parts) > 1:
    print(parts[-1].strip())
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
# Extract the final Elma response block
parts = text.split("Elma:")
answer = parts[-1].strip() if len(parts) > 1 else ""
repo = Path.cwd()

def fail(msg: str) -> None:
    print(f"SEMANTIC FAILURE: {msg}")
    sys.exit(1)

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
    fail("Could not find a final Elma answer")

lower_prompt = prompt.lower()
lower_answer = answer.lower()

# H001: Sloppy Chat Greeting
if "yo elma" in lower_prompt:
    # Expect a normal conversational greeting, not meta-failure
    if "no steps observed" in lower_answer or "internal error" in lower_answer:
        fail("Answer contains meta-failure or internal error text")
    if len(answer) > 200:
        fail("Greeting is too long/verbose")

# H002: Sloppy Casual Shell Request
if "list src" in lower_prompt:
    # Handle filenames that might not have src/ prefix in the output
    existing = extract_existing_files(answer)
    src_files = [f for f in existing if f.startswith("src/")]
    
    # Also check for basenames that exist in src/
    if not src_files:
        tokens = re.findall(r"[_A-Za-z0-9./-]+\.(?:go|rs|py|ts|tsx|js|jsx|md|toml|json|yaml|yml|sql)", answer)
        for token in tokens:
            normalized = token.rstrip(".,:;)")
            if (repo / "src" / normalized).exists():
                src_files.append(f"src/{normalized}")
    
    if not src_files:
        fail("No real files from 'src/' detected in answer")

# H003: Sloppy Multi-Instruction Bounded Workflow
if "2 bullets" in lower_prompt and "identify the primary entry point" in lower_prompt:
    # Check for 2 bullets
    bullet_lines = [
        line for line in answer.splitlines()
        if line.strip().startswith("- ") or line.strip().startswith("* ") or re.match(r"^\d+\.\s", line.strip())
    ]
    if len(bullet_lines) != 2:
        fail(f"Expected exactly 2 bullet lines, got {len(bullet_lines)}")
    
    # Check for exact entry point path
    existing = extract_existing_files(answer)
    entry_point = "_stress_testing/_opencode_for_testing/main.go"
    if entry_point not in existing:
        fail(f"Expected exact entry point path '{entry_point}' not found in answer")

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
    tail -n 40 "$trace_file"
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

  validate_semantic_answer "$test_name" "$prompt" "$output_file" || {
    print_failure_context "$session_id"
    return 1
  }

  return 0
}

extract_prompt() {
  local file="$1"
  python3 - <<PY "$file"
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

run_test() {
  local file="$1"
  local test_name="$(basename "$file" .md)"
  local terminated=0
  local output_file="/tmp/elma_sloppy_test_$$.txt"
  
  echo "=========================================="
  echo "Sloppy Test: $test_name"
  echo "=========================================="
  
  local prompt="$(extract_prompt "$file")"
  
  if [[ "$prompt" == ERROR* ]]; then
    echo "FAILED: $prompt"
    return 1
  fi
  
  echo "Prompt: $prompt"
  echo ""
  
  # macOS background process with progress monitoring
  (echo "$prompt" | cargo run --quiet 2>&1) > "$output_file" &
  PID=$!
  
  ELAPSED=0
  LAST_SIZE=0
  NO_PROGRESS_COUNT=0
  MAX_NO_PROGRESS=10 # 5 minutes of no progress
  MAX_ELAPSED=300    # 5 minutes hard limit
  
  while kill -0 $PID 2>/dev/null; do
    sleep 30
    ELAPSED=$((ELAPSED + 30))
    
    if [[ -f "$output_file" ]]; then
      CURRENT_SIZE=$(wc -c < "$output_file")
      if [[ "$CURRENT_SIZE" -eq "$LAST_SIZE" ]]; then
        NO_PROGRESS_COUNT=$((NO_PROGRESS_COUNT + 1))
        echo "  - No progress ($NO_PROGRESS_COUNT/$MAX_NO_PROGRESS)"
        if [[ $NO_PROGRESS_COUNT -ge $MAX_NO_PROGRESS ]]; then
          echo "  - Terminating: No progress"
          kill $PID 2>/dev/null
          terminated=1
          break
        fi
      else
        NO_PROGRESS_COUNT=0
        LAST_SIZE=$CURRENT_SIZE
        echo "  - Progress: ${CURRENT_SIZE} bytes"
      fi
    fi
    
    if [[ $ELAPSED -ge $MAX_ELAPSED ]]; then
      echo "  - Terminating: Timeout"
      kill $PID 2>/dev/null
      terminated=1
      break
    fi
  done
  
  # Final cat to see what happened
  cat "$output_file"
  
  validate_cli_run "$test_name" "$prompt" "$output_file" "$terminated" || {
    rm -f "$output_file"
    return 1
  }
  rm -f "$output_file"
  
  echo ""
  echo "✅ Sloppy Test PASSED: $test_name"
  echo ""
}

# Run H001, H002, H003
for file in _stress_testing/H00[123]*.md; do
  if [[ -f "$file" ]]; then
    if ! run_test "$file"; then
      echo "❌ SLOPPY FAILURE: $file"
      exit 1
    fi
  fi
done

echo "=========================================="
echo "All SLOPPY tests PASSED!"
echo "=========================================="
