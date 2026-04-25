# Task 277: SQLite Database Integration For Structured Session Storage

## Status: PENDING
## Priority: HIGH

## Problem Statement
Elma uses file-based JSON storage for sessions and state, lacking structured querying, efficient storage, and advanced caching capabilities that SQLite provides.

## Analysis from Rust Crates
- rusqlite provides ergonomic SQLite bindings
- Used in multiple projects for local data storage
- Better than file-based storage for structured data

## Solution Architecture
1. **SQLite session storage** to replace JSON file storage
2. **Schema design** for sessions, messages, artifacts, and metadata
3. **Migration system** for schema evolution
4. **Query capabilities** for session analysis and retrieval

## Implementation Steps
1. Add rusqlite to Cargo.toml
2. Design database schema for sessions and state
3. Create migration system for schema updates
4. Implement SessionStore trait with SQLite backend
5. Migrate existing JSON storage to SQLite
6. Add query methods for session analysis
7. Update all storage calls to use new backend

## Integration Points
- `src/session.rs`: Session storage abstraction
- `src/session_write.rs`: Write operations
- `src/storage.rs`: Storage interface
- All components using file-based session storage

## Success Criteria
- All existing functionality preserved
- Improved storage performance and reliability
- Query capabilities for session analysis
- Backward compatibility maintained
- `cargo build` and tests pass

## Files to Create/Modify
- `Cargo.toml` (add rusqlite)
- `src/storage.rs` (SQLite implementation)
- `src/session.rs` (storage interface updates)
- `src/session_write.rs` (SQLite writes)
- Migration scripts and schema files

## Risk Assessment
- MEDIUM: Major storage system change
- Need careful migration of existing data
- SQLite is battle-tested and reliable
- Can rollback if issues arise