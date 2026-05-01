#!/bin/bash
# Workspace Scanner - Idempotent automation for source file analysis
set -euo pipefail

OUTPUT_DIR="${1:-project_tmp}"
LOG_FILE="${OUTPUT_DIR}/scan_log.txt"
REPORT_FILE="${OUTPUT_DIR}/scan_report.txt"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

mkdir -p "$OUTPUT_DIR"

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting workspace scan..." >> "$LOG_FILE"
echo "Output directory: $OUTPUT_DIR" >> "$LOG_FILE"
echo ""

# Find all source files (excluding .git, target, sessions)
FILES=$(find . \( -name '*.py' -o -name '*.js' -o -name '*.ts' -o -name '*.sh' -o -name '*.bash' -o -name '*.rb' -o -name '*.go' -o -name '*.rs' -o -name '*.java' -o -name '*.c' -o -name '*.cpp' -o -name '*.h' -o -name '*.hpp' \) \
    ! -path '*/.git/*' ! -path '*/target/*' ! -path '*/sessions/*' ! -path '*/.opencode/*' \
    -type f 2>/dev/null | sort)

TOTAL_FILES=$(echo "$FILES" | grep -c . || echo "0")
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Found $TOTAL_FILES source files" >> "$LOG_FILE"
echo "Found $TOTAL_FILES source files"
echo ""

# Initialize counters
TOTAL_LINES=0
TODO_COUNT=0
FIXME_COUNT=0
LARGE_FILES=0

echo "$FILES" | while IFS= read -r filepath; do
    [ -z "$filepath" ] && continue
    size=$(stat -f%z "$filepath" 2>/dev/null || stat -c%s "$filepath")
    lines=$(wc -l < "$filepath" 2>/dev/null | tr -d ' \t\n' || echo "0")
    
    ext="${filepath##*.}"
    TOTAL_LINES=$((TOTAL_LINES + lines))
    
    todos=$(grep -c 'TODO' "$filepath" 2>/dev/null | tr -d ' \t\n' || echo "0")
    fixmes=$(grep -c 'FIXME' "$filepath" 2>/dev/null | tr -d ' \t\n' || echo "0")
    TODO_COUNT=$((TODO_COUNT + todos))
    FIXME_COUNT=$((FIXME_COUNT + fixmes))
    
    if (( size > 1048576 )); then
        LARGE_FILES=$((LARGE_FILES + 1))
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] [LARGE] $filepath (${size} bytes)" >> "$LOG_FILE"
    fi
done

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Total lines: $TOTAL_LINES" >> "$LOG_FILE"
echo ""

# Print summary to console
echo -e "${BLUE}Files by Type:${NC}"
echo "$FILES" | while IFS= read -r filepath; do
    [ -z "$filepath" ] && continue
    ext="${filepath##*.}"
    echo "  $ext: $(grep -c ".$ext\$" <<< "$filepath" | tr -d ' \t\n')"
done | sort -t: -k2 -rn
echo ""

echo -e "${YELLOW}TODO/FIXME Summary:${NC}"
echo "  Total TODOs:    $TODO_COUNT"
echo "  Total FIXMEs:   $FIXME_COUNT"
echo ""

echo -e "${RED}Large Files (>1MB): ${LARGE_FILES}${NC}"
if (( LARGE_FILES > 0 )); then
    echo ""
    echo -e "${BLUE}Top 10 Largest Files:${NC}"
    echo "$FILES" | while IFS= read -r filepath; do
        [ -z "$filepath" ] && continue
        size=$(stat -f%z "$filepath" 2>/dev/null || stat -c%s "$filepath")
        if (( size > 1048576 )); then
            printf "  %-50s %d bytes\n" "$filepath" "$size"
        fi
    done | sort -t' ' -k2 -rn | head -10
echo ""
fi

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Scan complete!" >> "$LOG_FILE"
echo "Report saved to: $REPORT_FILE"
echo "Log saved to: $LOG_FILE"
echo -e "${GREEN}✓ Scan completed successfully${NC}"
