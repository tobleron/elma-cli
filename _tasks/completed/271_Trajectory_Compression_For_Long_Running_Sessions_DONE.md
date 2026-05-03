# Task 271: Trajectory Compression For Long-Running Sessions

## Status: PENDING
## Priority: LOW

## Problem Statement
Elma lacks trajectory compression for long-running tasks. Hermes-Agent's trajectory_compressor reduces long histories into compact forms for deep reasoning tasks.

## Analysis from Hermes-Agent
- `trajectory_compressor.py` reduces long histories
- Specialized for trajectory compression
- Better handling of long-term memory
- Prevents context overflow in deep tasks

## Solution Architecture
1. **Trajectory Tracker**: Create `src/trajectory.rs` for session trajectory management
2. **Compression Engine**: Implement trajectory-specific compression
3. **Long-Term Memory**: Specialized storage for compressed trajectories
4. **Integration**: Wire into existing session management

## Implementation Steps
1. Create trajectory tracking system
2. Implement compression algorithms
3. Add trajectory-specific storage
4. Integrate with session management
5. Add compression triggers for long sessions
6. Test with extended reasoning tasks

## Integration Points
- `src/trajectory.rs`: New trajectory management module
- `src/session.rs`: Integrate trajectory tracking
- `src/auto_compact.rs`: Extend with trajectory compression
- Long-running task detection logic

## Success Criteria
- Long-running sessions compressed effectively
- Trajectory continuity maintained
- Memory usage optimized for deep reasoning
- No loss of critical trajectory state
- `cargo build` passes

## Files to Create/Modify
- `src/trajectory.rs` (new)
- `src/session.rs` (modify)
- `src/auto_compact.rs` (modify)
- Trajectory compression algorithms (new)

## Risk Assessment
- LOW: Specialized feature for long sessions
- Can be disabled if compression quality issues
- Backward compatible
- Start simple, enhance iteratively