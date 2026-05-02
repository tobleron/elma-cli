# Task 267: Optional API-Based Sub-Agent Delegation (POSTPONED)

## Backlog Reconciliation (2026-05-02)

Superseded by Task 492. Keep delegation bounded, optional, and dependent on local/headless infrastructure first.


## Status: POSTPONED (Optional Feature - Disabled by Default)

## Priority: LOW

## Context
This task was originally designed for Claude-Code-style sub-agent delegation using git worktrees. However, Elma-CLI is a local-LLM-only system, making multi-agent delegation impractical since users typically run only one LLM locally.

## Revised Approach
Enable optional API-based sub-agent delegation for users who have access to external LLM APIs (OpenAI, Anthropic, Ollama with API server, etc.). This is an opt-in feature disabled by default.

## Problem Statement
Elma lacks optional sub-agent delegation for complex tasks that could benefit from parallel processing or isolation. When users have API access to external LLMs, they may want to:
- Run complex refactoring in an isolated git worktree
- Use a second model for different capabilities
- Delegate heavy tasks to a more powerful API model while keeping local model for lightweight work

## Analysis from Claude-Code
- `AgentTool` supports "isolation" modes like `worktree`
- Creates temporary git worktree for isolated repository copy
- Sub-agent works on isolated copy, main workspace unaffected
- Supports `fork` and `resume` for long-running tasks

## Solution Architecture
1. **Feature Flag**: Add `enable_api_sub_agents = false` to config (default off)
2. **API Configuration**: Allow users to configure external LLM endpoint (API key, base URL, model name)
3. **Sub-Agent Framework**: Create `src/sub_agent.rs` for delegation
4. **Git Worktree Integration**: Use git worktrees for isolation when using API models
5. **Fork/Resume**: Implement continuation mechanisms
6. **Coordination**: Integrate with main orchestration loop

## Implementation Steps
1. Add config section for API sub-agent settings
2. Create SubAgent struct with API client management
3. Implement git worktree creation/cleanup for isolation
4. Add fork/resume capabilities
5. Wire feature flag checks - feature only active when enabled
6. Add clear error message when feature disabled
7. Implement result aggregation from sub-agents

## Integration Points
- `config/`: Add API sub-agent configuration (disabled by default)
- `src/sub_agent.rs`: New module for API-based sub-agent management
- `src/skills.rs`: Extend skill delegation (opt-in)
- `src/orchestration_loop.rs`: Coordinate sub-agent execution
- `src/session.rs`: Manage sub-agent sessions

## Success Criteria
- Feature disabled by default, no impact on local-only users
- When enabled, uses API LLM for sub-agents (not local model)
- Sub-agents can work in isolated git worktrees
- Fork/resume functionality works
- Main workspace protected from sub-agent changes
- Clear messaging when feature not configured
- `cargo build` passes

## Files to Create/Modify
- `config/defaults.toml` or similar (add API sub-agent config section)
- `src/sub_agent.rs` (new - API client wrapper)
- `src/skills.rs` (modify - add delegation option)
- `src/orchestration_loop.rs` (modify - coordinate sub-agents)
- `src/session.rs` (modify - manage sub-agent sessions)

## Risk Assessment
- LOW: Disabled by default, no impact on existing users
- MEDIUM: Git worktree management complexity when enabled
- Need careful cleanup of temporary worktrees
- API key security considerations
- Testing required for git operations and API integration

## Example Configuration (when enabled)
```toml
[api_sub_agent]
enabled = false  # Default: disabled

# When enabled:
enabled = true
provider = "openai"  # or "anthropic", "ollama", "custom"
api_key = "${OPENAI_API_KEY}"  # or env var
base_url = "https://api.openai.com/v1"  # for custom/ollama
model = "gpt-4o-mini"  # cheap model for delegation
worktree_dir = "./_dev-system/sub-agent-worktrees"
```