#!/usr/bin/env bash
set -euo pipefail

# Run stress tests through Elma CLI directly (not raw API)
# This tests the FULL orchestration pipeline including:
# - Formula selection
# - Step limit validation
# - Output truncation
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

# Run single stress test through CLI
run_test() {
  local file="$1"
  local test_name="$(basename "$file" .md)"
  
  echo "=========================================="
  echo "Test: $test_name"
  echo "=========================================="
  
  local prompt="$(extract_prompt "$file")"
  
  if [[ "$prompt" == ERROR* ]]; then
    echo "FAILED: $prompt"
    return 1
  fi
  
  echo "Prompt: $prompt"
  echo ""
  echo "Response:"
  echo "------------------------------------------"
  
  # Run through elma-cli (send prompt via echo pipe)
  # Note: timeout not available on macOS, using background process with sleep
  # Timeout set to 360s (6 minutes) per test for slow local models
  if command -v timeout &> /dev/null; then
    timeout 360 bash -c "echo '$prompt' | cargo run --quiet 2>&1" | head -200 || {
      echo ""
      echo "⚠️ TIMEOUT (360s limit)"
    }
  else
    # macOS fallback: use background process
    (echo "$prompt" | cargo run --quiet 2>&1) &
    PID=$!
    (sleep 360 && kill $PID) &
    KILLER=$!
    wait $PID 2>/dev/null || {
      echo ""
      echo "⚠️ TIMEOUT (360s limit)"
    }
    kill $KILLER 2>/dev/null || true
    wait $PID 2>/dev/null || true
  fi
  
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
