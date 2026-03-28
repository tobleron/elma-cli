#!/usr/bin/env bash
set -euo pipefail

# Smoke test for a llama.cpp server (typically OpenAI-compatible).
#
# Usage:
#   ./smoke_llamacpp.sh
#
# Optional env vars:
#   LLAMA_HOST=192.168.1.186
#   LLAMA_PORT=8080                # on your rubox, llama-server is on 8080; 8082 is Open WebUI
#   LLAMA_BASE_URL=http://192.168.1.186:8082
#   LLAMA_MODEL=<model id>          # if omitted, we'll try to read the first id from /v1/models
#   LLAMA_TIMEOUT=15

red() { printf "\033[31m%s\033[0m\n" "$*" >&2; }
grn() { printf "\033[32m%s\033[0m\n" "$*"; }
ylw() { printf "\033[33m%s\033[0m\n" "$*"; }

need() {
  command -v "$1" >/dev/null 2>&1 || { red "Missing dependency: $1"; exit 127; }
}

need curl

LLAMA_HOST="${LLAMA_HOST:-192.168.1.186}"
LLAMA_PORT="${LLAMA_PORT:-8080}"
LLAMA_BASE_URL="${LLAMA_BASE_URL:-http://${LLAMA_HOST}:${LLAMA_PORT}}"
LLAMA_TIMEOUT="${LLAMA_TIMEOUT:-15}"

tmpdir="$(mktemp -d 2>/dev/null || mktemp -d -t llamasmoke)"
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

http_get() {
  local path="$1"
  curl -fsS --max-time "$LLAMA_TIMEOUT" "${LLAMA_BASE_URL}${path}"
}

http_post_json() {
  local path="$1"
  local json_file="$2"
  curl -fsS --max-time "$LLAMA_TIMEOUT" \
    -H 'Content-Type: application/json' \
    -d @"$json_file" \
    "${LLAMA_BASE_URL}${path}"
}

grn "Base URL: ${LLAMA_BASE_URL}"

# 1) /health
ylw "1) GET /health"
if http_get "/health" >"$tmpdir/health.json"; then
  grn "PASS: /health"
else
  red "FAIL: /health"
  exit 1
fi

# 2) /v1/models (optional, but preferred). If this returns HTML, you're probably
# talking to a UI proxy (eg Open WebUI) not llama-server.
ylw "2) GET /v1/models"
models_ok=0
if http_get "/v1/models" >"$tmpdir/models.json"; then
  if head -c 64 "$tmpdir/models.json" | LC_ALL=C tr -d '\n' | grep -qi '<!doctype\|<html'; then
    red "FAIL: /v1/models returned HTML. This is not a llama-server OpenAI API."
    red "Hint: Open WebUI often runs on 8082, while llama-server runs on a different port (commonly 8080)."
    exit 1
  fi
  models_ok=1
  grn "PASS: /v1/models"
else
  ylw "WARN: /v1/models not available (some llama.cpp builds disable it). Continuing."
fi

model_id="${LLAMA_MODEL:-}"
if [[ -z "$model_id" && "$models_ok" == "1" ]]; then
  # Prefer jq, but fall back to a tiny python parser.
  if command -v jq >/dev/null 2>&1; then
    model_id="$(jq -r '.data[0].id // empty' "$tmpdir/models.json" || true)"
  else
    model_id="$(
      python3 - <<'PY' 2>/dev/null || true
import json,sys
obj=json.load(open(sys.argv[1],'r'))
data=obj.get("data") or []
mid=(data[0] or {}).get("id") if data else ""
print(mid or "")
PY
      "$tmpdir/models.json"
    )"
  fi
fi

if [[ -z "$model_id" ]]; then
  ylw "WARN: Could not infer model id from /v1/models. Set LLAMA_MODEL to skip inference."
  # Keep going; some servers ignore the model field anyway.
  model_id="(unknown)"
fi

# 3) /v1/chat/completions
ylw "3) POST /v1/chat/completions"
cat >"$tmpdir/chat.json" <<JSON
{
  "model": "${model_id}",
  "messages": [
    {"role": "user", "content": "Respond with exactly: OK"}
  ],
  "temperature": 0.0,
  "stream": false,
  "max_tokens": 32
}
JSON

if http_post_json "/v1/chat/completions" "$tmpdir/chat.json" >"$tmpdir/chat_out.json"; then
  # Heuristic check: ensure there's a 'choices' array in the response, and that at least
  # one of message.content / message.reasoning_content exists (thinking models may put
  # output into reasoning_content).
  if command -v jq >/dev/null 2>&1; then
    has_choices="$(jq -r 'has("choices")' "$tmpdir/chat_out.json" 2>/dev/null || echo "false")"
    has_msg="$(jq -r '(.choices[0].message | has("content") or has("reasoning_content")) // false' "$tmpdir/chat_out.json" 2>/dev/null || echo "false")"
  else
    has_choices="$(
      python3 - <<'PY' 2>/dev/null || true
import json,sys
obj=json.load(open(sys.argv[1],'r'))
print("true" if "choices" in obj else "false")
PY
      "$tmpdir/chat_out.json"
    )"
    has_msg="$(
      python3 - <<'PY' 2>/dev/null || true
import json,sys
obj=json.load(open(sys.argv[1],'r'))
try:
  msg=(obj.get("choices") or [{}])[0].get("message") or {}
  print("true" if ("content" in msg or "reasoning_content" in msg) else "false")
except Exception:
  print("false")
PY
      "$tmpdir/chat_out.json"
    )"
  fi

  if [[ "$has_choices" == "true" && "$has_msg" == "true" ]]; then
    grn "PASS: /v1/chat/completions"
    grn "Smoke test: OK"
  else
    red "FAIL: /v1/chat/completions returned unexpected JSON."
    ylw "Raw response saved to: $tmpdir/chat_out.json"
    exit 1
  fi
else
  red "FAIL: /v1/chat/completions"
  exit 1
fi
