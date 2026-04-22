# Memory Footprint Optimization Guide for elma-cli

## Goal
Reduce the memory footprint of **elma-cli** to improve startup time, reduce overall process size, and lower resource consumption when running locally with a small model.

---

### 1. Profile Current Memory Usage
```bash
# Use `valgrind` or `massif` (part of Valgrind) to capture heap snapshots before/after launch
valgrind --tool=massif ./target/release/elma-cli --help > /dev/null
```
*Record the peak RSS (resident set size) and identify hot modules.*

### 2. Dependency Audit
| Library | Approx. Size | Potential Issue | Action |
|---------|--------------|-----------------|--------|
| serde   | ~200 KB      | JSON parsing overhead | Consider `serde_json::Value` with `pretty` disabled; use `simd-json` if performance permits.
| tokio   | ~300 KB      | Async runtime weight  | Profile async task count; consider disabling background workers for static mode.
| serde_derive | ~150 KB | Codegen bloat | Enable only needed derive macros (`Serialize`, `Deserialize`).

Run `cargo tree -i` to visualize binary size contributions and prune unused crates.

### 3. Data Structure Refactoring
- **Use smaller integer types** where precision allows (e.g., `u8`/`i16` instead of `usize` for small indices).
- **Avoid deep cloning**: replace `.clone()` on large structs with references or move semantics when ownership is safe.
- **Leverage `Rc<RefCell<_>>` only when shared mutable state is truly required; otherwise, pass by value.

### 4. Lazy Loading & On‑Demand Initialization
Wrap heavyweight modules (e.g., the full JSON schema parser) behind a lazy flag:
```rust
lazy_static! {
    static ref SCHEMA: Option<serde_json::Value> = None;
}
fn init_schema() -> &'static serde_json::Value {
    SCHEMA.get_or_insert_with(||
        serde_json::from_str(include_str!("../data/schema.json")).unwrap()
    ))
}

// In code:
let schema = init_schema();
```
This defers loading until the first request, reducing initial RAM usage.

### 5. JSON Parsing/Serialization Optimizations
- Switch to **`simd-json`** (if allowed) for ~2× faster parsing with comparable memory use.
- Disable pretty‑printing in logs:
```rust
serde_json::Serializer::with_formatter(...).pretty(false)
```
- Batch multiple small objects into a single `Vec` and serialize once, then stream if needed.

### 6. Native Bindings / WASM Considerations
- Evaluate **Rust‑to‑Wasm** for the inference engine: compile critical math modules to WebAssembly to offload heavy CPU/GPU work while keeping the host binary small.
- Use **`cargo-afl`** or **`wasm-pack`** to generate a WASM build and benchmark size (`ls -lh target/wasm32-wasi/*.wasm`).

### 7. Binary Size Reduction Techniques
```bash
# Strip debug info & unused symbols
strip --strip-all target/release/elma-cli
# Remove DWARF debugging data
rustc --debug-information-format=dwarfless src/main.rs
```
Also enable **`lto=full`** during build to allow the linker to eliminate dead code more aggressively.

### 8. Memory Profiling Hooks & Threshold Alerts
Add a runtime hook that logs memory usage at key stages:
```rust
use std::sync::{Once, ONCE_INIT};
static LOG_ONCE: Once = ONCE_INIT;
sleep_once(&mut LOG_ONCE, || {
    println!("=== MEMORY DUMP === ",);
    process::MemoryInfo::new().unwrap();
});
```
Configure a threshold (e.g., > 150 MiB) that triggers an error message to alert developers.

### 9. Binary Size Reduction Final Pass
```bash
# After all changes, rebuild and compare sizes
cargo build --release && du -h target/release/elma-cli
```
Verify the binary now fits within your target memory budget (e.g., < 30 MiB). Adjust step priorities until satisfied.

### 10. Benchmark & Verify Impact
Use **`perf`, `valgrind --tool=massif`, or custom benchmarks** to measure:
- Startup time reduction (%)
- Peak RSS before/after changes
- Memory allocation rate (allocations/sec) during typical workflow

Document results in this file and iterate until the memory budget is met.

---

*This guide provides a concrete roadmap for aggressively minimizing elma-cli’s memory usage while preserving core functionality.*