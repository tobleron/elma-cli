#!/usr/bin/env python3
import os
import re
import subprocess
import sys

# Architectural Truthfulness Audit (Task 780)
# This script syncs the Module Map in docs/ARCHITECTURE.md with actual wc -l counts.

def get_line_count(filepath):
    try:
        result = subprocess.run(['wc', '-l', filepath], capture_output=True, text=True)
        if result.returncode == 0:
            return result.stdout.strip().split()[0]
    except Exception:
        pass
    return None

def find_file(filename):
    # Direct path check
    if os.path.exists(os.path.join('src', filename)):
        return os.path.join('src', filename)
    if os.path.exists(filename):
        return filename
        
    # Recursive search in src
    for root, dirs, files in os.walk('src'):
        if filename in files:
            return os.path.join(root, filename)
    return None

def main():
    arch_file = 'docs/ARCHITECTURE.md'
    if not os.path.exists(arch_file):
        print(f"Error: {arch_file} not found")
        sys.exit(1)

    with open(arch_file, 'r') as f:
        lines = f.readlines()

    new_lines = []
    # Match | `file.rs` | 123 | ... |
    table_regex = re.compile(r'^\| `([^`]+)` \| ([^|]+) \| (.*)\|')
    
    updated_count = 0
    missing_count = 0

    for line in lines:
        match = table_regex.match(line.strip())
        if match:
            filename = match.group(1)
            # Skip directories
            if filename.endswith('/'):
                new_lines.append(line)
                continue
                
            filepath = find_file(filename)
            if filepath:
                count = get_line_count(filepath)
                if count:
                    new_line = f"| `{filename}` | {count} | {match.group(3)}|\n"
                    new_lines.append(new_line)
                    updated_count += 1
                else:
                    new_lines.append(line)
            else:
                # File not found - mark as MISSING
                new_line = f"| `{filename}` | MISSING | {match.group(3)}|\n"
                new_lines.append(new_line)
                missing_count += 1
        else:
            new_lines.append(line)

    with open(arch_file, 'w') as f:
        f.writelines(new_lines)
    
    print(f"Updated {updated_count} modules. {missing_count} modules missing.")
    if missing_count > 0:
        print("Warning: Some modules are listed as MISSING. Please audit ARCHITECTURE.md for deleted modules.")

if __name__ == '__main__':
    main()
