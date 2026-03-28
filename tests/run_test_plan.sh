#!/usr/bin/env bash
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing $1" >&2; exit 127; }; }
need cargo
need rg

tmpdir="$(mktemp -d)"
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

run_case() {
  local name="$1"
  local input="$2"
  echo "== $name =="
  cargo run --quiet -- --no-color <<<"$input" >"$tmpdir/out.txt" 2>&1
  cat "$tmpdir/out.txt"
  echo
}

# 1) Shell action
run_case "Shell List" $'list files in current directory\n/exit\n'
sid2="$(rg '^Session: ' "$tmpdir/out.txt" | sed -E 's/^Session: //' | tail -n1)"
test -f "$sid2/shell/001.sh"
test -f "$sid2/shell/001.out"
rg -n 'ls ' "$sid2/shell/001.sh" >/dev/null

# 2) Plan workflow
run_case "Plan" $'Create a step-by-step plan to add a new config file to this Rust project.\n/exit\n'
sid3="$(rg '^Session: ' "$tmpdir/out.txt" | sed -E 's/^Session: //' | tail -n1)"
test -f "$sid3/plans/_master.md"
test -f "$sid3/plans/plan_001.md"

echo "OK"
