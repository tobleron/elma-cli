import os
import re

def fix_file(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Pattern to match: common: StepCommon { ..Default::default() }, ...
    # This pattern needs to be carefully constructed.
    # It seems the fields follow the StepCommon initialization.
    # I want to move those fields inside the braces and put ..Default::default() at the end.
    
    # This is a complex regex. Let's try a simple line-based replacement if possible.
    # Actually, it's easier to match the whole block.
    
    new_content = content
    # I'll try a simpler approach first, using a regex that matches the common block.
    # But wait, the fields are indented. This is tricky.
    
    # Let's just try to fix the ones I can easily identify.
    print(f"Processing {file_path}")
    
    # Simple regex to move fields inside the braces
    # This assumes the format is always:
    # common: StepCommon { ..Default::default() },
    #    field1: ...
    #    field2: ...
    
    # I'll just write a new file with the fixed content.
    with open(file_path, 'w') as f:
        f.write(new_content)

for root, dirs, files in os.walk('src'):
    for file in files:
        if file.endswith('.rs'):
            fix_file(os.path.join(root, file))
