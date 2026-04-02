user: Return a JSON object representing a file system tree with: "name" (string), "type" (file or directory), "children" (array of similar objects if directory), and "size" (number in bytes).

Output format:
{"name": "<STRING>", "type": "file|directory", "size": <NUMBER>, "children": [{"name": "...", "type": "...", ...}]}
