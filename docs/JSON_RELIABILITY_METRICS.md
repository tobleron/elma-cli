# JSON Reliability Metrics

## Current Snapshot

- Parse success rate on `./run_intention_scenarios.sh`: `61/61` prompts completed with `0` transport or parse failures in the script run.
- Expanded grammar coverage: `11` profiles mapped in `config/grammar_mapping.toml`.
- Rust verification: `cargo build` and `cargo test` passing.

## Latency Measurement

Measured against the live endpoint `http://192.168.1.186:8080` using repeated router-style requests.

```json
{
  "n": 15,
  "with_grammar_avg_ms": 1103.69,
  "without_grammar_avg_ms": 890.16,
  "with_grammar_median_ms": 1054.7,
  "without_grammar_median_ms": 823.84,
  "overhead_pct_avg": 23.99
}
```

## Interpretation

- Reliability is materially improved and live scenario replay is currently stable.
- Grammar overhead is currently above the original `<10%` target, so performance optimization remains a follow-up item even though the implementation phases are in place.
