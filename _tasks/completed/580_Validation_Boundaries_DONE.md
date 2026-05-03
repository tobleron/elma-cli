# 580 — Document Validation Boundaries in Architecture Docs

- **Priority**: Medium
- **Category**: Documentation
- **Depends on**: 552 (split tool_calling.rs), 553 (tool arg validation), 557 (path sandboxing)
- **Blocks**: None

## Problem Statement

The codebase lacks a document that maps ALL validation boundaries — where untrusted input crosses a trust boundary and what validation is applied. Without this document:

1. New developers don't know where to add validation
2. Security auditors can't systematically review validation coverage
3. The system can't guarantee that all model output is validated before execution

Per AGENTS.md, validation boundaries include: model output, user input, config files, paths, shell commands, tool arguments, JSON/TOML/custom DSL parsing.

## Recommended Target Behavior

Create `docs/VALIDATION_BOUNDARIES.md` documenting:

1. **Trust Boundary Map**: Diagram of all input sources and where they enter the system
2. **Validation Layers**: What validation happens at each boundary
3. **Data Flow**: How data moves through validation layers
4. **Gap Analysis**: Known missing validation points
5. **Validation Functions**: Reference to the code that implements each validation

### Boundary Catalog

| Boundary | Source | Input Type | Validation | File |
|----------|--------|------------|------------|------|
| User input | stdin/TUI | Text | Prefix parsing, length limit | `input_parser.rs` |
| Model output - tool calls | LLM API | JSON | JSON parse, arg validation | `tool_calling.rs` |
| Model output - classification | LLM API | JSON | JSON parse, schema validation | `routing_parse.rs` |
| Model output - intel units | LLM API | JSON | JSON parse, repair, fallback | `json_parser.rs` |
| Model output - final answer | LLM API | Text | Thinking block stripping | `text_utils.rs` |
| Shell commands | Model | String | Preflight, classification, sandbox | `shell_preflight.rs` |
| File paths | Model | String | Relative path only, no ../ | Various tools |
| Config files | Disk | TOML | Parse, schema, defaults | `defaults.rs` |
| Tool arguments | Model | JSON | Type check, range, required fields | `tool_calling.rs` |
| API responses | Network | JSON/SSE | Status code, parse, streaming | `llm_provider.rs` |

## Acceptance Criteria

- Document covers all 10+ validation boundaries
- Each boundary references source file and validation function
- Includes data flow diagram
- Includes known gaps
- Maintenance note: update when adding new input sources
