rg --type source --json --files-with-matches --no-heading --no-line-number _stress_testing/_opencode_for_testing/ | jq -r '.[].path' | sort -k1,1 -nr | head -3
