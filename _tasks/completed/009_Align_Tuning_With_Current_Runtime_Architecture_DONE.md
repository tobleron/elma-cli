# Task 009: Align Tuning With Current Runtime Architecture

## Priority
**P1 - HIGH** (Trustworthy, reproducible tuning)

## Status
**PENDING** — Requires updates based on Task 044/045 architecture

## Context
Current tuning infrastructure exists:
- ✅ `tune_runtime.rs`, `tune_setup.rs`, `tune_scenario.rs`
- ✅ `model_behavior.toml` for model profiling
- ✅ `Profile` struct with tunable parameters
- ✅ Baseline comparison (`active`, `shipped` baselines)

**What needs to be added:**
- Explicit llama.cpp runtime defaults capture as protected baseline
- Variance/stability tracking across multiple runs
- Unit-type-specific parameter search bands
- Model behavior integration into tuning policy

## Objective
Make Elma's tuning pipeline safe, reproducible, and accountable by aligning with current runtime architecture.

## Non-Goals (Explicitly Out of Scope)
- ❌ Do NOT tune system prompts (prompts are frozen)
- ❌ Do NOT allow prompt mutation or automatic prompt rewriting
- ❌ Do NOT turn tuning into open-ended search over all request fields
- ❌ Do NOT optimize for single lucky runs instead of stable performance

## Safe Tuning Policy

### What Is Safe To Tune
| Parameter | Safe Range | Notes |
|-----------|------------|-------|
| `temperature` | 0.0 - 0.8 | Lower for JSON/verification units |
| `top_p` | 0.9 - 1.0 | Near-1.0 for deterministic units |
| `repeat_penalty` | 1.0 - 1.2 | Close to 1.0 for most units |
| `max_tokens` | 64 - 4096 | Unit-specific bounds |
| `reasoning_format` | "none" or "auto" | Only when allowed by `model_behavior.toml` |

### What Must Stay Fixed
- System prompts (all `*.toml` files with `system_prompt` field)
- Route label schema (CHAT/SHELL/PLAN/MASTERPLAN/DECIDE)
- JSON schema contracts
- Safety policy (command bans, destructive operation guards)
- Verification criteria (what constitutes success/failure)
- Formula memory acceptance rules
- Snapshot/rollback mechanics
- Shell safety guards and output caps

## Technical Tasks

### 1. Capture llama.cpp Runtime Defaults As Protected Baseline

**Problem:** Tuning compares against `active` and `shipped` baselines, but not against llama.cpp's actual runtime defaults.

**Solution:**
```rust
// In tune_setup.rs or tune_runtime.rs:
async fn capture_runtime_defaults(client: &Client, base_url: &Url) -> Result<Profile> {
    // Query llama.cpp /v1/models or infer from current generation
    // Create Profile with runtime defaults
    Ok(Profile {
        name: "runtime_defaults".to_string(),
        temperature: 0.2,  // llama.cpp default
        top_p: 0.95,       // llama.cpp default
        repeat_penalty: 1.0,
        max_tokens: 2048,  // llama.cpp default
        ..
    })
}
```

**Acceptance:** Runtime defaults appear as explicit baseline candidate in tuning reports.

### 2. Add Variance Penalty To Candidate Selection

**Problem:** Tuning may select candidates with high peak score but high variance (lucky runs).

**Solution:**
```rust
// For each serious candidate, run 3-5 repeated evaluations:
let scores = run_repeated_evaluations(candidate, scenarios, 5).await?;
let mean = scores.iter().sum::<f64>() / scores.len() as f64;
let variance = calculate_variance(&scores);
let std_dev = variance.sqrt();

// Penalize high-variance candidates:
let adjusted_score = mean - (std_dev * VARIANCE_PENALTY_MULTIPLIER);
```

**Acceptance:** Candidate ranking prefers low-variance, low-parse-failure profiles.

### 3. Bound Parameter Search By Unit Type

**Problem:** Uniform search bands may be too broad for some units, too narrow for others.

**Solution:**
```rust
// Define unit-type-specific search bands:
struct ParameterBands {
    temperature: (f64, f64),
    top_p: (f64, f64),
    repeat_penalty: (f64, f64),
    max_tokens: (u32, u32),
}

fn get_parameter_bands(unit_type: &str) -> ParameterBands {
    match unit_type {
        // Routing/verification/JSON units: near-deterministic
        "speech_act" | "router" | "critic" | "outcome_verifier" => {
            ParameterBands {
                temperature: (0.0, 0.1),
                top_p: (1.0, 1.0),
                repeat_penalty: (1.0, 1.0),
                max_tokens: (64, 256),
            }
        }
        // Orchestration units: low creativity
        "orchestrator" | "workflow_planner" | "formula_selector" => {
            ParameterBands {
                temperature: (0.2, 0.5),
                top_p: (0.9, 1.0),
                repeat_penalty: (1.0, 1.1),
                max_tokens: (2048, 4096),
            }
        }
        // Response units: modest creativity
        "elma" | "summarizer" | "formatter" => {
            ParameterBands {
                temperature: (0.4, 0.7),
                top_p: (0.9, 1.0),
                repeat_penalty: (1.0, 1.2),
                max_tokens: (1024, 4096),
            }
        }
        _ => DEFAULT_BANDS,
    }
}
```

**Acceptance:** Parameter search respects unit-type-specific bands.

### 4. Integrate Model Behavior Profile Into Tuning Policy

**Problem:** `model_behavior.toml` exists but may not influence parameter selection.

**Solution:**
```rust
// In tune_runtime.rs or parameter selection:
let behavior = load_model_behavior(model_id)?;

// If model has poor JSON reliability, penalize parse failures:
if !behavior.json_clean_with_auto {
    candidate_score -= PARSE_FAILURE_PENALTY;
}

// If model separates reasoning but truncates, tune response-side max_tokens:
if behavior.auto_reasoning_separated && behavior.auto_truncated_before_final {
    // Allow tuning of final_answer_extractor max_tokens
    enable_response_token_tuning = true;
}
```

**Acceptance:** Model behavior profile influences safe parameter policy.

### 5. Tune Finalizer And Formatter As First-Class Units

**Problem:** `final_answer_extractor` and `formatter` may not be included in tuning.

**Solution:**
- Include `final_answer_extractor` in response-quality stage
- Include `formatter` in output formatting stage
- Evaluate leaky thinking models through same runtime rescue path

**Acceptance:** Finalizer and formatter are tuned with bounded parameters.

### 6. Emit Clear Tune Summary

**Problem:** Tuning summary may not explain why winner was chosen.

**Solution:**
```rust
// At end of tuning, emit:
struct TuneSummary {
    model_id: String,
    active_run_id: Option<String>,
    active_baseline_score: f64,
    runtime_defaults_score: f64,
    shipped_baseline_score: f64,
    winner_score: f64,
    winner_variance: f64,
    certification_state: String,
    activation_happened: bool,
    why_winner_was_chosen: String,  // "lower variance", "fewer parse failures", etc.
}
```

**Acceptance:** Summary includes stability and accountability data.

## Acceptance Criteria
- [ ] llama.cpp runtime defaults captured as explicit baseline
- [ ] Variance/stability measured for all serious candidates
- [ ] Parameter search bands respect unit type
- [ ] Model behavior profile influences tuning policy
- [ ] Finalizer and formatter included in tuning
- [ ] Tune summary explains why winner was chosen
- [ ] No prompt text changes in active model folder
- [ ] Tuned profile not activated when runtime defaults are equal or more reliable

## Verification
1. Run `cargo run -- --tune` on:
   - One stable non-thinking model
   - One leaky thinking model
2. Confirm:
   - No prompt text changes
   - Runtime defaults appear as baseline candidate
   - Final summary includes variance and accountability data
3. Compare two repeated tune runs — winners should be materially consistent

## Dependencies
- ✅ Task 044 (Execution Ladder) — Provides level-aware validation
- ✅ Task 045 (Intel Units) — Provides trait pattern for tuning units

## Files to Modify
- `src/tune_runtime.rs` — Runtime defaults capture, variance tracking
- `src/tune_setup.rs` — Baseline handling, parameter bands
- `src/tune_scenario.rs` — Stability measurement
- `src/defaults_evidence.rs` — Model behavior integration

## Estimated Effort
4-6 hours

## Architecture Alignment
- ✅ IntelUnit trait pattern (Task 045)
- ✅ Model behavior profiling (existing)
- ✅ Principle-based tuning (no prompt mutation)
- ✅ Reproducible, accountable results
