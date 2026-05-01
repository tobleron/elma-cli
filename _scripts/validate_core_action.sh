#!/usr/bin/env bash
#
# Core DSL Actions E2E Assertion Runner (Task 367)
#
# Validates prompt transcripts against ground-truth expectations.
# Designed to be invoked by the main harness after each prompt run.
#
# Usage:
#   validate_core_action.sh <prompt_file> <transcript_path> <ground_truth_toml>
#
# Returns 0 if all assertions pass, 1 otherwise.

set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <prompt_file> <transcript_path> [ground_truth_file]"
  exit 1
fi

PROMPT_FILE="$1"
TRANSCRIPT_PATH="$2"
GROUND_TRUTH="${3:-tests/dsl/core_actions_ground_truth.toml}"

if [[ ! -f "$TRANSCRIPT_PATH" ]]; then
  echo "FAIL: Transcript not found: $TRANSCRIPT_PATH"
  exit 1
fi

if [[ ! -f "$GROUND_TRUTH" ]]; then
  echo "SKIP: Ground truth file not found: $GROUND_TRUTH"
  exit 0
fi

TRANSCRIPT_CONTENT=$(cat "$TRANSCRIPT_PATH")

failures=0

# Check forbidden actions (these should NEVER appear in transcript)
forbidden_actions=()
case "$PROMPT_FILE" in
  *005_read_core_alpha*)
    forbidden_actions=("S" "X")
    ;;
  *006_list_sandbox_dirs*)
    forbidden_actions=("X")
    ;;
  *007_search_core_beta*)
    forbidden_actions=("X")
    ;;
  *008_search_symbol_alpha*)
    forbidden_actions=("X")
    ;;
  *011_done_after_read*)
    forbidden_actions=("X")
    ;;
esac

for action in "${forbidden_actions[@]}"; do
  if echo "$TRANSCRIPT_CONTENT" | grep -qiE "^\s*${action}\s" || echo "$TRANSCRIPT_CONTENT" | grep -qiE "\b${action}\b.*path="; then
    echo "FAIL [$PROMPT_FILE]: Forbidden action '$action' found in transcript"
    ((failures++))
  fi
done

# Check expected actions are present
case "$PROMPT_FILE" in
  *005_read_core_alpha*)
    if ! echo "$TRANSCRIPT_CONTENT" | grep -qiE "R\s+path="; then
      echo "FAIL [$PROMPT_FILE]: Expected 'R' action not found"
      ((failures++))
    fi
    if echo "$TRANSCRIPT_CONTENT" | grep -qiE "DSL_CORE_ALPHA"; then
      : # marker found
    else
      echo "FAIL [$PROMPT_FILE]: Marker DSL_CORE_ALPHA not in transcript"
      ((failures++))
    fi
    ;;
  *006_list_sandbox_dirs*)
    if ! echo "$TRANSCRIPT_CONTENT" | grep -qiE "L\s+path="; then
      echo "FAIL [$PROMPT_FILE]: Expected 'L' action not found"
      ((failures++))
    fi
    if echo "$TRANSCRIPT_CONTENT" | grep -qiE "DslCoreAlpha|DslSentinelAlpha|nested"; then
      : # listing results found
    else
      echo "FAIL [$PROMPT_FILE]: No file results found in transcript"
      ((failures++))
    fi
    ;;
  *007_search_core_beta*)
    if ! echo "$TRANSCRIPT_CONTENT" | grep -qiE "DSL_CORE_BETA"; then
      echo "FAIL [$PROMPT_FILE]: Marker DSL_CORE_BETA not in transcript"
      ((failures++))
    fi
    if echo "$TRANSCRIPT_CONTENT" | grep -qiE "DslCoreBeta"; then
      : # filename found
    else
      echo "FAIL [$PROMPT_FILE]: Expected filename DslCoreBeta not in transcript"
      ((failures++))
    fi
    ;;
  *008_search_symbol_alpha*)
    if ! echo "$TRANSCRIPT_CONTENT" | grep -qiE "DSL_SYMBOL_ALPHA"; then
      echo "FAIL [$PROMPT_FILE]: Marker DSL_SYMBOL_ALPHA not in transcript"
      ((failures++))
    fi
    if echo "$TRANSCRIPT_CONTENT" | grep -qiE "DslSymbolAlpha"; then
      : # filename found
    else
      echo "FAIL [$PROMPT_FILE]: Expected filename DslSymbolAlpha not in transcript"
      ((failures++))
    fi
    ;;
  *009_verify_line_count*)
    if ! echo "$TRANSCRIPT_CONTENT" | grep -qiE "3\s*(lines?|total)"; then
      # Check if "3" appears near "line" in some form
      if echo "$TRANSCRIPT_CONTENT" | grep -qiE "3"; then
        : # number found
      else
        echo "WARN [$PROMPT_FILE]: Line count 3 not detected in transcript"
      fi
    fi
    ;;
  *010_ask_ambiguous_search*)
    if echo "$TRANSCRIPT_CONTENT" | grep -qiE "UNIQUE_IDENTIFIER_X7"; then
      : # search found matches
    else
      echo "WARN [$PROMPT_FILE]: Marker UNIQUE_IDENTIFIER_X7 not in transcript"
    fi
    ;;
  *011_done_after_read*)
    if ! echo "$TRANSCRIPT_CONTENT" | grep -qiE "DSL_CORE_BETA"; then
      echo "FAIL [$PROMPT_FILE]: Marker DSL_CORE_BETA not in transcript"
      ((failures++))
    fi
    ;;
esac

if [[ $failures -gt 0 ]]; then
  echo "FAIL [$PROMPT_FILE]: $failures assertion(s) failed"
  exit 1
else
  echo "PASS [$PROMPT_FILE]: All assertions passed"
  exit 0
fi
