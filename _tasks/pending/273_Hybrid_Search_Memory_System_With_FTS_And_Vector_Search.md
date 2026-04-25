# Task 273: Hybrid Search Memory System With FTS And Vector Search

## Status: PENDING
## Priority: LOW

## Problem Statement
Elma lacks hybrid search memory. OpenCrabs' hybrid search (FTS + Vector) provides better knowledge retrieval and persistence for complex reasoning.

## Analysis from OpenCrabs
- Hybrid search combining FTS and vector search
- Better retrieval for different query types
- Improved knowledge persistence
- Enhanced memory capabilities

## Solution Architecture
1. **Search Backend**: Create `src/hybrid_search.rs` for search management
2. **FTS Integration**: Add full-text search capabilities
3. **Vector Search**: Implement vector similarity search
4. **Hybrid Queries**: Combine both search types intelligently

## Implementation Steps
1. Implement FTS indexing system
2. Add vector embedding generation
3. Create hybrid search query logic
4. Integrate with existing memory systems
5. Add persistence for search indices
6. Test retrieval quality

## Integration Points
- `src/hybrid_search.rs`: New search management module
- Existing memory/session systems
- Intel units for embedding generation
- Query processing in orchestration

## Success Criteria
- Improved knowledge retrieval
- Better search results for different query types
- Persistent search indices
- Integration with existing memory
- `cargo build` passes

## Files to Create/Modify
- `src/hybrid_search.rs` (new)
- Memory system integration (modify)
- Embedding generation (new)
- Search index persistence (new)

## Risk Assessment
- LOW: Advanced feature, can be optional
- Performance considerations for large indices
- Backward compatible
- Start with basic implementation