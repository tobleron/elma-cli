# Master Plan

Last updated: 2026-04-29

## Completed

### Track 191 — Skill-First Rebaseline
Task 191 and subtasks (193-204, T235) — COMPLETE.

### Phase 1 — Security & Reliability
Tasks 236-245, 286 — ALL COMPLETE.

### Session & Runtime Infrastructure (T282–T285, T287)
| # | Summary |
|---|---------|
| 282 | Session garbage collector and index (session-gc CLI) |
| 283 | Session transcript flush on tool results and PTY |
| 284 | Panic hook transcript path integration |
| 285 | Sessions storage optimization (compression, dedup, retention) |
| 287 | Evidence ledger — structured evidence tracking, claim enforcement |

### Respond & Stagnation Hardening (T319, T333)
| # | Summary |
|---|---------|
| 319 | Principle-based strategy guidance (replaced hardcoded suggest_alternatives) |
| 333 | Respond tool abuse guard — prevents empty respond loops |

### Governance & Directives
| Document | Status |
|----------|--------|
| D001: Evidence-Grounded Stability | Proposed |
| D002: Dispatchable Execution Modes | Proposed |
| D003: Semantic Continuity | Proposed |
| D004: Offline-First Architecture | Proposed |
| D005: Transcript Visibility | Proposed |
| D006: Principle-First Prompts | Proposed |
| D007: Dynamic Decomposition | Proposed |
| D008: Tokenized Theme | Proposed |

### Key Completions
| # | Summary |
|---|---------|
| 333 | Respond tool abuse guard — prevents empty respond loops |
| 319 | Principle-based strategy guidance — replaces hardcoded command hints |
| 287 | Evidence ledger — structured evidence tracking with claim enforcement |
| 285 | Sessions storage optimization — compression and retention policies |
| 284 | Panic hook transcript path integration — faster debugging |
| 283 | Session transcript flush — incremental crash-safe persistence |
| 282 | Session garbage collector and index — safe cleanup with archiving |
| 278 | Native Rust LLM API client (OpenAI, Anthropic, compatible) |
| 277 | SQLite session store |
| 276 | Parallel document processing (rayon) |
| 291 | Core tools always available |
| 290 | Advanced reliability and guidance |
| 306 | AGENTS.md rule against hardcoded input processing |
| 305 | Collect-then-reduce decision pattern |
| 304 | Feature-flagged classification system |
| 303 | Structured classifier interface |
| 302 | Eliminate deterministic checks in routing |
| 281 | 30-minute timeout with failure message |
| 280 | Non-interactive shell permission gate fix |
| 273 | Hybrid search memory system |
| 272 | Safe mode toggle system |
| 271 | Trajectory compression |
| 250 | PDF page-aware extraction |
| 251 | EPUB/HTML/XHTML structure-aware extraction |
| 207 | Advanced assessment intel units |

## In Progress

### Phase 2 — Foundation Enhancement
| # | Summary |
|---|---------|
| 214 | Beautiful panic reports (color-eyre) |
| 220 | Compression support (flate2) |
| 221 | Async trait methods (async-trait) |
| 222 | Stream adapters (tokio-stream) |
| 225 | Enhanced serialization (serde_with) |
| 226 | ZIP archive reading |
| 229 | Debug transforms (tap) |
| 230 | Enum utilities (strum) |
| 231 | RON config format |
| 232 | TOML editing preservation |
| 233 | Fast XML parsing (quick-xml) |

### Phase 3 — Document Intelligence
| # | Summary |
|---|---------|
| 246 | Ebook chat intelligence master plan |
| 249 | Document model v2 (metadata, provenance, quality) |
| 252 | MOBI/AZW/AZW3/KFX adapters |
| 253 | FB2 and DjVu structure-aware adapters |
| 254 | TXT/RTF/DOCX/legacy DOC adapters |
| 255 | Comic and Apple Book package adapters |
| 256 | Legacy ebook format policy |
| 257 | Token-aware document chunking |
| 258 | Context budget document work planner |
| 259 | Persistent document index cache |
| 260 | Hybrid document retrieval |
| 261 | Ebook chat answering with citations |
| 262 | Transcript-native document telemetry |
| 263 | Document intelligence fixtures and regression suite |

### Phase 4 — Agent Architecture
| # | Summary |
|---|---------|
| 264 | Dynamic tool registry (searchable capabilities) |
| 265 | Granular tool control flags |
| 266 | Large result persistence |
| 267 | Isolated sub-agent delegation |
| 268 | Background task management |
| 269 | Advanced context compaction |
| 270 | Model Context Protocol integration |
| 274 | FETCH operation security gating |
| 275 | OBSERVE step type |

### Session & Runtime
| # | Summary |
|---|---------|
| 279 | Lightweight auxiliary LLM helper |
| 334 | Persist finalized summaries as markdown in session summaries folder |

### Phase 5 — Modes & Architecture Enforcement (NEW)
| # | Summary | Directive |
|---|---------|-----------|
| 301 | Data Analysis Mode | D002 |
| 302 | Semantic Continuity Tracking | D003 |
| 303 | Offline-First Architecture | D004 |
| 304 | Transcript Operational Visibility | D005 |
| 305 | Principle-First Prompt Cleanup | D006 |
| 306 | Dynamic Decomposition On Weakness | D007 |
| 307 | Tokenized Theme Enforcement | D008 |
| 308 | Regex Crate Integration | P004 |
| 309 | Strip-ANSI Escapes Crate | P004 |

## Sequencing

1. Complete Phase 2 (foundation crates) — low risk, high parallelization
2. Continue Phase 3 (document intelligence) — depends on 250-251 (complete)
3. Phase 4 (agent architecture) — can parallelize after foundation solid
4. Session & runtime improvements — can slipstream into any phase
5. Phase 5 (modes & architecture enforcement) — implement after Phase 2 foundation crates are complete; data analysis mode (301) depends on extraction crate integration

## Dependency Notes

- Security tasks (240-245): Independent, all complete
- Library additions (214-233): Simple crate additions, no cross-dependencies
- Performance crates (276-277): Complete
- Document Intelligence (246-263): Depends on 250-251 (complete)
- Agent improvements (264-275): Can be parallelized
- API replacement (278): Complete
