#!/bin/bash
# Stop Ollama for doccura project

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PID_FILE="$PROJECT_DIR/.ollama.pid"

if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        rm "$PID_FILE"
        echo "Ollama stopped"
    else
        echo "Ollama process not found"
        rm "$PID_FILE"
    fi
else
    # Try to kill any ollama process
    if pkill -x ollama 2>/dev/null; then
        echo "Ollama stopped"
    else
        echo "Ollama is not running"
    fi
fi

