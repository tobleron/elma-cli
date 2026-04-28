#!/usr/bin/env bash
#
# Workspace Scanner - Idempotent automation for source file analysis
# Outputs: summary_report.txt, scan_log.txt in project_tmp/
#
set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_TMP="$SCRIPT_DIR"
REPORT="$PROJECT_TMP/summary_report.txt"
LOG="$PROJECT_TMP/scan_log.txt"
TMPDIR="${TMPDIR:-/tmp}/elma-scan-$$"
mkdir -p "$TMPDIR"
trap 'rm -rf "$TMPDIR"' EXIT

# Thresholds
SIZE_THRESHOLD_BYTES=1048576  # 1MB

echo "[status] Starting workspace scan at $(date)" >&2

# Define extensions to scan
EXTENSIONS=(py js sh rs ts toml md json txt yml yaml)

# Build exclude pattern for macOS find
EXCLUDE_PATTERN="-not -path '*/.git/*' -not -path '*/target/*' -not -path '*/node_modules/*' -not -path '*/.opencode/node_modules/*' -not -path '*/_knowledge_base/_source_code_agents/*' -not -path '*/_dev-system/*'"

# Step 1: Find all source files efficiently using find with null delimiter
echo "[status] Scanning for source files..." >&2
> "$TMPDIR/all_files.txt"
for ext in "${EXTENSIONS[@]}"; do
    # Use -print0 and read -d '' for safe handling of filenames with spaces/newlines
    find "$SCRIPT_DIR/.." -type f -name "*.$ext" $EXCLUDE_PATTERN -print0 2>/dev/null >> "$TMPDIR/all_files.txt"
done

# Sort null-delimited list
sort -z "$TMPDIR/all_files.txt" -o "$TMPDIR/all_files.txt"

file_count=$(wc -l < "$TMPDIR/all_files.txt")
if [ "$file_count" -eq 0 ]; then
    echo "[error] No source files found." >&2
    cat > "$REPORT" << EOF
=== Workspace Scan Summary ===
Scanned: $(date)
Source files found: 0

--- Counts by Type ---
(no files found)

--- TODO/FIXME Comments ---
(0 entries)

--- Files >1MB ---
(0 files)
EOF
    cat > "$LOG" << EOF
=== Scan Log ===
Timestamp: $(date)
Total files scanned: 0
TODO/FIXME count: 0
Large file count: 0
(no data)
EOF
    echo "[status] No source files found. Done." >&2
    exit 0
fi

echo "[status] Found $file_count source files" >&2

# Step 2: Count files by type
echo "[status] Counting by type..." >&2
total=0
> "$TMPDIR/counts.txt"
for ext in "${EXTENSIONS[@]}"; do
    count=$(grep -c "\.$ext$" "$TMPDIR/all_files.txt" 2>/dev/null || echo 0)
    if [ "$count" -gt 0 ]; then
        printf "%-6s %d\n" ".$ext" "$count" >> "$TMPDIR/counts.txt"
        total=$((total + count))
    fi
done

# Step 3: Extract TODO/FIXME comments
echo "[status] Extracting TODO/FIXME comments..." >&2
> "$TMPDIR/todos.txt"
while IFS= read -r -d '' filepath; do
    if [ -r "$filepath" ]; then
        # Use grep -in for line numbers and case-insensitive matching
        grep -inE 'TODO|FIXME|HACK|XXX' "$filepath" 2>/dev/null | while IFS=: read -r linenum content; do
            rel="${filepath#$SCRIPT_DIR/../}"
            printf "%s:%d: %s\n" "$rel" "$linenum" "$content" >> "$TMPDIR/todos.txt"
        done || true
    fi
done < "$TMPDIR/all_files.txt"

todo_count=$(wc -l < "$TMPDIR/todos.txt")
echo "[status] Found $todo_count TODO/FIXME entries" >&2

# Step 4: Identify files larger than 1MB
echo "[status] Identifying files >1MB..." >&2
> "$TMPDIR/large_files.txt"
while IFS= read -r -d '' filepath; do
    if [ -f "$filepath" ] && [ -r "$filepath" ]; then
        # macOS: stat -f%z gives size in bytes
        size=$(stat -f%z "$filepath" 2>/dev/null || echo 0)
        if [ "$size" -gt "$SIZE_THRESHOLD_BYTES" ]; then
            rel="${filepath#$SCRIPT_DIR/../}"
            # Calculate MB with 2 decimal places using awk (more portable than bc)
            size_mb=$(awk "BEGIN {printf \"%.2f\", $size / 1048576}")
            printf "%-8s MB  %s\n" "$size_mb" "$rel" >> "$TMPDIR/large_files.txt"
        fi
    fi
done < "$TMPDIR/all_files.txt"

large_count=$(wc -l < "$TMPDIR/large_files.txt")
echo "[status] Found $large_count files >1MB" >&2

# Step 5: Write outputs
echo "[status] Writing outputs..." >&2

{
    echo "=== Workspace Scan Summary ==="
    echo "Scanned: $(date)"
    echo "Source files found: $total"
    echo ""
    echo "--- Counts by Type ---"
    cat "$TMPDIR/counts.txt"
    echo ""
    echo "--- TODO/FIXME Comments ---"
    echo "$todo_count entries found"
    if [ "$todo_count" -gt 0 ]; then
        echo "(see scan_log.txt for details)"
    fi
    echo ""
    echo "--- Files >1MB ---"
    echo "$large_count files found"
    if [ "$large_count" -gt 0 ]; then
        cat "$TMPDIR/large_files.txt"
    fi
} > "$REPORT"

{
    echo "=== Scan Log ==="
    echo "Timestamp: $(date)"
    echo "Total files scanned: $total"
    echo "TODO/FIXME count: $todo_count"
    echo "Large file count: $large_count"
    echo ""
    echo "--- TODO/FIXME Entries ---"
    if [ "$todo_count" -gt 0 ]; then
        cat "$TMPDIR/todos.txt"
    else
        echo "(none)"
    fi
} > "$LOG"

echo "[status] Done. Report: $REPORT" >&2
echo "[status] Log: $LOG" >&2
