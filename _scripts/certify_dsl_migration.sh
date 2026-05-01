#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "[certify] syncing managed prompts (dry-run)"
SYNC_OUT="$(cargo run --quiet -- sync-prompts --dry-run || true)"
echo "$SYNC_OUT"

UPDATED="$(echo "$SYNC_OUT" | awk -F'updated=' '{print $2}' | awk '{print $1}' | tail -n 1 || true)"
if [[ -n "${UPDATED}" ]] && [[ "${UPDATED}" != "0" ]]; then
  echo "[certify] FAIL: managed prompts drift detected (updated=${UPDATED})."
  echo "[certify] Run: cargo run -- sync-prompts"
  exit 1
fi

echo "[certify] static gate: no model-output JSON contracts in src/ or config/"
RG_PATTERNS='Return ONLY one valid JSON object|Output valid JSON|Output ONLY valid Program JSON|Program JSON|chat_json_with_repair'
if rg -n "$RG_PATTERNS" src config \
  --glob '!config/**/*.DISABLED/**' \
  --glob '!config/**/tune/**' \
  --glob '!config/**/tune\\**' \
  --glob '!docs/**' \
  --glob '!_scripts/**' \
  ; then
  echo "[certify] FAIL: found forbidden JSON-contract patterns."
  exit 1
fi

echo "[certify] running verification suite"
bash -n _scripts/certify_dsl_migration.sh
cargo fmt --check
cargo test dsl
cargo test agent_protocol
cargo test intel_units
cargo test tool_loop
cargo test stop_policy
cargo test prompt_core
cargo test
cargo check --all-targets

echo "[certify] OK"

