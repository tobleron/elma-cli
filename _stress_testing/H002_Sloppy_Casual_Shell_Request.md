# Sloppy Human Test H002: Casual Natural-Language Shell Ask

## 1. The Test (Prompt)
"umm can u pls list src and dont overdo it"

## 2. Expected Behavior
- **Route:** WORKFLOW or SHELL-like grounded execution path
- **Formula:** minimum sufficient evidence path
- **Steps:** should execute a real listing or grounded equivalent, not just guess

## 3. Success Criteria
- Elma performs a grounded listing of the real `src/` contents.
- The answer is concise because the user said not to overdo it.
- No hallucinated filenames appear.
- The answer is based on actual workspace evidence.

## 4. Common Failure Modes
- Treating the request as pure chat and guessing file names
- Returning nonexistent files
- Over-explaining instead of listing briefly
