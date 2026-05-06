#!/bin/bash

set -e

ENDPOINT="http://192.168.1.186:8080/v1"
CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/crush/crush.json"

echo "Fetching model info from $ENDPOINT..."

response=$(curl -s "$ENDPOINT/models")
if [ -z "$response" ]; then
    echo "Error: Failed to fetch model info from $ENDPOINT"
    exit 1
fi

model_name=$(echo "$response" | jq -r '.data[0].id')
context_window=$(echo "$response" | jq -r '.data[0].meta.n_ctx_train // 131072')

if [ -z "$model_name" ] || [ "$model_name" = "null" ]; then
    echo "Error: Could not extract model name from response"
    exit 1
fi

echo "Detected model: $model_name"
echo "Context window: $context_window"

backup_file="${CONFIG_FILE}.bak"
cp "$CONFIG_FILE" "$backup_file" 2>/dev/null || true

cat > "$CONFIG_FILE" << EOF
{
  "$schema": "https://charm.land/crush.json",
  "models": {
    "large": {
      "provider": "local",
      "model": "$model_name"
    },
    "small": {
      "provider": "local",
      "model": "$model_name"
    }
  },
  "providers": {
    "local": {
      "id": "local",
      "name": "Local LLM",
      "type": "openai-compat",
      "base_url": "$ENDPOINT",
      "api_key": "",
      "models": [
        {
          "id": "$model_name",
          "name": "$model_name",
          "context_window": $context_window
        }
      ]
    }
  }
}
EOF

echo "Updated $CONFIG_FILE with model: $model_name"

exec crush