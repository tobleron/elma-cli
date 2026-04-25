# Task Management - Follow Instructions In Exact Order

## Task Creation Rule

### Main Project Tasks (Numbered 0XX / 1XX / 2XX)
- Every new task uses the next available numeric prefix across `_tasks/active/`, `_tasks/pending/`, `_tasks/completed/`, and `_tasks/postponed/`.
- Use three-digit padding.
- Task files must be self-documenting enough that status can be inferred from filename and header.

### Dev-System Tasks (D###)
- Stored in `_dev-tasks/`.
- Advisory only.

### Troubleshooting Tasks (T###)
- Use the same numeric sequence with a `T` prefix.
- Start immediately when a real regression or failure class is being investigated.

## Current Master Plan
- **Task 191**: Elma Skill-First Rebaseline Master Plan — COMPLETE.
- **Task 203**: Document Intelligence Track — ACTIVE (EPUB framework complete, PDF complete).
- **Task 278**: Agent Architecture Enhancement Track — PENDING (Rust crate additions, agent improvements).
- Security hardening (236-245): 236-239 complete, 240-245 pending.
- Library additions (214-234): Most complete, remaining pending.

## Active Tasks
| # | File | Summary |
|---|------|---------|

## Recently Completed
| # | File | Summary |
|---|------|---------|
| **250** | `250_PDF_Page_Aware_Rust_Extraction_Upgrade.md` | ✅ PDF page-aware extraction with metadata, quality flags, and predictable failure modes (2026-04-25) |
| **251** | `251_EPUB_HTML_XHTML_And_Package_Text_Extraction.md` | ✅ EPUB/HTML/XHTML structure-aware extraction framework with metadata, spine processing, and intelligent chunking (2026-04-25) |

## Pending Task Queue

### Phase 1: Security & Reliability (P0 — Complete These First)
| # | File | Summary |
|---|------|---------|
| **240** | `240_Shell_Injection_Fix_Workspace_Discovery.md` | Fix shell injection in `try_workspace_discovery` |
| **241** | `241_Remove_Atty_Crate_Use_IsTerminal.md` | Replace `atty` (RUSTSEC-2021-0145) with `std::io::IsTerminal` |
| **242** | `242_Snapshot_Rollback_Atomicity_And_Preflight.md` | Pre-flight gate + atomicity for snapshot rollback |
| **243** | `243_Incremental_Token_Counter_And_Busy_Queue_Type_Fix.md` | O(n)→O(1) token counter; fix double-Result in `await_with_busy_queue` |
| **244** | `244_Shell_Preflight_Glob_Fix_And_Dead_Code_Cleanup.md` | Multi-star `glob_match` fix; remove dead `parse_shlex` |
| **245** | `245_Reliability_Hardening_Logprobs_Verbose_Correlation.md` | Logprobs probe, verbose default fix, request correlation IDs |

### Phase 2: Foundation Enhancement (High Impact, Low Complexity)
| # | File | Summary |
|---|------|---------|
| **214** | `214_Beautiful_Panic_Reports_Via_Color_Eyre.md` | `color-eyre` for beautiful panic reports |
| **220** | `220_Gzip_Deflate_Compression_Via_Flate2.md` | `flate2` for compression support |
| **221** | `221_Async_Trait_Methods_Via_Async_Trait.md` | `async-trait` for ergonomic async traits |
| **222** | `222_Stream_Adapters_Via_Tokio_Stream.md` | `tokio-stream` for async stream utilities |
| **225** | `225_Extra_Serde_Adapters_Via_Serde_With.md` | `serde_with` for enhanced serialization |
| **226** | `226_ZIP_Archive_Reading_Via_Zip.md` | `zip` for archive handling |
| **229** | `229_Chainable_Debug_Transforms_Via_Tap.md` | `tap` for debug transformations |
| **230** | `230_Enum_Utilities_Via_Strum.md` | `strum` for enum utilities |
| **231** | `231_RON_Config_Format_Support_Via_Ron.md` | `ron` for Rust object notation |
| **232** | `232_TOML_Editing_With_Formatting_Preservation_Via_Toml_Edit.md` | `toml_edit` for TOML editing |
| **233** | `233_Fast_XML_Parsing_Via_Quick_Xml.md` | `quick-xml` for XML parsing |
| **276** | `276_Add_rayon_For_Parallel_Document_Processing.md` | Add rayon for parallel CPU-intensive tasks |
| **277** | `277_SQLite_Database_Integration_For_Structured_Session_Storage.md` | SQLite database for structured session storage |

### Phase 3: Document Intelligence (Current Major Track)
| # | File | Summary |
|---|------|---------|
| **246** | `246_Rust_Native_Ebook_Chat_Intelligence_Master_Plan.md` | Master plan for Rust-native ebook/document chat across common and legacy formats |
| **249** | `249_Document_Model_V2_Metadata_Provenance_And_Quality.md` | Add normalized metadata, units, chunks, provenance, and quality reports |
| **252** | `252_Kindle_MOBI_AZW_AZW3_KFX_Adapter_Plan.md` | Improve MOBI/AZW and evaluate AZW3/KFX Rust-native support |
| **253** | `253_FB2_And_DjVu_Structure_Aware_Adapters.md` | Add FB2 and page/TOC-aware DjVu handling |
| **254** | `254_Text_RTF_DOCX_And_Legacy_DOC_Adapters.md` | Add encoding-aware TXT, RTF, DOCX, and honest legacy DOC handling |
| **255** | `255_Comic_And_Apple_Book_Package_Adapters_CBZ_CBR_IBA.md` | Add CBZ/CBR/IBA package handling with image-only limitations disclosed |
| **256** | `256_Legacy_And_Exotic_Ebook_Format_Policy_CHM_LIT_PDB_LRF_LRX.md` | Recognize or gate CHM, LIT, Palm PDB, LRF, and LRX formats |
| **257** | `257_Token_And_Structure_Aware_Document_Chunking.md` | Replace byte chunking with token and structure-aware chunking |
| **258** | `258_Context_Budget_Document_Work_Planner.md` | Add full-document, staged, scoped, and retrieval-first work planning |
| **259** | `259_Persistent_Document_Index_Cache_And_Change_Detection.md` | Cache extraction/chunking with source signatures and invalidation |
| **260** | `260_Hybrid_Document_Retrieval_BM25_Embeddings_And_Source_Diversity.md` | Add lexical/optional semantic retrieval with source diversity |
| **261** | `261_Ebook_Chat_Answering_With_Citations_And_Extractive_Fallback.md` | Add source-grounded ebook answers with citations and extractive fallback |
| **262** | `262_Transcript_Native_Document_Telemetry_And_User_Controls.md` | Surface extraction/index/retrieval state in transcript rows |
| **263** | `263_Document_Intelligence_Fixtures_Stress_Gates_And_Regression_Suite.md` | Add fixtures, stress gates, and real CLI validation for the ebook track |

### Phase 4: Agent Architecture Improvements (Medium Priority)
| # | File | Summary |
|---|------|---------|
| **264** | `264_Dynamic_Tool_Registry_With_Searchable_Capabilities.md` | Dynamic tool registry with searchable capabilities (Claude-Code approach) |
| **265** | `265_Granular_Tool_Control_Flags_For_Enhanced_Safety.md` | Add granular tool control flags (isReadOnly, isDestructive, etc.) |
| **266** | `266_Automatic_Large_Result_Persistence_Mechanism.md` | Automatic persistence of large tool outputs to prevent context flooding |
| **267** | `267_Isolated_Sub_Agent_Delegation_With_Git_Worktrees.md` | Isolated sub-agent delegation using git worktrees |
| **268** | `268_Formal_Background_Task_Management_System.md` | Formal background task management for long-running operations |
| **269** | `269_Advanced_Context_Compaction_With_LLM_Summarization.md` | Advanced context compaction using LLM-based summarization (Roo-Code approach) |
| **270** | `270_Model_Context_Protocol_Integration_For_Dynamic_Capabilities.md` | Model Context Protocol (MCP) integration for dynamic capabilities |
| **271** | `271_Trajectory_Compression_For_Long_Running_Sessions.md` | Trajectory compression for long-running sessions (Hermes-Agent approach) |
| **272** | `272_Safe_Mode_Toggle_System_For_Permission_Levels.md` | Safe mode toggle system like Open-Interpreter (ask/on/off modes) |
| **273** | `273_Hybrid_Search_Memory_System_With_FTS_And_Vector_Search.md` | Hybrid search memory with FTS and vector search (OpenCrabs approach) |
| **274** | `274_FETCH_Operation_With_Comprehensive_Security_Gating.md` | FETCH operation with comprehensive security gating |
| **275** | `275_OBSERVE_Step_Type_For_Metadata_Only_Inspection.md` | OBSERVE step type for metadata-only file/directory inspection |

### Phase 5: Advanced API Integration (Lower Priority)
| # | File | Summary |
|---|------|---------|
| **278** | `278_Replace_litellm_With_Native_Rust_LLM_API_Client.md` | Replace litellm with enhanced reqwest-based API client supporting all LLM providers natively |

## Canonical Implementation Sequence (Updated)
1. ✅ **191 Track**: Complete (193-204, T235)
2. 🔄 **Phase 1**: Security & Reliability (240-245)
3. 🔄 **Phase 2**: Foundation Enhancement (214-233, 276-277)
4. 🔄 **Phase 3**: Document Intelligence (246, 249-263)
5. 🔄 **Phase 4**: Agent Architecture (264-275)
6. 🔄 **Phase 5**: Advanced Integration (278)

## Dependency Notes
- **Security tasks (240-245)**: Independent, can be parallelized but complete before other work
- **Library additions (214-233)**: Simple crate additions, low risk, high parallelization
- **Performance crates (276-277)**: High impact, implement after basic libraries
- **Document Intelligence (246, 249-263)**: Depends on PDF/EPUB work (250-251 active)
- **Agent improvements (264-275)**: Can be parallelized after foundation is solid
- **API replacement (278)**: Advanced integration, implement last

## Workflow Instructions
1. Pickup: move the intended task to `_tasks/active/` if starting formal implementation.
2. Implement surgically.
3. Verify with `cargo build`.
4. Verify with relevant tests and real CLI or PTY validation.
5. Report while the task is still active.
6. Archive only after approval.

## Folder Meaning
- `_tasks/active/`: current implementation tracks.
- `_tasks/pending/`: next approved work in the task-first, formula-driven direction.
- `_tasks/completed/`: finished work.
- `_tasks/postponed/`: deferred, absorbed, or superseded work kept for history.
- `_dev-tasks/`: analyzer guidance.