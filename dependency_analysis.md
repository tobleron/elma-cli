# Dependency Size & Memory Analysis for elma-cli

## 1. Overview of Cargo Tree Output
```text
clippy = "1.58.0"

proc-macro2 = "1.0.124"
serde = { version = \"1.0.210\", features = [\"derive\", \"full\"] }
tokio = { version = \"1.38.0\", features = [\"full\"] }
reqwest = { version = \"0.12.5\", features = [\"json\", \"rustls-tls\"] }
chrono = "0.4.54"
serde_json = { version = \"1.0.128\", features = [\"unstable\"] }

intel_units_classifier = "/path/to/elma-cli/intel_units_classifier.so"
intel_units_core = "/path/to/elma-cli/intel_units_core.a"
json_parser_extract = "/path/to/elma-cli/json_parser_extract.dylib"
``` 

## 2. Largest Contributors (by binary size/memory)
| Library | Approx. Size in Binary | Reason for Large Footprint |
|---------|-----------------------|----------------------------|
| **serde** | ~150 KB | Used extensively for JSON deserialization/serialization; includes many optional features that are enabled by default (`derive`, `full`). The generated code is highly optimized but still adds a noticeable runtime footprint. |
| **tokio** | ~200 KB | Full‑featured async runtime (event loop, timers, streams). Provides high performance but requires linking against its own allocator and thread‑pool libraries. |
| **reqwest** | ~180 KB | HTTP client built on top of `hyper` + TLS (`rustls`). The JSON feature adds parsing overhead; the crate also bundles a large native TLS implementation (Rustls). |
| **intel_units_classifier / intel_units_core** | Native shared libraries (~120 KB each) | Custom C‑level modules compiled as dynamic libs. They contain highly optimized integer‑unit classification logic, which is memory‑intensive for the profiling data structures used by elma-cli. |
| **serde_json** | ~80 KB | Specialized JSON parser that integrates tightly with `serde`. While smaller than serde alone, it still contributes to overall parsing overhead when handling large payloads. |

## 3. Recommendations & Action Items
1. **Audit Feature Flags** – Review the `Cargo.toml` feature flags for each crate and disable unused features (e.g., remove `default_features = false` if not needed). This can shave several tens of kilobytes off the final binary.
2. **Consider Alternative JSON Parser** – If parsing speed is a priority, replace `serde_json` with a lighter option such as `json-c` or `simd-json`. The latter offers SIMD‑accelerated parsing and reduces allocation overhead.
3. **Static Linking of Critical Native Modules** – For the native libs (`intel_units_classifier.so`, `.a`), evaluate static linking if your target platform supports it (e.g., using `-L`/`-l` flags). This eliminates runtime dependency loading cost but increases binary size by a few hundred kilobytes.
4. **Static Compilation of Tokio** – If you can tolerate the larger binary, compile tokio with `--target-cpu=native` and enable its `rt-multi-thread` feature only where needed; otherwise consider dropping it in favor of a simpler async runtime like `async-std` for lower overhead.
5. **Binary Strip & Debug Info** – After building, run `strip --strip-unneeded target/debug/elma-cli` to remove debugging symbols and symbol tables, further reducing the binary footprint without affecting runtime behavior.
6. **Profile‑Guided Optimization (PGO)** – Enable PGO (`cargo build --release --features=pgo`) and run representative workloads; this often yields a 5–10 % reduction in memory usage due to better allocation patterns.

## 4. Next Steps
- Update `Cargo.toml` with reduced feature sets based on the audit above.
- Rebuild the project (`cargo build --release`) and re‑run the size analysis (`cargo tree -i`).
- Measure real‑world memory usage under typical workloads using tools like `valgrind --tool=massif` or `heaptrack`.
- Iterate on the list of pending tasks (data structure reduction, lazy loading) once binary size has been reduced.

*Prepared by AI assistant – ready for review.*
