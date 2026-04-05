# JSON Temperature Tuning System

> **See [ARCHITECTURE.md](../ARCHITECTURE/ARCHITECTURE.md)** for comprehensive documentation including design philosophy, GBNF grammar integration, and full implementation details.


## 📚 Related Documentation

- **[Architecture Reference](../ARCHITECTURE/ARCHITECTURE.md)**
- **[Task Management System](../PLANNING_AND_TASKS/TASKS.md)**
- **[Roadmap](../PLANNING_AND_TASKS/REPRIORITIZED_ROADMAP.md)**


The JSON Temperature Tuning system automatically finds the optimal temperature for reliable JSON output from the model. This is **Stage 0** (highest priority) in the tuning pipeline, ensuring JSON reliability before any other tuning occurs.

---

## How It Works

### Temperature Grid Search
- Tests temperatures from **0.0 to 1.0** (step 0.1 = 11 test points)
- Each temperature is tested against **6 JSON scenarios** across 3 difficulty levels
- Total: 66 model API calls per tuning session

### Difficulty Levels

**LOW (weight: 1.0)**
- Simple two-field objects
- Arrays of basic objects

**MEDIUM (weight: 2.0)**
- Nested recursive structures
- Elma Program JSON format

**HARD (weight: 3.0)**
- Complex multi-section configurations
- JSON Schema definitions

### Scoring System

For each temperature, the system measures:
1. **valid_json_count**: JSON that parses successfully
2. **repairable_count**: JSON that becomes valid after jsonrepair
3. **failed_count**: JSON that cannot be repaired

**Weighted Score** = base_accuracy + repair_bonus
- `base_accuracy = valid / total`
- `repair_bonus = (repairable / valid) * 0.1`

### Temperature Selection Strategy

1. **Find optimal**: Temperature with highest weighted score
2. **Apply preference**: If multiple temps score within 0.05 of optimal, prefer **lower temperature** (more deterministic)
3. **Threshold**: If best score < 0.9, the model may not be suitable for JSON tasks

## Integration Points

### Stage 0 in Tuning Pipeline
```
Stage 0/5: JSON Temperature Tuning ← NEW (Priority 0)
Stage 1/5: Protected Baselines
Stage 2/5: Routing Parameters
Stage 3/5: Workflow Orchestration
Stage 4/5: Response Quality
Stage 5/5: Final Validation
```

### Profiles Updated
After finding optimal temperature, it's applied to:
- `orchestrator.toml`
- `workflow_planner.toml`
- `json_outputter.toml`

## Files

### New Files
- `src/json_tuning.rs` - Main tuning logic
- `scenarios/json_tune/manifest.toml` - Scenario definitions
- `scenarios/json_tune/json_*.md` - 6 test scenarios

### Modified Files
- `src/main.rs` - Added json_tuning module
- `src/optimization_tune.rs` - Added Stage 0 integration

## Output

### Console Output
```
tune stage 0/5: JSON temperature tuning for llama-3.2-3b-instruct
  temp=0.0: valid=6/6, repairable=6, failed=0, score=1.000
  temp=0.1: valid=6/6, repairable=6, failed=0, score=1.000
  temp=0.2: valid=5/6, repairable=6, failed=0, score=0.933
  ...
  JSON tuning complete: optimal_temp=0.10, recommended_temp=0.10, score=1.000
```

### Report File
Saved to: `config/<model>/tune/json/json_tuning_<timestamp>.toml`

```toml
# JSON Temperature Tuning Report
timestamp = 1774905600
optimal_temperature = 0.10
recommended_temperature = 0.10

# Results by Temperature
[[temperatures]]
  { temperature = 0.0, valid = 6, total = 6, repairable = 6, failed = 0, score = 1.000 }
  { temperature = 0.1, valid = 6, total = 6, repairable = 6, failed = 0, score = 1.000 }
  ...

# Results by Difficulty
[[difficulties]]
  { difficulty = "LOW", best_score = 1.000 }
  { difficulty = "MEDIUM", best_score = 1.000 }
  { difficulty = "HARD", best_score = 0.833 }
```

## Relationship to Helper/Intel Unit Tuning

See [ARCHITECTURE.md](./ARCHITECTURE.md#json-reliability-pipeline) for the complete multi-layer protection pipeline.

### Key Difference: Adaptive vs. Grid Search

**JSON Tuning (Stage 0)**: Always does full grid search
- Tests ALL 11 temperatures (0.0 to 1.0)
- Reason: JSON reliability is critical and temperature-sensitive
- Cannot skip: Need to find globally optimal temperature

**Helper/Intel Unit Tuning (Stages 2-4)**: Adaptive early-exit
- Tests default temperature FIRST
- **If default passes threshold → skip other temperatures**
- **If default fails → test alternatives**

### Quick Mode Optimization

In `--tune-mode quick`:

| Stage | Default Tested | Skip Condition | Threshold |
|-------|---------------|----------------|-----------|
| Stage 2: Routing | `router_soft` | score ≥ 0.85, not hard-rejected | 0.85 |
| Stage 3: Orchestration | `orch_balanced` | score ≥ 0.80, not hard-rejected | 0.80 |
| Stage 4: Response | `response_balanced` | score ≥ 0.80, not hard-rejected | 0.80 |

**Example Flow (Quick Mode)**:
```
Stage 2: Routing
  → Test router_soft (default-like)
  → Score = 0.92, passed ✓
  → Skip router_strict (saves 1 evaluation)

Stage 3: Orchestration  
  → Test orch_balanced (default)
  → Score = 0.75, below threshold
  → Test orch_conservative
  → Score = 0.82, passed ✓
  → Skip orch_creative (saves 1 evaluation)
```

### Why Different Approaches?

1. **JSON is foundational**: Bad JSON breaks everything downstream
2. **JSON is temperature-sensitive**: Small temp changes affect JSON quality significantly
3. **Helpers are robust**: Can tolerate more temperature variation
4. **Speed vs. thoroughness**: Quick mode prioritizes speed for helpers, not JSON

### Full Mode Behavior

In full tuning mode (`--tune` without `--tune-mode`):
- All variants are tested (no early exit)
- Comprehensive search for optimal configuration
- Slower but more thorough

The JSON tuning system works **in conjunction** with the JSON repair pipeline:

1. **Prevention (Tuning)**: Find temperature that minimizes malformed JSON
2. **Detection (Parsing)**: Catch repetition loops and length violations
3. **Repair (fix_orphaned_keys)**: Fix common structural errors
4. **Fallback (jsonrepair-rs)**: General JSON repair library
5. **Graceful Degradation**: CHAT route fallback when all else fails

### Expected Flow
```
Model Output → Length Check → Repetition Check → 
Orphaned Keys Fix → jsonrepair-rs → Parse Error (with preview)
```

## Usage

### Automatic (Startup)
When `--tune` or `--calibrate` is used, JSON tuning runs automatically as Stage 0.

### Manual
```bash
# Full tuning (includes JSON tuning)
./elma-cli --tune

# Quick tuning (includes JSON tuning)
./elma-cli --tune --tune-mode quick
```

## Future Enhancements

1. **Adaptive Re-tuning**: Re-run JSON tuning if parse errors exceed threshold during normal operation
2. **Per-Difficulty Temperatures**: Use different temperatures for different task complexities
3. **Model-Specific Profiles**: Store and reuse optimal temperatures per model
4. **Continuous Learning**: Update temperature based on runtime JSON success rate

---

## 📚 Quick Links

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Comprehensive reference documentation
- **[TASKS.md](./TASKS.md)** - Complete task list by pillar
- **[IMPLEMENTATION_NOTES.md](./IMPLEMENTATION_NOTES.md)** - Recent progress & troubleshooting
- **[INTEL_UNIT_STANDARD.md](./INTEL_UNIT_STANDARD.md)** - Intel unit output format standard

---

## 🚀 Essential Commands

### Development
```bash
cargo build
cargo run -- [args]
cargo test
cargo fmt
```

### Testing & Probing
```bash
# Run unit tests
cargo test

# Run behavioral probes
./probe_parsing.sh
./reliability_probe.sh
./run_intention_scenarios.sh
./smoke_llamacpp.sh

# Run stress tests
./run_stress_tests_cli.sh
```

### Architecture Analysis
```bash
# Run the de-bloating analyzer
cd _dev-system/analyzer && cargo run
```

### Configuration Management
```bash
# View current config structure
ls -la config/

# View defaults
ls -la config/defaults/

# Test model-specific override
mv config/<model>/angel_helper.toml /tmp/
cargo run  # Should fall back to defaults
mv /tmp/angel_helper.toml config/<model>/
```

### Troubleshooting Quick Reference

**Connection Pool Exhaustion:**
- Symptom: Hangs after ~5 HTTP API calls, no timeout errors.
- Root Cause: Creating `reqwest::Client::new()` in hot paths (each intel unit call).
- Solution: Pass shared client through `IntelContext`.

**Shell Command Timeouts:**
- Symptom: 30-minute timeouts for simple tasks.
- Causes: Model hangs in retry loops, shell syntax issues, 30-minute timeout too long.
- Solution: Reduce to 5-minute timeout, fix shell command syntax.

**Terminology Mismatch:**
- Symptom: All requests routed to CHAT with entropy=0.00.
- Root Cause: Model tuned on old terminology (CHAT, SHELL), new terms not recognized.
- Solution: Revert to original terminology or perform full re-tuning.

**Pattern-Matching Routing:**
- Symptom: Over-orchestration, keyword-based decisions.
- Root Cause: Hardcoded word patterns in routing logic.
- Solution: Use confidence-based fallback (entropy > 0.8 → CHAT).
