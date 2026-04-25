# Task 276: Add rayon For Parallel Document Processing

## Status: PENDING
## Priority: HIGH

## Problem Statement
Elma lacks parallel processing capabilities for CPU-intensive tasks like bulk document processing, text analysis, and file operations. Rayon provides ergonomic data parallelism across CPU cores.

## Analysis from Rust Crates
- Rayon enables easy parallelization of CPU-bound tasks
- Used in projects like aider and goose for performance
- Complements existing async tokio usage for CPU parallelism

## Solution Architecture
1. **Add rayon dependency** to Cargo.toml
2. **Parallel document processing** in document_adapter.rs for bulk operations
3. **Parallel text analysis** in intel units for large text processing
4. **Parallel file operations** in file_scout.rs for directory scanning

## Implementation Steps
1. Add rayon to Cargo.toml
2. Identify CPU-intensive loops that can be parallelized
3. Replace sequential processing with rayon::iter::ParallelIterator
4. Add parallel processing to document extraction pipeline
5. Implement parallel text chunking and analysis
6. Test performance improvements

## Integration Points
- `src/document_adapter.rs`: Parallel document processing
- `src/file_scout.rs`: Parallel file scanning
- Intel units with text processing
- `src/execution_steps_read.rs`: Parallel bulk reads

## Success Criteria
- Document processing performance improved by 2-4x on multi-core systems
- No breaking changes to existing APIs
- Graceful fallback to sequential processing if needed
- `cargo build` passes

## Files to Create/Modify
- `Cargo.toml` (add rayon dependency)
- `src/document_adapter.rs` (parallel processing)
- `src/file_scout.rs` (parallel scanning)
- Various intel units (parallel text processing)

## Risk Assessment
- LOW: Rayon is mature and widely used
- Performance benefits outweigh integration effort
- Backward compatible addition