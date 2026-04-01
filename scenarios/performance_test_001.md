# Elma Performance Test Scenarios

## Purpose
Test Elma's performance after all improvements:
- Angel Helper intention clarification
- Formula patterns (abstract, no hardcoded commands)
- Formula scoring (efficiency optimization)
- Tool discovery + caching
- Read/Search step types
- Hierarchical decomposition for OPEN_ENDED

## Scenarios

### Scenario 1: Simple Greeting (DIRECT, CHAT)
**Expected:** reply_only formula, no inspection, fast response

```
User: "hello"
Expected:
- Angel: "CHAT: greet the user"
- Formula: reply_only (efficiency: 3.0)
- Steps: 1 (reply only)
- Time: < 5s
```

---

### Scenario 2: List Files (DIRECT, SHELL)
**Expected:** inspect_reply formula, shell command, no over-inspection

```
User: "ls -ltr"
Expected:
- Angel: "ACTION: execute shell command ls -ltr"
- Formula: inspect_reply (efficiency: 2.0)
- Steps: 2 (shell + reply)
- Time: < 10s
- NO reflection cycles (simple command)
```

---

### Scenario 3: Find File Content (INVESTIGATE)
**Expected:** inspect_reply formula, read step type

```
User: "Show me what's in Cargo.toml"
Expected:
- Angel: "ACTION: read Cargo.toml and show contents"
- Formula: inspect_reply (efficiency: 2.0)
- Steps: 2 (read + reply)
- Uses: read step type (not shell cat)
- Time: < 10s
```

---

### Scenario 4: Project Structure (INVESTIGATE, MEDIUM)
**Expected:** inspect_summarize_reply formula, workspace_tree tool

```
User: "What's the project structure?"
Expected:
- Angel: "ACTION: show project structure"
- Formula: inspect_summarize_reply (efficiency: 1.75)
- Steps: 3 (inspect + summarize + reply)
- Uses: workspace_tree tool
- Time: < 15s
```

---

### Scenario 5: Code Search (INVESTIGATE)
**Expected:** inspect_reply formula, search step type

```
User: "Where is the main function defined?"
Expected:
- Angel: "ACTION: search for main function"
- Formula: inspect_reply (efficiency: 2.0)
- Steps: 2 (search + reply)
- Uses: search step type (rg)
- Time: < 10s
```

---

### Scenario 6: Multi-Step Task (MULTISTEP)
**Expected:** plan_reply or inspect_edit_verify_reply formula

```
User: "Add a new function to src/main.rs that prints hello"
Expected:
- Angel: "ACTION: edit src/main.rs to add function"
- Formula: inspect_edit_verify_reply (efficiency: 1.29)
- Steps: 4 (read + edit + verify + reply)
- Uses: read + edit step types
- Time: < 30s
```

---

### Scenario 7: OPEN_ENDED Task (Hierarchical Decomposition)
**Expected:** masterplan generated first, NO massive single command

```
User: "Analyze the entire codebase and provide a comprehensive summary"
Expected:
- Complexity: OPEN_ENDED
- Decomposition: TRIGGERED (depth=5)
- Masterplan: Generated with 3-5 phases
- Saved to: sessions/<id>/masterplans/plan_<timestamp>.json
- NO massive commands like "find | xargs cat"
- Phase 1 executed first
- Time: Phase 1 < 15s
```

---

## Success Criteria

| Metric | Target | Pass |
|--------|--------|------|
| Simple tasks use reply_only | 100% | ☐ |
| No over-inspection on DIRECT tasks | 100% | ☐ |
| Read/Search step types used | 100% | ☐ |
| workspace_tree for structure questions | 100% | ☐ |
| OPEN_ENDED triggers masterplan | 100% | ☐ |
| No massive single-step commands | 100% | ☐ |
| Formula efficiency matches selection | 90%+ | ☐ |
| Average response time < 15s | 90%+ | ☐ |

---

## Run Instructions

```bash
# Start Elma
cd /Users/r2/elma-cli
cargo run

# Test each scenario
# Check session trace for:
# - Angel response
# - Formula selected
# - Steps generated
# - Tools used
# - Execution time
```
