#!/usr/bin/env bash
set -euo pipefail

# Probe different parsing strategies for llama.cpp OpenAI-style responses.
#
# Default target matches your rubox llama-server.
#   ./probe_parsing.sh
#
# Override:
#   LLAMA_BASE_URL=http://192.168.1.186:8080 LLAMA_MODEL=Nanbeige-4.1-3B-Q6_K.gguf ./probe_parsing.sh

BASE_URL="${LLAMA_BASE_URL:-http://192.168.1.186:8080}"
MODEL="${LLAMA_MODEL:-Nanbeige-4.1-3B-Q6_K.gguf}"

N="${N_TRIALS:-8}"
TIMEOUT="${LLAMA_TIMEOUT:-60}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing $1" >&2; exit 127; }; }
need curl
need python3

post_chat() {
  local user="$1"
  local max_tokens="${2:-256}"
  curl -sS --max-time "$TIMEOUT" --retry 2 --retry-delay 0 --retry-all-errors \
    "$BASE_URL/v1/chat/completions" \
    -H 'Content-Type: application/json' \
    -d "{\"model\":\"$MODEL\",\"messages\":[{\"role\":\"user\",\"content\":$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$user")}],\"temperature\":0,\"top_p\":1,\"stream\":false,\"max_tokens\":$max_tokens}"
}

post_chat_with_system() {
  local system="$1"
  local user="$2"
  local max_tokens="${3:-256}"
  curl -sS --max-time "$TIMEOUT" --retry 2 --retry-delay 0 --retry-all-errors \
    "$BASE_URL/v1/chat/completions" \
    -H 'Content-Type: application/json' \
    -d "{\"model\":\"$MODEL\",\"messages\":[{\"role\":\"system\",\"content\":$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$system")},{\"role\":\"user\",\"content\":$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$user")}],\"temperature\":0,\"top_p\":1,\"stream\":false,\"max_tokens\":$max_tokens}"
}

extract() {
  local mode="$1"
  python3 - "$mode" <<'PY'
import json, re, sys

mode = sys.argv[1]
raw = sys.stdin.read()
if not raw.strip():
  sys.stdout.write("")
  raise SystemExit(0)
obj = json.loads(raw)
msg = (((obj.get("choices") or [{}])[0]).get("message") or {})
content = msg.get("content") or ""
reasoning = msg.get("reasoning_content") or ""
text = content or reasoning or ""

def out(s):
  sys.stdout.write((s or "").strip())

if mode == "content_only":
  out(content)
elif mode == "reasoning_only":
  out(reasoning)
elif mode == "content_else_reasoning":
  out(content if content.strip() else reasoning)
elif mode == "final_marker":
  # Look for the *last* line starting with FINAL:
  ms = re.findall(r'(?im)^\s*FINAL\s*:\s*(.+?)\s*$', text)
  out(ms[-1] if ms else "")
elif mode == "think_tags":
  # If <think>...</think> exists, return everything after the closing tag.
  m = re.search(r'(?is)</think>\s*(.*)\Z', text)
  out(m.group(1) if m else "")
else:
  raise SystemExit(f"unknown mode: {mode}")
PY
}

score_case() {
  local case_name="$1"
  local system="$2"
  local user="$3"
  local expect="$4"
  local max_tokens="$5"

  echo "== Case: $case_name =="
  local ok_content=0 ok_reasoning=0 ok_fallback=0 ok_final=0 ok_think=0
  local printed=0
  local bad=0
  for i in $(seq 1 "$N"); do
    local json
    if [[ -n "$system" ]]; then
      json="$(post_chat_with_system "$system" "$user" "$max_tokens" || true)"
    else
      json="$(post_chat "$user" "$max_tokens" || true)"
    fi
    if [[ -z "${json:-}" ]]; then
      bad=$((bad+1))
      continue
    fi

    local a b c d e
    a="$(printf '%s' "$json" | extract content_only)"
    b="$(printf '%s' "$json" | extract reasoning_only)"
    c="$(printf '%s' "$json" | extract content_else_reasoning)"
    d="$(printf '%s' "$json" | extract final_marker)"
    e="$(printf '%s' "$json" | extract think_tags)"

    if [[ "$printed" -eq 0 ]]; then
      printed=1
      echo "sample_extracts:"
      echo "  content_only:           $(printf '%q' "$a")"
      echo "  reasoning_only:         $(printf '%q' "$b")"
      echo "  content_else_reasoning: $(printf '%q' "$c")"
      echo "  final_marker (FINAL:):  $(printf '%q' "$d")"
      echo "  think_tags:             $(printf '%q' "$e")"
    fi

    [[ "$a" == "$expect" ]] && ok_content=$((ok_content+1))
    [[ "$b" == "$expect" ]] && ok_reasoning=$((ok_reasoning+1))
    [[ "$c" == "$expect" ]] && ok_fallback=$((ok_fallback+1))
    [[ "$d" == "$expect" ]] && ok_final=$((ok_final+1))
    [[ "$e" == "$expect" ]] && ok_think=$((ok_think+1))
  done

  echo "content_only:           $ok_content/$N"
  echo "reasoning_only:         $ok_reasoning/$N"
  echo "content_else_reasoning: $ok_fallback/$N"
  echo "final_marker (FINAL:):  $ok_final/$N"
  echo "think_tags:             $ok_think/$N"
  [[ "$bad" -gt 0 ]] && echo "request_failures:       $bad/$N"
  echo
}

echo "Target: $BASE_URL (model: $MODEL)"
echo "Trials per case: $N"
echo

# Case 1: exact one-token final.
score_case \
  "FINAL Marker (Long Budget)" \
  "Output only one line: FINAL: OK" \
  "Output the required final line." \
  "OK" \
  2048

score_case \
  "Think Tags (Long Budget)" \
  "Wrap your private reasoning in <think>...</think>. After </think>, output exactly: OK (no other text)." \
  "Follow the format exactly." \
  "OK" \
  2048
