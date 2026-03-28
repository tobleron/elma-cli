#!/usr/bin/env bash
set -euo pipefail

# Reliability probe for llama.cpp OpenAI-compatible /v1/chat/completions.
# Measures how often we get a separable "final" (message.content or a FINAL: line),
# vs only reasoning_content / no-final.
#
# Usage:
#   ./reliability_probe.sh
#
# Env:
#   LLAMA_BASE_URL=http://192.168.1.186:8080
#   LLAMA_MODEL=Nanbeige-4.1-3B-Q6_K.gguf
#   N=10

BASE_URL="${LLAMA_BASE_URL:-http://192.168.1.186:8080}"
MODEL="${LLAMA_MODEL:-Nanbeige-4.1-3B-Q6_K.gguf}"
N="${N:-10}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing $1" >&2; exit 127; }; }
need curl
need python3

call() {
  local system="$1"
  local user="$2"
  local max_tokens="$3"
  curl -sS --max-time 120 \
    "$BASE_URL/v1/chat/completions" \
    -H 'Content-Type: application/json' \
    -d "$(python3 - <<PY
import json
print(json.dumps({
  "model": "$MODEL",
  "messages": [
    {"role":"system","content": "$system"},
    {"role":"user","content": "$user"},
  ],
  "temperature": 0.0,
  "top_p": 1.0,
  "stream": False,
  "max_tokens": int($max_tokens),
  "reasoning_format": "auto",
}))
PY
)"
}

score_case() {
  local name="$1"
  local system="$2"
  local user="$3"
  local max_tokens="$4"
  echo "== $name =="

  local ok_final=0
  local ok_split=0
  local only_reasoning=0
  local failures=0

  for _ in $(seq 1 "$N"); do
    local out
    out="$(call "$system" "$user" "$max_tokens" || true)"
    if [[ -z "$out" ]]; then
      failures=$((failures+1))
      continue
    fi

    python3 - "$out" <<'PY' >/tmp/probe_result.$$.txt
import json,sys,re
obj=json.loads(sys.argv[1])
msg=((obj.get("choices") or [{}])[0]).get("message") or {}
c=(msg.get("content") or "").strip()
r=(msg.get("reasoning_content") or "").strip()

has_final = bool(c) or bool(re.search(r'(?m)^\s*FINAL\s*:\s*\S', r))
has_split = bool(c) and bool(r)
only_reasoning = (not c) and bool(r)

print("final=%d split=%d only_reasoning=%d" % (has_final, has_split, only_reasoning))
PY

    local line
    line="$(cat /tmp/probe_result.$$.txt)"
    [[ "$line" == *"final=1"* ]] && ok_final=$((ok_final+1))
    [[ "$line" == *"split=1"* ]] && ok_split=$((ok_split+1))
    [[ "$line" == *"only_reasoning=1"* ]] && only_reasoning=$((only_reasoning+1))
  done

  echo "final_present:    $ok_final/$N"
  echo "structured_split: $ok_split/$N"
  echo "only_reasoning:   $only_reasoning/$N"
  [[ "$failures" -gt 0 ]] && echo "request_failures: $failures/$N"
  echo
}

echo "Target: $BASE_URL"
echo "Model:  $MODEL"
echo "Trials per case: $N"
echo

score_case \
  "Greeting (default assistant)" \
  "You are a helpful assistant." \
  "hi" \
  256

score_case \
  "Strict FINAL-only" \
  "Output exactly one line: FINAL: <your answer>. Do not output reasoning." \
  "hi" \
  512

score_case \
  "Normal QA" \
  "You are a helpful assistant." \
  "What is the capital of France? Answer in one short sentence." \
  512
