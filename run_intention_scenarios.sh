#!/usr/bin/env bash
set -euo pipefail

# Replay scenarios/* through the intention workflow only.
#
# It extracts lines starting with "user:" and sends ONLY that user text to the
# model using config/<model>/intention.toml.
#
# Usage:
#   ./run_intention_scenarios.sh
#
# Env:
#   LLAMA_BASE_URL=http://192.168.1.186:8080
#   LLAMA_MODEL=<override model id>   # optional

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing $1" >&2; exit 127; }; }
need curl
need python3

BASE_URL="${LLAMA_BASE_URL:-http://192.168.1.186:8080}"

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

# Must match Rust sanitizer.
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

cfg="config/$folder/intention.toml"
if [[ ! -f "$cfg" ]]; then
  echo "Missing $cfg. Run elma-cli once to generate configs." >&2
  exit 1
fi

echo "Model: $model_id"
echo "Config: $cfg"
echo

system_prompt="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["system_prompt"])' "$cfg")"
temp="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["temperature"])' "$cfg")"
top_p="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["top_p"])' "$cfg")"
repeat_penalty="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["repeat_penalty"])' "$cfg")"
max_tokens="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["max_tokens"])' "$cfg")"
reasoning_format="$(python3 -c 'import sys,tomllib; print(tomllib.load(open(sys.argv[1],"rb"))["reasoning_format"])' "$cfg")"

for f in scenarios/intention/scenario_*.md; do
  echo "== $(basename "$f") =="
  while IFS= read -r line; do
    [[ "$line" == user:* ]] || continue
    msg="${line#user:}"
    msg="${msg# }"

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
python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$msg"
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

    word="$(printf '%s' "$out" | python3 -c 'import json,sys; o=json.load(sys.stdin); m=(o.get("choices") or [{}])[0].get("message") or {}; t=(m.get("content") or m.get("reasoning_content") or "").strip().split(); print(t[0] if t else "")')"
    printf "user: %s\n=>   %s\n\n" "$msg" "$word"
  done <"$f"
  echo
done
