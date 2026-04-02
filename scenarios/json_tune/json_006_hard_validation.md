user: Return a JSON Schema object that defines a simple user record with: "$schema" (string), "type" (object), "properties" (object with "name", "email", "age" definitions), "required" (array), and "additionalProperties" (boolean).

Output format:
{"$schema": "<STRING>", "type": "object", "properties": {"name": {"type": "string"}, "email": {"type": "string"}, "age": {"type": "number"}}, "required": ["<FIELD>", ...], "additionalProperties": <BOOL>}
