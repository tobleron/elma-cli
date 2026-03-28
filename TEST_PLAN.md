# Elma Formula Orchestration Test Plan (Workspace-Only)

This plan validates that Elma can orchestrate formulas (intel units) to produce real workspace results
without internet access.

## Preconditions
- llama.cpp endpoint reachable at `LLAMA_BASE_URL` (default: `http://192.168.1.186:8080`)
- Run in a writable workspace directory (this repo)

## Tests
1. ACTION->SHELL Formula (Tooler + Executor)
   - User input: "list files in current directory"
   - Expected:
     - tool JSON printed (`tool> {...}`)
     - command executed and output printed
     - `sessions/<id>/shell/001.sh` and `001.out` created

2. ACTION->PLAN Formula (Planner)
   - User input: "create a plan to add a new config file"
   - Expected:
     - `sessions/<id>/plans/plan_001.md` created
     - `_master.md` updated with a checkbox link to `plan_001.md`

3. Safety: Block Network/Remote Commands
   - User input: "curl http://example.com"
   - Expected:
     - tool command is blocked by policy (no execution)
