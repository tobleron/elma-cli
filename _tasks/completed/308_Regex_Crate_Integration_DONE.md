# Task 308: Wire `regex` Crate

**Status:** completed
**References:** Proposal 004

## Objective

The `regex` crate is declared in `Cargo.toml` but never imported in any source file. Integrate it into the codebase where regex-based pattern matching would provide cleaner, faster alternatives to string splitting and manual character scanning.

## Scope

1. Identify candidate locations in `src/` where manual string manipulation could be replaced with regex patterns
2. Import `regex::Regex` and use for: pattern extraction in `text_utils.rs`, content parsing in `document_adapter.rs`, command parsing in `app_chat_loop.rs`
3. Compile regexes once (lazy_static or OnceLock) for performance
4. Write tests for regex-based utilities

## Verification

```bash
cargo build
cargo test
```
