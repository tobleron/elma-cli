# Task 275: OBSERVE Step Type For Metadata-Only Inspection

## Backlog Reconciliation (2026-05-02)

Superseded by completed Task 393. Any follow-up belongs in Task 445 parity or Task 457 rust-first file operations.


## Status: PENDING
## Priority: MEDIUM

## Problem Statement
Elma lacks explicit metadata-only inspection. An OBSERVE step type would complement existing READ operations by allowing metadata inspection without content consumption.

## Analysis from Multiple Implementations
- OBSERVE for metadata-only operations
- Complements READ operations
- Better for discovery and inspection
- Prevents unnecessary content loading

## Solution Architecture
1. **OBSERVE Step**: Add OBSERVE variant to Step enum
2. **Metadata Collection**: Implement metadata gathering logic
3. **Integration**: Wire into existing execution framework
4. **Use Cases**: File stats, directory listings, etc.

## Implementation Steps
1. Add OBSERVE to Step enum
2. Implement metadata collection functions
3. Create execution logic for OBSERVE steps
4. Add to orchestration loop
5. Test with various metadata types
6. Integrate with existing tools

## Integration Points
- `src/types_core.rs`: Add OBSERVE step variant
- `src/execution_steps_*.rs`: New OBSERVE execution module
- `src/orchestration_loop.rs`: Handle OBSERVE steps
- Metadata utility functions (new)

## Success Criteria
- OBSERVE steps work for metadata inspection
- Complements existing READ operations
- Efficient metadata collection
- Integration with orchestration
- `cargo build` passes

## Files to Create/Modify
- `src/types_core.rs` (modify)
- `src/execution_steps_observe.rs` (new)
- `src/orchestration_loop.rs` (modify)
- Metadata utility functions (new)

## Risk Assessment
- LOW: Simple additive feature
- No breaking changes
- Backward compatible
- Easy to implement and test