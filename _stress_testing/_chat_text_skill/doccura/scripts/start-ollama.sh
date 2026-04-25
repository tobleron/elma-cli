#!/bin/bash
# Start Ollama in background for doccura project

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PID_FILE="$PROJECT_DIR/.ollama.pid"
LOG_FILE="$PROJECT_DIR/.ollama.log"

# Check if Ollama is already running
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        echo "Ollama is already running (PID: $PID)"
        exit 0
    else
        # Stale PID file
        rm "$PID_FILE"
    fi
fi

# Check if ollama process is running
if pgrep -x ollama > /dev/null; then
    echo "Ollama is already running"
    exit 0
fi

# Start Ollama in background
nohup ollama serve > "$LOG_FILE" 2>&1 &
echo $! > "$PID_FILE"

echo "Ollama started in background (PID: $(cat "$PID_FILE"))"
echo "Logs: $LOG_FILE"

