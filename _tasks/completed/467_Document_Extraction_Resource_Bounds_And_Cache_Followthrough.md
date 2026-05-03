# Task 467: Document Extraction Resource Bounds And Cache Followthrough

**Status:** completed
**Priority:** HIGH
**Completed:** 2026-05-02

## Summary

Made document detection, extraction, chunking, and cache behavior resource-bounded and truthful.

## Implementation

### 1. Bounded Format Detection (64KB sniffing)
- Added `SNIFF_BYTES` constant (64KB)
- Changed `DocumentFormat::detect` to use bounded file reads instead of reading entire file
- Uses `std::io::Read` with a fixed-size buffer

### 2. Stable Content Signature
- Replaced `DefaultHasher` with SHA-1 for stable persisted cache keys
- Added `sha1 = "0.10"` dependency
- Full file content is hashed for unique document identity

### 3. Session Cache Persistence
- Implemented `DocumentIndexCache::save()` to persist cache to TOML file
- Implemented `DocumentIndexCache::load_cache()` to load from disk
- Cache file: `<cache_dir>/document_index_cache.toml`
- Added `toml = "0.8"` dependency

### 4. RAM-Aware Extraction
- Added `get_available_memory()` function to estimate available system RAM
- Added memory check before extraction: estimated RAM usage = file_size * 4.0
- Returns clear error if document too large for available memory
- Platform-specific: reads `/proc/meminfo` on Linux, estimates on macOS/Windows

### 5. Tests
- `test_format_detection_uses_bounded_read`: Verifies format detection works with bounded reads
- `test_signature_is_stable`: Verifies SHA-1 signatures are consistent
- `test_cache_persistence`: Verifies cache saves to disk correctly

## Resource Budgets (per user decision)
- **Sniffing bytes:** 64KB
- **Max document size:** No limit (with RAM management)
- **Cache:** Session cache only (persists to disk during session)
- **Huge docs:** Full extraction with RAM calculations