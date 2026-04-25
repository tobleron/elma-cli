# Task Consolidation Analysis

## Duplicate Tasks Found

### 1. Pub/Sub (Merged into one)
- **Task 150**: Service Layer With PubSub - OpenCode pattern
- **Task 159**: Generic PubSub Broker - Crush pattern
- **DECISION**: Keep only Task 159 (more complete implementation)
- **Action**: Archive Task 150

### 2. Checkpoint/Recovery (Different approaches - Keep both)
- **Task 143**: AutoSave Checkpoint Recovery - JSON-based state saves
- **Task 164**: Enhanced Checkpoint Manager - Shadow git repos
- **DECISION**: Keep both - solve different problems

## NOT Suitable for Small Local Models (3B/4B)

These tasks require capabilities beyond small local models:
- Task 161: Self-Learning from Errors (requires RL training)
- Task 162: Delegate Subagent (complex reasoning) 
- Task 165: Hermes Additional (MoA, trajectory compression)
- Most "LLM-based" features
- Context compression (requires LLM summarization)

**DECISION**: Postpone these tasks

## Priority Order for Small Model CLI

### Tier 1: Critical Foundation (Must have)
1. SQLite Persistence (149) - Core data storage
2. Mode System (134-135) - Core routing
3. Task Lifecycle with Events (136) - Reliability
4. Tool Refactor (151) - Better tool interface

### Tier 2: Important Capabilities
5. AutoSave Checkpoint (143) - Crash recovery
6. UpdateTodoList Tool (138) - Task management
7. AttemptCompletion Tool (139) - Task completion
8. MCP Hub (142) - Extendability

### Tier 3: UI Improvements
9. Typeahead (144) - Input experience
10. Hints System (145) - Help users
11. Input Prefixes (146) - Command modes
12. Keyboard Shortcuts (147) - Productivity
13. Chat Undo (148) - History

### Tier 4: Reliability
14. Permission System (152) - Safety
15. Graceful Shutdown (153) - Stability
16. Retry with Backoff (154) - Resilience
17. PubSub Broker (159) - Decoupling

### Tier 5: Extended Features
18. Skills Framework (156) - Extensibility
19. File Tracker (158) - Context awareness
20. Memory Tool (163) - Learning
21. Background Jobs (160) - Dev workflow
22. SwitchMode (140) - Mode switching
23. ExecuteCommand IO (141) - Better shell

### Tier 6: Postponed (Need larger models)
24. Self-Learning (161)
25. Delegate Subagent (162)
26. Hermes Additional (165)
27. Context Compression (114)

## New Numbering

001: SQLite Persistence (149 → 001)
002: Mode System Types (134 → 002)  
003: Mode Manager (135 → 003)
004: Task Lifecycle (136 → 004)
005: Tool Interface (151 → 005)
006: AutoSave Checkpoint (143 → 006)
007: UpdateTodoList (138 → 007)
008: AttemptCompletion (139 → 008)
009: MCP Hub (142 → 009)
010: Typeahead (144 → 010)
011: Hints System (145 → 011)
012: Input Prefixes (146 → 012)
013: Keyboard Shortcuts (147 → 013)
014: Chat Undo (148 → 014)
015: Permission System (152 → 015)
016: Graceful Shutdown (153 → 016)
017: Retry Backoff (154 → 017)
018: PubSub Broker (159 → 018)
019: Skills Framework (156 → 019)
020: File Tracker (158 → 020)
021: Memory Tool (163 → 021)
022: Background Jobs (160 → 022)
023: SwitchMode (140 → 023)
024: ExecuteCommand IO (141 → 024)
025: Task Persistence (137 → 025)
026: LSP Client (155 → 026)
027: Shadow Git Checkpoints (164 → 027)

## Postponed (large model only)
- 161 Self-Learning
- 162 Delegate Subagent
- 165 Hermes Additional Features
- 114 Auto-Compact Context (needs LLM)
- 093 Hybrid MasterPlan (needs large model)