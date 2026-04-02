user: Return a JSON Program object with "objective" (string) and "steps" array. Each step has "id" (string), "type" (shell or reply), "cmd" or "instructions", "purpose" (string), and "depends_on" (array of step ids).

Output format:
{"objective": "<STRING>", "steps": [{"id": "<STRING>", "type": "shell|reply", "cmd": "<STRING>", "purpose": "<STRING>", "depends_on": ["<STEP_ID>"]}]}
