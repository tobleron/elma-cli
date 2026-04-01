#!/usr/bin/env bash
set -euo pipefail

# Run stress tests from _stress_testing/ folder
# Directly calls the API (like run_intention_scenarios.sh)
#
# Usage:
#   ./run_stress_tests.sh
#
# Env:
#   LLAMA_BASE_URL=http://192.168.1.186:8080
#   LLAMA_MODEL=<override model id>

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing $1" >&2; exit 127; }; }
need curl
need python3

BASE_URL="${LLAMA_BASE_URL:-http://192.168.1.186:8080}"

# Get model ID
model_id="$(
  if [[ -n "${LLAMA_MODEL:-}" ]]; then
    echo "$LLAMA_MODEL"
  else
    curl -sS --max-time 5 "$BASE_URL/v1/models" | python3 -c 'import json,sys; o=json.load(sys.stdin); d=o.get("data") or []; print((d[0] or {}).get("id",""))'
  fi
)"

if [[ -z "$model_id" ]]; then
  echo "Could not determine model id from /v1/models" >&2
  exit 1
fi

# Get model-specific config
folder="$(python3 - <<'PY' "$model_id"
import re,sys
s=sys.argv[1]
out=[]
for ch in s:
  if ch.isalnum() or ch in "._-_":
    out.append(ch)
  elif ch.isspace():
    out.append("_")
  else:
    out.append("_")
t="".join(out)
while "__" in t:
  t=t.replace("__","_")
print(t.strip("_"))
PY
)"

cfg="config/$folder/orchestrator.toml"
if [[ ! -f "$cfg" ]]; then
  echo "Missing $cfg. Run elma-cli once to generate configs." >&2
  exit 1
fi

echo "=========================================="
echo "Stress Test Runner"
echo "=========================================="
echo "Model: $model_id"
echo "Config: $cfg"
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

# Run single stress test
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
  
  # Get config values
  system_prompt="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["system_prompt"])' "$cfg")"
  temp="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["temperature"])' "$cfg")"
  top_p="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["top_p"])' "$cfg")"
  repeat_penalty="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["repeat_penalty"])' "$cfg")"
  max_tokens="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["max_tokens"])' "$cfg")"
  reasoning_format="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["reasoning_format"])' "$cfg")"
  
  # Call API
  out="$(
    curl -sS --max-time 120 "$BASE_URL/v1/chat/completions" \
      -H 'Content-Type: application/json' \
      -d "$(python3 - <<PY
import json
print(json.dumps({
  "model": "$model_id",
  "messages": [{"role":"system","content": $(
python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$system_prompt"
)},{"role":"user","content": $(
python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$prompt"
)}],
  "temperature": float($temp),
  "top_p": float($top_p),
  "repeat_penalty": float($repeat_penalty),
  "stream": False,
  "max_tokens": int($max_tokens),
  "reasoning_format": "$reasoning_format",
}))
PY
)"
  )"
  
  # Extract response
  response="$(printf '%s' "$out" | python3 -c 'import json,sys; o=json.load(sys.stdin); m=(o.get("choices") or [{}])[0].get("message") or {}; print((m.get("content") or m.get("reasoning_content") or "").strip())')"
  
  echo "Response:"
  echo "------------------------------------------"
  echo "$response"
  echo "------------------------------------------"
  echo ""
  echo "✅ Test complete: $test_name"
  echo ""
}

# Run all stress tests in order
for file in _stress_testing/S*.md; do
  if [[ -f "$file" ]]; then
    run_test "$file" || true  # Continue even if test fails
  fi
done

echo "=========================================="
echo "All stress tests complete"
echo "=========================================="
