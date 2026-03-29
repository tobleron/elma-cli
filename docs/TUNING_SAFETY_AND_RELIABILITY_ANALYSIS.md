# Tuning Safety And Reliability Analysis

## Current Tuning Surface
Elma tuning is restricted to per-profile numeric inference fields:
- `temperature`
- `top_p`
- `repeat_penalty`
- `max_tokens`

Tuning may also activate a different profile set, but it must not mutate:
- `system_prompt`
- `reasoning_format`
- profile names
- schemas
- slash-command behavior
- deterministic safety controls

## Protected Baselines
Each tune run now evaluates three protected anchors when available:
- active live profile set
- immutable shipped baseline
- runtime-default baseline derived from `/props.default_generation_settings`

Activation is no longer based only on the highest raw score. It now records:
- baseline scores
- stability penalties
- the preferred baseline
- the final activation reason

## Quick Tune Guarantees
Quick tune is a startup gate, not a full certification pass.

It validates:
- routing quality
- workflow entry behavior
- execution entry behavior
- simple inspection/plan/decide behavior

It does not guarantee:
- deep multi-step recovery quality
- large artifact workflows
- broad platform portability
- full reviewer stability

## Full Tune Guarantees
Full tune evaluates:
- routing
- workflow/program quality
- execution quality
- response quality
- efficiency
- baseline comparison
- stability penalty on a critical quick subset

## Activation Policy
Activation prefers the candidate only when it meaningfully beats the preferred protected baseline.

If the improvement is marginal, Elma keeps the more stable baseline instead.

When runtime defaults are close to the best baseline, runtime defaults are preferred.

## Reliability Boundaries
Tuning is intentionally prevented from rewriting Elma's identity. Prompt mutation is disabled by reliability policy.

This keeps tuning:
- reproducible
- explainable
- safe to compare across models
- accountable when a model is simply a poor fit
