#!/bin/bash

# Find all files in config/ excluding config/defaults/
files=$(find config -type f -not -path "config/defaults/*")

for file in $files; do
    if [ -f "$file" ]; then
        echo "Updating $file..."
        # Handle pipe replacements with optional spaces and optional escaped quotes
        sed -i '' 's/"INVESTIGATE" *| *//g' "$file"
        sed -i '' 's/ *| *"INVESTIGATE"//g' "$file"
        sed -i '' 's/\\"INVESTIGATE\\" *| *//g' "$file"
        sed -i '' 's/ *| *\\"INVESTIGATE\\"//g' "$file"
        
        sed -i '' 's/"OPEN_ENDED" *| *//g' "$file"
        sed -i '' 's/ *| *"OPEN_ENDED"//g' "$file"
        sed -i '' 's/\\"OPEN_ENDED\\" *| *//g' "$file"
        sed -i '' 's/ *| *\\"OPEN_ENDED\\"//g' "$file"

        # Handle direct complexity replacements
        sed -i '' 's/"complexity":"INVESTIGATE"/"complexity":"DIRECT"/g' "$file"
        sed -i '' 's/"complexity":"OPEN_ENDED"/"complexity":"MULTISTEP"/g' "$file"
        sed -i '' 's/"complexity": "INVESTIGATE"/"complexity": "DIRECT"/g' "$file"
        sed -i '' 's/"complexity": "OPEN_ENDED"/"complexity": "MULTISTEP"/g' "$file"
        
        sed -i '' 's/\\"complexity\\":\\"INVESTIGATE\\"/\\"complexity\\":\\"DIRECT\\"/g' "$file"
        sed -i '' 's/\\"complexity\\":\\"OPEN_ENDED\\"/\\"complexity\\":\\"MULTISTEP\\"/g' "$file"

        # Handle line deletions for INVESTIGATE and OPEN_ENDED
        sed -i '' '/- INVESTIGATE/d' "$file"
        sed -i '' '/- OPEN_ENDED/d' "$file"
        sed -i '' '/= INVESTIGATE/d' "$file"
        sed -i '' '/= OPEN_ENDED/d' "$file"
        
        # Also handle cases where they might be used in descriptions or other places if it looks like stale config
        sed -i '' 's/INVESTIGATE | //g' "$file"
        sed -i '' 's/ | INVESTIGATE//g' "$file"
        sed -i '' 's/OPEN_ENDED | //g' "$file"
        sed -i '' 's/ | OPEN_ENDED//g' "$file"
        
        # Handle some more variations seen in README/Grammars
        sed -i '' 's/INVESTIGATE, //g' "$file"
        sed -i '' 's/, INVESTIGATE//g' "$file"
        sed -i '' 's/OPEN_ENDED, //g' "$file"
        sed -i '' 's/, OPEN_ENDED//g' "$file"
        
        # Specific grammar fix
        sed -i '' 's/\\"INVESTIGATE\\" | //g' "$file"
        sed -i '' 's/ | \\"INVESTIGATE\\"//g' "$file"
        sed -i '' 's/\\"OPEN_ENDED\\" | //g' "$file"
        sed -i '' 's/ | \\"OPEN_ENDED\\"//g' "$file"
    fi
done

echo "Batch update complete."
