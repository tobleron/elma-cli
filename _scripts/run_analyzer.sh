#!/bin/bash
# run_analyzer.sh - Runs the _dev-system efficiency analyzer

set -e

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( dirname "$SCRIPT_DIR" )"

echo "🚀 Starting Architectural Analysis..."

# Navigate to the analyzer directory and run
cd "$PROJECT_ROOT/_dev-system/analyzer"
cargo run --quiet

echo "✅ Analysis complete. Check _dev-tasks/ for updated guidance."
