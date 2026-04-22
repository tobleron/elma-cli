#! /bin/bash
# UI Parity Probe — runs Elma through the test harness and checks behavior
# Usage: ./ui_parity_probe.sh --fixture <name> [--snapshot] [--all]
# Fixtures live in tests/fixtures/ui_parity/

set -euo pipefail

FIXTURE=""
SNAPSHOT_MODE=0
ALL_FIXTURES=0
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fixture)
      FIXTURE="$2"
      shift 2
      ;;
    --snapshot)
      SNAPSHOT_MODE=1
      shift
      ;;
    --all)
      ALL_FIXTURES=1
      shift
      ;;
    *)
      echo "Unknown arg: $1"
      exit 1
      ;;
  esac
done

cd "$PROJECT_ROOT"

if [[ ! -x "target/debug/elma-cli" ]]; then
  echo "Building elma..."
  cargo build -q
fi

if [[ $ALL_FIXTURES -eq 1 ]]; then
  echo "==> Running all UI parity fixtures..."
  cargo test --test ui_parity 2>&1
  exit $?
fi

if [[ -z "$FIXTURE" ]]; then
  echo "Usage: $0 --fixture <name> [--snapshot] [--all]"
  echo "Available fixtures:"
  for f in tests/fixtures/ui_parity/*.yaml; do
    if [[ -f "$f" ]]; then
      basename "$f" .yaml
    fi
  done
  exit 1
fi

FIXTURE_FILE="tests/fixtures/ui_parity/${FIXTURE}.yaml"

if [[ ! -f "$FIXTURE_FILE" ]]; then
  echo "Fixture not found: ${FIXTURE_FILE}"
  exit 1
fi

echo "==> Running UI parity fixture: ${FIXTURE}"
echo "    Fixture: ${FIXTURE_FILE}"

if [[ $SNAPSHOT_MODE -eq 1 ]]; then
  echo "    Snapshot mode: will update snapshots if they differ"
  export INSTA_UPDATE=always
fi

# Run the specific fixture test
TEST_FILTER="${FIXTURE//-/_}_fixture"
cargo test --test ui_parity "${TEST_FILTER}" 2>&1

echo ""
echo "Fixture completed: ${FIXTURE}"
