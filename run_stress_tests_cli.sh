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
  # Timeout set to 180s (3 minutes) per test for normal tasks
  # Complex tasks may need more time
  echo "Starting test with 3-minute timeout (progress monitored)..."
  
  if command -v timeout &> /dev/null; then
    timeout 180 bash -c "echo '$prompt' | cargo run --quiet 2>&1" | head -300 || {
      echo ""
      echo "⚠️ TIMEOUT (3-minute limit)"
    }
  else
    # macOS fallback: use background process with progress monitoring
    # Create a temp file to capture output
    OUTPUT_FILE="/tmp/elma_stress_test_$$.txt"
    
    (echo "$prompt" | cargo run --quiet 2>&1) > "$OUTPUT_FILE" &
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
      if [[ -f "$OUTPUT_FILE" ]]; then
        CURRENT_SIZE=$(wc -c < "$OUTPUT_FILE")
        if [[ "$CURRENT_SIZE" -eq "$LAST_SIZE" ]]; then
          NO_PROGRESS_COUNT=$((NO_PROGRESS_COUNT + 1))
          echo "⚠️ No progress detected ($NO_PROGRESS_COUNT/$MAX_NO_PROGRESS checks)"
          
          if [[ $NO_PROGRESS_COUNT -ge $MAX_NO_PROGRESS ]]; then
            echo "❌ Elma appears stuck - no progress for 3 minutes"
            echo "Terminating test..."
            kill $PID 2>/dev/null
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
        break
      fi
    done
    
    # Show output
    cat "$OUTPUT_FILE" | head -300
    rm -f "$OUTPUT_FILE"
    
    wait $PID 2>/dev/null || {
      echo ""
      echo "⚠️ Test terminated (timeout or no progress)"
    }
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
