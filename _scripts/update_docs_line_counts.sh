#!/bin/bash
# Documentation Line Count Audit Tool

# Files to audit
CORE_FILES=(
    "src/app_chat_loop.rs"
    "src/tool_loop/mod.rs"
    "src/tool_calling.rs"
    "src/orchestration_retry.rs"
    "src/orchestration_planning.rs"
    "src/evidence_ledger.rs"
    "src/document_adapter.rs"
    "src/stop_policy.rs"
)

echo "--- Architectural Metrics Audit ---"
printf "%-30s %-10s %-10s\n" "File" "Actual" "Status"
printf "%-30s %-10s %-10s\n" "------------------------------" "----------" "----------"

EXIT_CODE=0

for file in "${CORE_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        printf "%-30s %-10s %-10s\n" "$file" "MISSING" "ERROR"
        EXIT_CODE=1
        continue
    fi

    ACTUAL=$(wc -l < "$file" | tr -d ' ')
    
    # Check if ARCHITECTURE.md contains a stale count
    # Look for | `file` | COUNT |
    BASENAME=$(basename "$file")
    DIRNAME=$(dirname "$file")
    if [ "$DIRNAME" == "src/tool_loop" ]; then
        SEARCH_KEY="tool_loop/$BASENAME"
    else
        SEARCH_KEY="$BASENAME"
    fi

    DOC_COUNT=$(grep "| \`$SEARCH_KEY\` |" docs/ARCHITECTURE.md | cut -d'|' -f3 | tr -d ' ' | tr -d '[:alpha:]' | tr -d '(' | tr -d ')')
    
    if [ -n "$DOC_COUNT" ]; then
        DIFF=$((ACTUAL - DOC_COUNT))
        ABS_DIFF=${DIFF#-}
        THRESHOLD=100
        
        if [ "$ABS_DIFF" -gt "$THRESHOLD" ]; then
            printf "%-30s %-10s %-10s (Doc says %s, diff %s)\n" "$SEARCH_KEY" "$ACTUAL" "STALE" "$DOC_COUNT" "$DIFF"
            EXIT_CODE=1
        else
            printf "%-30s %-10s %-10s\n" "$SEARCH_KEY" "$ACTUAL" "OK"
        fi
    else
        printf "%-30s %-10s %-10s\n" "$SEARCH_KEY" "$ACTUAL" "NOT_DOC"
    fi
done

exit $EXIT_CODE
