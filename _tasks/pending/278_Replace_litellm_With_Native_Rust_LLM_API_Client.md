# Task 278: Replace litellm With Native Rust LLM API Client

## Status: PENDING
## Priority: HIGH

## Problem Statement
Other implementations rely on Python's litellm for unified LLM API access, but Elma can benefit from a native Rust implementation using enhanced reqwest.

## Analysis from Non-Rust Crates
- litellm provides unified interface to 100+ LLM APIs
- Used in aider, openhands, and other projects
- Python dependency that Elma can avoid with native implementation

## Solution Architecture
1. **Enhanced API client** building on existing reqwest usage
2. **Provider abstractions** for different LLM services
3. **Unified interface** similar to litellm functionality
4. **Configuration system** for API keys and endpoints

## Implementation Steps
1. Create LLM provider trait and implementations
2. Extend existing models_api.rs with provider support
3. Add configuration for multiple providers
4. Implement unified API interface
5. Add provider-specific optimizations
6. Test with major LLM providers (OpenAI, Anthropic, etc.)

## Integration Points
- `src/models_api.rs`: Enhanced with provider support
- `config/` system for API keys
- Existing intel units using LLM calls
- `src/app_bootstrap.rs`: Provider initialization

## Success Criteria
- Support for all major LLM providers natively
- Better performance than Python litellm
- No external Python dependencies for LLM access
- Backward compatibility with existing configuration
- `cargo build` passes

## Files to Create/Modify
- `src/models_api.rs` (major enhancement)
- New provider modules for each LLM service
- Configuration files for API keys
- `src/app_bootstrap.rs` (provider setup)

## Risk Assessment
- MEDIUM: Extends existing API client architecture
- Builds on proven reqwest foundation
- Can be implemented incrementally per provider
- No breaking changes to existing usage