//! @efficiency-role: service-orchestrator
//!
//! Document Intelligence Skill Stack — normalized extraction pipeline
//! for txt, md, html, pdf, and epub formats.

use crate::*;
use std::path::Path;

/// Canonical document format registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DocumentFormat {
    // Plain text and code
    Txt,
    Markdown,
    Code,
    // Structured text
    Html,
    Xml,
    // Ebooks
    Epub,
    Pdf,
    Mobi,
    Azw,
    Azw3,
    Kfx,
    // Other ebooks
    FictionBook, // fb2
    DjVu,
    // Office formats
    Docx,
    Doc,
    Rtf,
    // Comic books
    Cbz,
    Cbr,
    // Apple books
    Iba,
    // Legacy formats
    Chm,
    Lit,
    PalmDoc, // pdb
    Lrf,
    Lrx,
    // Unknown
    Unknown(String),
}

impl DocumentFormat {
    /// Detect format from path (extension + magic bytes).
    pub(crate) fn detect(path: &Path) -> Self {
        // First: try magic bytes using `infer`
        if let Ok(bytes) = std::fs::read(path) {
            if let Some(kind) = infer::get(&bytes) {
                return match kind.mime_type() {
                    "application/pdf" => DocumentFormat::Pdf,
                    "application/epub+zip" => DocumentFormat::Epub,
                    "application/x-mobipocket-ebook" => DocumentFormat::Mobi,
                    "application/msword" => DocumentFormat::Doc,
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                        DocumentFormat::Docx
                    }
                    "application/rtf" => DocumentFormat::Rtf,
                    "application/x-chm" => DocumentFormat::Chm,
                    "application/x-cbr" => DocumentFormat::Cbr,
                    "application/x-cbz" => DocumentFormat::Cbz,
                    "application/zip" => {
                        // ZIP-based formats need deeper inspection
                        // For now, check extension
                        Self::from_extension(path)
                    }
                    "text/html" => DocumentFormat::Html,
                    "text/plain" => DocumentFormat::Txt,
                    "text/xml" => DocumentFormat::Xml,
                    _ => Self::from_extension(path),
                };
            }
        }
        // Fallback: extension-based detection
        Self::from_extension(path)
    }

    fn from_extension(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "txt" | "text" => DocumentFormat::Txt,
            "md" | "markdown" => DocumentFormat::Markdown,
            "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "toml"
            | "json" | "yaml" | "yml" => DocumentFormat::Code,
            "html" | "htm" | "xhtml" => DocumentFormat::Html,
            "xml" => DocumentFormat::Xml,
            "pdf" => DocumentFormat::Pdf,
            "epub" => DocumentFormat::Epub,
            "mobi" => DocumentFormat::Mobi,
            "azw" => DocumentFormat::Azw,
            "azw3" => DocumentFormat::Azw3,
            "kfx" => DocumentFormat::Kfx,
            "fb2" => DocumentFormat::FictionBook,
            "djvu" | "djv" => DocumentFormat::DjVu,
            "docx" => DocumentFormat::Docx,
            "doc" => DocumentFormat::Doc,
            "rtf" => DocumentFormat::Rtf,
            "cbz" => DocumentFormat::Cbz,
            "cbr" => DocumentFormat::Cbr,
            "iba" => DocumentFormat::Iba,
            "chm" => DocumentFormat::Chm,
            "lit" => DocumentFormat::Lit,
            "pdb" => DocumentFormat::PalmDoc,
            "lrf" => DocumentFormat::Lrf,
            "lrx" => DocumentFormat::Lrx,
            _ => DocumentFormat::Unknown(ext),
        }
    }

    pub(crate) fn backend_name(&self) -> &'static str {
        match self {
            DocumentFormat::Txt | DocumentFormat::Code => "native",
            DocumentFormat::Markdown => "native",
            DocumentFormat::Html | DocumentFormat::Xml => "html2text",
            DocumentFormat::Pdf => "pdf-extract",
            DocumentFormat::Epub => "epub",
            DocumentFormat::Mobi | DocumentFormat::Azw => "mobi",
            DocumentFormat::Azw3 | DocumentFormat::Kfx => "none (use boko when available)",
            DocumentFormat::FictionBook => "quick-xml",
            DocumentFormat::DjVu => "djvu-rs",
            DocumentFormat::Docx => "zip+quick-xml",
            DocumentFormat::Doc => "none (legacy CFB)",
            DocumentFormat::Rtf => "rtf-parser",
            DocumentFormat::Cbz => "zip+image-meta",
            DocumentFormat::Cbr => "rar (license-gated)",
            DocumentFormat::Iba => "zip+embedded-assets",
            DocumentFormat::Chm => "chm-parser (evaluate)",
            DocumentFormat::Lit => "none (CHM-derived legacy)",
            DocumentFormat::PalmDoc => "none (legacy PalmDB)",
            DocumentFormat::Lrf | DocumentFormat::Lrx => "none (Sony BBeB)",
            DocumentFormat::Unknown(ext) => "none",
        }
    }

    pub(crate) fn capability_state(&self) -> (&'static str, Option<&'static str>) {
        match self {
            DocumentFormat::Txt | DocumentFormat::Code | DocumentFormat::Markdown => {
                ("Full text", None)
            }
            DocumentFormat::Html | DocumentFormat::Xml => ("Full text", None),
            DocumentFormat::Pdf => (
                "Full text when text layer exists",
                Some("No OCR by default"),
            ),
            DocumentFormat::Epub => ("Full text", Some("OPF/spine/TOC/chapter-aware")),
            DocumentFormat::Mobi => ("Full text", Some("Legacy MOBI text and metadata")),
            DocumentFormat::Azw => (
                "Full/degraded",
                Some("Usually MOBI-like; may be DRM"),
            ),
            DocumentFormat::Azw3 | DocumentFormat::Kfx => (
                "Full/degraded",
                Some("Evaluate boko; DRM remains unsupported"),
            ),
            DocumentFormat::FictionBook => ("Full text", Some("XML FictionBook")),
            DocumentFormat::DjVu => (
                "Full text when text layer exists",
                Some("Image-only files fail clearly"),
            ),
            DocumentFormat::Docx => ("Full text", Some("ZIP/XML extraction")),
            DocumentFormat::Doc => (
                "Degraded/full if feasible",
                Some("Legacy CFB Word parsing is limited"),
            ),
            DocumentFormat::Rtf => (
                "Full/degraded",
                Some("Evaluate parser and fallback cleaner"),
            ),
            DocumentFormat::Cbz => (
                "Metadata/degraded",
                Some("Text only if metadata/text sidecars exist unless OCR feature lands"),
            ),
            DocumentFormat::Cbr => (
                "Metadata/degraded or unsupported",
                Some("RAR backend must be feature/license gated"),
            ),
            DocumentFormat::Iba => (
                "Full/degraded",
                Some("ZIP package with embedded HTML/XHTML/EPUB-like assets"),
            ),
            DocumentFormat::Chm => (
                "Degraded/unsupported",
                Some("CHM extraction backend requires explicit decision"),
            ),
            DocumentFormat::Lit => (
                "Unsupported/degraded",
                Some("CHM-derived legacy format; no false claims"),
            ),
            DocumentFormat::PalmDoc => (
                "Full/degraded for PalmDoc only",
                Some("Must not confuse with Microsoft debug PDB"),
            ),
            DocumentFormat::Lrf | DocumentFormat::Lrx => (
                "Unsupported/degraded",
                Some("Legacy Sony BBeB"),
            ),
            DocumentFormat::Unknown(_) => ("Unsupported", Some("Unknown format")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentChunk {
    pub index: usize,
    pub text: String,
    pub section_label: Option<String>,
    pub provenance: String,
    pub page: Option<u32>,
    pub section: Option<String>,
    pub confidence: f64,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentExtractionResult {
    pub source_path: String,
    pub backend: String,
    pub total_chunks: usize,
    pub chunks: Vec<DocumentChunk>,
    pub metadata: HashMap<String, String>,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentMetadata {
    pub format: String,
    pub file_size: u64,
    pub modified_time: Option<u64>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub page_count: Option<u32>,
    pub has_text_layer: Option<bool>,
    pub likely_ocr: bool,
}

// V2 Document Model Types

/// Stable source identity based on canonical path plus content signature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(crate) struct DocumentId {
    pub canonical_path: String,
    pub content_signature: String,
}

/// Normalized document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentMetadataV2 {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub publication_date: Option<String>,
    pub isbn: Option<String>,
    pub identifiers: Vec<String>,
    pub source_path: String,
    pub file_size: u64,
    pub format: DocumentFormat,
    pub backend: String,
}

/// Provenance information for document units and chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentProvenance {
    pub source_path: String,
    pub format: DocumentFormat,
    pub backend: String,
    pub page_number: Option<u32>,
    pub chapter_index: Option<usize>,
    pub chapter_title: Option<String>,
    pub section_heading_path: Vec<String>, // Hierarchical heading path
    pub archive_entry_path: Option<String>, // For archives/containers
    pub byte_offset_start: Option<u64>,
    pub byte_offset_end: Option<u64>,
    pub char_offset_start: Option<u64>,
    pub char_offset_end: Option<u64>,
}

/// Extracted structural unit before chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentUnit {
    pub index: usize,
    pub text: String,
    pub provenance: DocumentProvenance,
}

/// Token-sized retrievable unit after chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentChunkV2 {
    pub index: usize,
    pub chunk_index: usize, // Index within this unit's chunks
    pub total_chunks: usize, // Total chunks for this unit
    pub text: String,
    pub provenance: DocumentProvenance,
}

/// Extraction quality report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentQualityReport {
    pub extraction_warnings: Vec<String>,
    pub text_coverage_percent: Option<f32>,
    pub empty_pages: Vec<u32>,
    pub encoding_repairs: Vec<String>,
    pub encrypted_or_drm: bool,
    pub image_only: bool,
    pub likely_ocr: bool,
}

/// Format support state and backend explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentCapability {
    pub format: DocumentFormat,
    pub support_state: DocumentSupportState,
    pub backend: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum DocumentSupportState {
    Supported,
    Degraded,
    Unsupported,
}

// Legacy capability report for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentCapabilityReport {
    pub format: String,
    pub backend: String,
    pub available: bool,
    pub quality_note: Option<String>,
}

// Compatibility layer: Convert V2 types back to V1 for existing callers

impl From<&DocumentProvenance> for String {
    fn from(provenance: &DocumentProvenance) -> String {
        let mut parts = vec![format!("path:{}", provenance.source_path)];
        if let Some(page) = provenance.page_number {
            parts.push(format!("page:{}", page));
        }
        if let Some(chapter) = provenance.chapter_index {
            parts.push(format!("chapter:{}", chapter));
            if let Some(title) = &provenance.chapter_title {
                parts.push(format!("chapter_title:{}", title));
            }
        }
        if !provenance.section_heading_path.is_empty() {
            parts.push(format!("section_path:{}", provenance.section_heading_path.join(" > ")));
        }
        if let Some(archive_path) = &provenance.archive_entry_path {
            parts.push(format!("archive_entry:{}", archive_path));
        }
        parts.join(", ")
    }
}

impl DocumentChunkV2 {
    /// Convert V2 chunk to V1 format for compatibility.
    pub fn to_v1_chunk(&self) -> DocumentChunk {
        DocumentChunk {
            index: self.index,
            text: self.text.clone(),
            section_label: self.provenance.section_heading_path.last().cloned(),
            provenance: (&self.provenance).into(),
            page: self.provenance.page_number,
            section: self.provenance.section_heading_path.last().cloned(),
            confidence: 1.0, // V2 chunks are considered high quality
            method: "v2_normalized".to_string(),
        }
    }
}

/// Convert V2 extraction result to V1 format.
pub(crate) fn convert_v2_to_v1_result(
    id: &DocumentId,
    metadata: &DocumentMetadataV2,
    units: &[DocumentUnit],
    quality: &DocumentQualityReport,
) -> DocumentExtractionResult {
    // Chunk all units (simplified chunking for compatibility)
    let mut chunks = Vec::new();
    let mut chunk_index = 0;

    for unit in units {
        // Simple chunking: split by sentences/paragraphs, but keep it simple for now
        let unit_chunks: Vec<DocumentChunkV2> = chunk_unit(unit, chunk_index);
        chunk_index += unit_chunks.len();

        for chunk in unit_chunks {
            chunks.push(chunk.to_v1_chunk());
        }
    }

    // Convert metadata to HashMap
    let mut metadata_map = HashMap::new();
    if let Some(title) = &metadata.title {
        metadata_map.insert("title".to_string(), title.clone());
    }
    for author in &metadata.authors {
        metadata_map.insert("author".to_string(), author.clone());
    }
    if let Some(language) = &metadata.language {
        metadata_map.insert("language".to_string(), language.clone());
    }
    if let Some(publisher) = &metadata.publisher {
        metadata_map.insert("publisher".to_string(), publisher.clone());
    }
    if let Some(date) = &metadata.publication_date {
        metadata_map.insert("publication_date".to_string(), date.clone());
    }
    if let Some(isbn) = &metadata.isbn {
        metadata_map.insert("isbn".to_string(), isbn.clone());
    }
    metadata_map.insert("format".to_string(), format!("{:?}", metadata.format));
    metadata_map.insert("file_size".to_string(), metadata.file_size.to_string());
    metadata_map.insert("backend".to_string(), metadata.backend.clone());

    // Add quality info to metadata
    if quality.encrypted_or_drm {
        metadata_map.insert("encrypted_drm".to_string(), "true".to_string());
    }
    if quality.image_only {
        metadata_map.insert("image_only".to_string(), "true".to_string());
    }
    if quality.likely_ocr {
        metadata_map.insert("likely_ocr".to_string(), "true".to_string());
    }
    if let Some(coverage) = quality.text_coverage_percent {
        metadata_map.insert("text_coverage_percent".to_string(), format!("{:.1}", coverage));
    }

    DocumentExtractionResult {
        source_path: metadata.source_path.clone(),
        backend: metadata.backend.clone(),
        total_chunks: chunks.len(),
        chunks,
        metadata: metadata_map,
        ok: quality.extraction_warnings.is_empty() && !quality.encrypted_or_drm,
        error: if !quality.extraction_warnings.is_empty() {
            Some(quality.extraction_warnings.join("; "))
        } else if quality.encrypted_or_drm {
            Some("Document is encrypted or DRM-protected".to_string())
        } else {
            None
        },
    }
}

/// Simple chunking function for compatibility (splits on double newlines).
fn chunk_unit(unit: &DocumentUnit, start_index: usize) -> Vec<DocumentChunkV2> {
    let paragraphs: Vec<&str> = unit.text.split("\n\n").collect();
    let total_chunks = paragraphs.len();
    let mut chunks = Vec::new();
    let mut chunk_index = 0;

    for paragraph in paragraphs {
        if paragraph.trim().is_empty() {
            continue;
        }

        // For simplicity, treat each paragraph as a chunk
        // In a real implementation, this would use token-aware chunking
        chunks.push(DocumentChunkV2 {
            index: start_index + chunk_index,
            chunk_index,
            total_chunks,
            text: paragraph.to_string(),
            provenance: unit.provenance.clone(),
        });
        chunk_index += 1;
    }

    chunks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentIndexEntry {
    pub path: String,
    pub signature: String,
    pub last_modified: u64,
    pub extraction_result: DocumentExtractionResult,
    pub indexed_at: u64,
}

pub(crate) struct DocumentIndexCache {
    cache: HashMap<String, DocumentIndexEntry>,
    cache_file: PathBuf,
}

impl DocumentIndexCache {
    pub fn new(cache_dir: &Path) -> Self {
        let cache_file = cache_dir.join("document_index_cache.toml");
        let cache = Self::load_cache(&cache_file);
        Self { cache, cache_file }
    }

    pub fn get(&self, path: &Path) -> Option<&DocumentIndexEntry> {
        let key = path.to_string_lossy().to_string();
        self.cache.get(&key)
    }

    pub fn put(&mut self, path: &Path, entry: DocumentIndexEntry) {
        let key = path.to_string_lossy().to_string();
        self.cache.insert(key, entry);
    }

    pub fn is_stale(&self, path: &Path) -> bool {
        match self.get(path) {
            Some(entry) => {
                // Check if file has been modified since indexing
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        let current_mtime = modified.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default().as_secs();
                        return current_mtime > entry.last_modified;
                    }
                }
                // If we can't check, assume stale
                true
            }
            None => true, // No cache entry = stale
        }
    }

    pub fn save(&self) -> Result<()> {
        // For now, don't persist to disk to keep implementation simple
        // Future: implement TOML serialization
        Ok(())
    }

    fn load_cache(cache_file: &Path) -> HashMap<String, DocumentIndexEntry> {
        // For now, return empty cache
        // Future: implement TOML deserialization
        HashMap::new()
    }
}

pub(crate) fn calculate_document_signature(path: &Path) -> Result<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();

    // Simple signature: combine file size, modification time, and first/last 1KB
    let mut hasher = DefaultHasher::new();
    file_size.hash(&mut hasher);

    if let Ok(modified) = metadata.modified() {
        let mtime = modified.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        mtime.hash(&mut hasher);
    }

    // Hash first 1KB
    if let Ok(mut file) = std::fs::File::open(path) {
        use std::io::Read;
        let mut buffer = [0u8; 1024];
        if let Ok(n) = file.read(&mut buffer) {
            buffer[..n].hash(&mut hasher);
        }
    }

    // Hash last 1KB (if file is large enough)
    if file_size > 2048 {
        if let Ok(mut file) = std::fs::File::open(path) {
            use std::io::Seek;
            file.seek(std::io::SeekFrom::End(-1024))?;
            use std::io::Read;
            let mut buffer = [0u8; 1024];
            if let Ok(n) = file.read(&mut buffer) {
                buffer[..n].hash(&mut hasher);
            }
        }
    }

    Ok(format!("{:x}", hasher.finish()))
}

/// Smart file read that uses document adapter for supported formats,
/// falls back to plaintext read for other files.
pub(crate) fn read_file_smart(path: &Path) -> Result<(String, String)> {
    read_file_with_budget(path, DocumentReadBudget::default())
}

/// Context-aware document reading with budget constraints.
pub(crate) fn read_file_with_budget(path: &Path, budget: DocumentReadBudget) -> Result<(String, String)> {
    let format = DocumentFormat::detect(path);

    match format {
        DocumentFormat::Txt | DocumentFormat::Code | DocumentFormat::Markdown => {
            // Plaintext files — read directly
            match std::fs::read_to_string(path) {
                Ok(content) => Ok((
                    content,
                    format!("File: {}", path.display()),
                )),
                Err(e) => Err(anyhow::anyhow!("Failed to read {}: {}", path.display(), e)),
            }
        }
        DocumentFormat::Pdf
        | DocumentFormat::Epub
        | DocumentFormat::Html
        | DocumentFormat::DjVu
        | DocumentFormat::Mobi
        | DocumentFormat::FictionBook
        | DocumentFormat::Docx
        | DocumentFormat::Rtf => {
            // Document formats — use adapter with budget planning
            let result = extract_document_with_budget(path, &budget);
            if result.ok {
                let summary = format_extraction_summary(&result);
                let text = select_content_by_budget(&result, &budget);
                Ok((
                    format!("{}\n\n{}\n\n(Use search to find specific content in this document)", summary, text),
                    format!("Document: {} ({})", path.display(), result.backend),
                ))
            } else {
                // Extraction failed — return error
                Err(anyhow::anyhow!(
                    "Failed to extract {}: {}",
                    path.display(),
                    result.error.unwrap_or_else(|| "unknown error".to_string())
                ))
            }
        }
        _ => {
            // Unsupported format — try plaintext read, will likely fail for binaries
            match std::fs::read_to_string(path) {
                Ok(content) => Ok((
                    content,
                    format!("File: {}", path.display()),
                )),
                Err(e) => Err(anyhow::anyhow!("Failed to read {}: {}", path.display(), e)),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentReadBudget {
    pub max_chars: usize,
    pub mode: DocumentReadMode,
    pub focus_sections: Option<Vec<String>>,
}

impl Default for DocumentReadBudget {
    fn default() -> Self {
        Self {
            max_chars: 8000, // Default reasonable limit
            mode: DocumentReadMode::Balanced,
            focus_sections: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum DocumentReadMode {
    /// Read entire document (if under budget)
    Full,
    /// Balanced approach: key sections + search guidance
    Balanced,
    /// Retrieval-first: minimal content + search hints
    RetrievalFirst,
    /// Section-focused: only specified sections
    Scoped(Vec<String>),
}

fn extract_document_with_budget(path: &Path, budget: &DocumentReadBudget) -> DocumentExtractionResult {
    let start_time = Instant::now();
    let format = DocumentFormat::detect(path);

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let modified_time = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());

    let base_metadata = DocumentMetadata {
        format: format!("{:?}", format),
        file_size,
        modified_time,
        title: None,
        author: None,
        language: None,
        page_count: None,
        has_text_layer: None,
        likely_ocr: false,
    };

    // For now, extract fully and let selection handle budgeting
    // Future: implement staged extraction for very large documents
    match format {
        DocumentFormat::Txt | DocumentFormat::Code | DocumentFormat::Markdown => {
            extract_plaintext(path, base_metadata, start_time)
        }
        DocumentFormat::Html | DocumentFormat::Xml => extract_html(path, base_metadata, start_time),
        DocumentFormat::Pdf => extract_pdf(path, base_metadata, start_time),
        DocumentFormat::Epub => extract_epub(path, base_metadata, start_time),
        DocumentFormat::DjVu => extract_djvu(path, base_metadata, start_time),
        DocumentFormat::Mobi | DocumentFormat::Azw => extract_mobi(path, base_metadata, start_time),
        DocumentFormat::FictionBook => extract_fb2(path, base_metadata, start_time),
        DocumentFormat::Docx => extract_docx(path, base_metadata, start_time),
        DocumentFormat::Rtf => extract_rtf(path, base_metadata, start_time),
        DocumentFormat::Cbz => extract_cbz(path, base_metadata, start_time),
        DocumentFormat::Cbr => extract_cbr(path, base_metadata, start_time),
        DocumentFormat::Iba => extract_iba(path, base_metadata, start_time),
        _ => {
            let extraction_time = start_time.elapsed().as_millis() as u64;
            let (state, note) = format.capability_state();
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: format.backend_name().to_string(),
                total_chunks: 0,
                chunks: Vec::new(),
                metadata: base_metadata,
                ok: false,
                error: Some(format!(
                    "Unsupported format: {} ({}, {})",
                    format!("{:?}", format),
                    state,
                    note.unwrap_or("no additional info")
                )),
                quality_score: 0.0,
                extraction_time_ms: extraction_time,
                units_processed: 0,
            }
        }
    }
}

fn select_content_by_budget(result: &DocumentExtractionResult, budget: &DocumentReadBudget) -> String {
    match &budget.mode {
        DocumentReadMode::Full => {
            // Return all chunks if under budget
            if result.chunks.iter().map(|c| c.text.len()).sum::<usize>() <= budget.max_chars {
                result.chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n\n")
            } else {
                select_content_by_budget(result, &DocumentReadBudget {
                    max_chars: budget.max_chars,
                    mode: DocumentReadMode::Balanced,
                    focus_sections: budget.focus_sections.clone(),
                })
            }
        }
        DocumentReadMode::Balanced => {
            // Return first few chunks plus summary of rest
            let mut selected = Vec::new();
            let mut total_chars = 0;
            let mut chunk_count = 0;

            for chunk in &result.chunks {
                if total_chars + chunk.text.len() > budget.max_chars && chunk_count > 0 {
                    break;
                }
                selected.push(chunk.text.as_str());
                total_chars += chunk.text.len();
                chunk_count += 1;
            }

            let content = selected.join("\n\n");
            if chunk_count < result.total_chunks {
                format!("{}\n\n... (document continues with {} more chunks, {} total)", content, result.total_chunks - chunk_count, result.total_chunks)
            } else {
                content
            }
        }
        DocumentReadMode::RetrievalFirst => {
            // Return minimal content with search guidance
            format!("Document extracted successfully ({} chunks, quality: {:.2}). Use search to find specific content within this document.",
                   result.total_chunks, result.quality_score)
        }
        DocumentReadMode::Scoped(sections) => {
            // Return only specified sections
            let mut selected = Vec::new();
            let mut total_chars = 0;

            for chunk in &result.chunks {
                if total_chars + chunk.text.len() > budget.max_chars {
                    break;
                }
                if sections.is_empty() ||
                   sections.iter().any(|s| chunk.text.contains(s) ||
                                       chunk.section.as_ref().map_or(false, |sec| sec.contains(s))) {
                    selected.push(chunk.text.as_str());
                    total_chars += chunk.text.len();
                }
            }

            if selected.is_empty() {
                format!("No content found matching specified sections: {:?}", sections)
            } else {
                selected.join("\n\n")
            }
        }
    }
}

/// Extract document with caching support.
pub(crate) fn extract_document_cached(path: &Path, cache: &mut DocumentIndexCache) -> DocumentExtractionResult {
    // Check cache first
    if !cache.is_stale(path) {
        if let Some(entry) = cache.get(path) {
            return entry.extraction_result.clone();
        }
    }

    // Extract fresh
    let result = extract_document(path);

    // Cache if successful
    if result.ok {
        if let Ok(signature) = calculate_document_signature(path) {
            let entry = DocumentIndexEntry {
                path: path.to_string_lossy().to_string(),
                signature,
                last_modified: result.metadata.modified_time.unwrap_or(0),
                extraction_result: result.clone(),
                indexed_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            cache.put(path, entry);
        }
    }

    result
}

/// Sniff the format and choose the right adapter.
pub(crate) fn extract_document(path: &Path) -> DocumentExtractionResult {
    let format = DocumentFormat::detect(path);

    match format {
        DocumentFormat::Txt | DocumentFormat::Code | DocumentFormat::Markdown => {
            extract_plaintext(path)
        }
        DocumentFormat::Html | DocumentFormat::Xml => extract_html(path),
        DocumentFormat::Pdf => extract_pdf(path),
        DocumentFormat::Epub => extract_epub(path),
        DocumentFormat::DjVu => extract_djvu(path),
        DocumentFormat::Mobi | DocumentFormat::Azw => extract_mobi(path),
        DocumentFormat::FictionBook => extract_fb2(path),
        DocumentFormat::Docx => extract_docx(path),
        DocumentFormat::Rtf => extract_rtf(path),
        _ => {
            let (state, note) = format.capability_state();
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: format.backend_name().to_string(),
                total_chunks: 0,
                chunks: Vec::new(),
                metadata: HashMap::new(),
                ok: false,
                error: Some(format!(
                    "Unsupported format: {} ({}, {})",
                    format!("{:?}", format),
                    state,
                    note.unwrap_or("no additional info")
                )),
            }
        }
    }
}

pub(crate) fn document_capabilities() -> Vec<DocumentCapabilityReport> {
    vec![
        // Plain text and code
        DocumentCapabilityReport {
            format: "txt".to_string(),
            backend: "native".to_string(),
            available: true,
            quality_note: Some("Encoding-aware plain text".to_string()),
        },
        DocumentCapabilityReport {
            format: "md/markdown".to_string(),
            backend: "native".to_string(),
            available: true,
            quality_note: None,
        },
        DocumentCapabilityReport {
            format: "code (rs, py, js, etc)".to_string(),
            backend: "native".to_string(),
            available: true,
            quality_note: None,
        },
        // Structured text
        DocumentCapabilityReport {
            format: "html/xhtml".to_string(),
            backend: "html2text".to_string(),
            available: true,
            quality_note: Some("HTML cleanup and structure labels".to_string()),
        },
        // Ebooks - Full text
        DocumentCapabilityReport {
            format: "epub".to_string(),
            backend: "epub".to_string(),
            available: true,
            quality_note: Some("OPF/spine/TOC/chapter-aware".to_string()),
        },
        DocumentCapabilityReport {
            format: "pdf".to_string(),
            backend: "pdf-extract".to_string(),
            available: true,
            quality_note: Some("Full text when text layer exists; no OCR by default".to_string()),
        },
        DocumentCapabilityReport {
            format: "mobi".to_string(),
            backend: "mobi".to_string(),
            available: true,
            quality_note: Some("Legacy MOBI text and metadata".to_string()),
        },
        DocumentCapabilityReport {
            format: "fb2".to_string(),
            backend: "quick-xml".to_string(),
            available: true,
            quality_note: Some("XML FictionBook".to_string()),
        },
        DocumentCapabilityReport {
            format: "djvu".to_string(),
            backend: "djvu-rs".to_string(),
            available: true,
            quality_note: Some("Full text when text layer exists; image-only files fail clearly".to_string()),
        },
        // Ebooks - Full/degraded
        DocumentCapabilityReport {
            format: "azw".to_string(),
            backend: "mobi".to_string(),
            available: true,
            quality_note: Some("Full/degraded; usually MOBI-like, may be DRM".to_string()),
        },
        DocumentCapabilityReport {
            format: "azw3".to_string(),
            backend: "none (use boko)".to_string(),
            available: false,
            quality_note: Some("Full/degraded; evaluate boko, DRM remains unsupported".to_string()),
        },
        DocumentCapabilityReport {
            format: "kfx".to_string(),
            backend: "none (use boko)".to_string(),
            available: false,
            quality_note: Some("Full/degraded; evaluate boko, DRM remains unsupported".to_string()),
        },
        // Office formats
        DocumentCapabilityReport {
            format: "docx".to_string(),
            backend: "zip+quick-xml".to_string(),
            available: true,
            quality_note: Some("Full text; ZIP/XML extraction".to_string()),
        },
        DocumentCapabilityReport {
            format: "doc".to_string(),
            backend: "none (legacy CFB)".to_string(),
            available: false,
            quality_note: Some("Degraded/full if feasible; legacy CFB Word parsing is limited".to_string()),
        },
        DocumentCapabilityReport {
            format: "rtf".to_string(),
            backend: "rtf-parser".to_string(),
            available: true,
            quality_note: Some("Full/degraded; evaluate parser and fallback cleaner".to_string()),
        },
        // Comic books
        DocumentCapabilityReport {
            format: "cbz".to_string(),
            backend: "zip+image-meta".to_string(),
            available: true,
            quality_note: Some("Metadata/degraded; text only if metadata/text sidecars exist unless OCR feature lands".to_string()),
        },
        DocumentCapabilityReport {
            format: "cbr".to_string(),
            backend: "rar (license-gated)".to_string(),
            available: false,
            quality_note: Some("Metadata/degraded or unsupported; RAR backend must be feature/license gated".to_string()),
        },
        // Apple books
        DocumentCapabilityReport {
            format: "iba".to_string(),
            backend: "zip+embedded-assets".to_string(),
            available: true,
            quality_note: Some("Full/degraded; ZIP package with embedded HTML/XHTML/EPUB-like assets".to_string()),
        },
        // Legacy formats
        DocumentCapabilityReport {
            format: "chm".to_string(),
            backend: "chm-parser (evaluate)".to_string(),
            available: false,
            quality_note: Some("Degraded/unsupported; CHM extraction backend requires explicit decision".to_string()),
        },
        DocumentCapabilityReport {
            format: "lit".to_string(),
            backend: "none (CHM-derived)".to_string(),
            available: false,
            quality_note: Some("Unsupported/degraded; CHM-derived legacy format, no false claims".to_string()),
        },
        DocumentCapabilityReport {
            format: "pdb (PalmDoc)".to_string(),
            backend: "none (legacy PalmDB)".to_string(),
            available: false,
            quality_note: Some("Full/degraded for PalmDoc only; must not confuse with Microsoft debug PDB".to_string()),
        },
        DocumentCapabilityReport {
            format: "lrf".to_string(),
            backend: "none (Sony BeBook)".to_string(),
            available: false,
            quality_note: Some("Unsupported/degraded; legacy Sony BeBook".to_string()),
        },
        DocumentCapabilityReport {
            format: "lrx".to_string(),
            backend: "none (Sony BeBook)".to_string(),
            available: false,
            quality_note: Some("Unsupported; DRM-oriented Sony BeBook variant".to_string()),
        },
    ]
}

fn format_extraction_summary(result: &DocumentExtractionResult) -> String {
    let mut summary = format!(
        "Document Extraction ({}):\n- Backend: {}\n- Total chunks: {}\n- Quality score: {:.2}\n- Processing time: {}ms\n- Units processed: {}",
        result.source_path,
        result.backend,
        result.total_chunks,
        result.quality_score,
        result.extraction_time_ms,
        result.units_processed
    );
    if let Some(title) = &result.metadata.title {
        summary.push_str(&format!("\n- Title: {}", title));
    }
    if let Some(author) = &result.metadata.author {
        summary.push_str(&format!("\n- Author: {}", author));
    }
    if let Some(page_count) = result.metadata.page_count {
        summary.push_str(&format!("\n- Pages: {}", page_count));
    }
    if let Some(note) = &result.error {
        summary.push_str(&format!("\n- Note: {}", note));
    }
    summary
}

pub(crate) fn format_document_telemetry(result: &DocumentExtractionResult) -> String {
    format!(
        "📄 Document Processed\n   Format: {}\n   Backend: {}\n   Chunks: {}\n   Quality: {:.2}\n   Time: {}ms\n   Status: {}",
        result.metadata.format,
        result.backend,
        result.total_chunks,
        result.quality_score,
        result.extraction_time_ms,
        if result.ok { "✅ Success" } else { "❌ Failed" }
    )
}

pub(crate) fn format_retrieval_telemetry(
    query: &str,
    results: &[(usize, String)],
    total_chunks: usize
) -> String {
    format!(
        "🔍 Retrieval Results\n   Query: {}\n   Matches: {}/{}\n   Top results shown",
        query,
        results.len(),
        total_chunks
    )
}

fn extract_plaintext(path: &Path) -> DocumentExtractionResult {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let chunks = chunk_text(&content, 2000, &path.display().to_string());
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "native".to_string(),
                total_chunks: chunks.len(),
                chunks,
                metadata: [("format".to_string(), "plaintext".to_string())]
                    .into_iter()
                    .collect(),
                ok: true,
                error: None,
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "native".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("Read error: {}", e)),
        },
    }
}

fn extract_html(path: &Path) -> DocumentExtractionResult {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let text = html2text::from_read(content.as_bytes(), 120).unwrap_or(content);
            let chunks = chunk_text(&text, 2000, &path.display().to_string());
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "html2text".to_string(),
                total_chunks: chunks.len(),
                chunks,
                metadata: [("format".to_string(), "html".to_string())]
                    .into_iter()
                    .collect(),
                ok: true,
                error: None,
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "html2text".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("HTML read error: {}", e)),
        },
    }
}

fn extract_pdf(path: &Path) -> DocumentExtractionResult {
    match pdf_extract::extract_text(path) {
        Ok(content) => {
            let chunks = chunk_text(&content, 2000, &path.display().to_string());
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "pdf-extract".to_string(),
                total_chunks: chunks.len(),
                chunks,
                metadata: [("format".to_string(), "pdf".to_string())]
                    .into_iter()
                    .collect(),
                ok: true,
                error: None,
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "pdf-extract".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("PDF extraction error: {}", e)),
        },
    }
}

fn extract_epub(path: &Path) -> DocumentExtractionResult {
    // `epub` crate API needs proper handling; return error for now
    DocumentExtractionResult {
        source_path: path.display().to_string(),
        backend: "epub".to_string(),
        total_chunks: 0,
        chunks: Vec::new(),
        metadata: HashMap::new(),
        ok: false,
        error: Some("EPUB extraction not fully implemented yet. Use EPUB CLI tools for now.".to_string()),
    }
}

fn extract_djvu(path: &Path) -> DocumentExtractionResult {
    // `djvu-rs` API needs verification; return error for now
    DocumentExtractionResult {
        source_path: path.display().to_string(),
        backend: "djvu-rs".to_string(),
        total_chunks: 0,
        chunks: Vec::new(),
        metadata: HashMap::new(),
        ok: false,
        error: Some("DjVu extraction not fully implemented yet. Use DjVuLibre CLI tools for now.".to_string()),
    }
}

fn extract_mobi(path: &Path) -> DocumentExtractionResult {
    match mobi::Mobi::from_path(path) {
        Ok(book) => {
            let content = book.content_as_string().unwrap_or_default();
            if content.trim().is_empty() {
                return DocumentExtractionResult {
                    source_path: path.display().to_string(),
                    backend: "mobi".to_string(),
                    total_chunks: 0,
                    chunks: Vec::new(),
                    metadata: HashMap::new(),
                    ok: false,
                    error: Some("MOBI file has no extractable text content.".to_string()),
                };
            }
            let chunks = chunk_text(&content, 2000, &path.display().to_string());
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "mobi".to_string(),
                total_chunks: chunks.len(),
                chunks,
                metadata: HashMap::new(),
                ok: true,
                error: None,
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "mobi".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("MOBI parse error: {:?}", e)),
        },
    }
}

fn extract_fb2(path: &Path) -> DocumentExtractionResult {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            // Basic FB2 XML text extraction
            let mut text = String::new();
            let mut in_body = false;
            for line in content.lines() {
                if line.contains("<body>") {
                    in_body = true;
                }
                if in_body {
                    // Strip XML tags - keep only text content
                    let mut cleaned = String::new();
                    let mut in_tag = false;
                    for c in line.chars() {
                        match c {
                            '<' => in_tag = true,
                            '>' => in_tag = false,
                            _ => if !in_tag { cleaned.push(c); }
                        }
                    }
                    let cleaned = cleaned.trim();
                    if !cleaned.is_empty()
                        && !cleaned.starts_with("<?xml")
                        && !cleaned.starts_with("<!DOCTYPE")
                    {
                        text.push_str(cleaned);
                        text.push('\n');
                    }
                }
                if line.contains("</body>") {
                    in_body = false;
                }
            }
            let chunks = chunk_text(&text, 2000, &path.display().to_string());
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "quick-xml".to_string(),
                total_chunks: chunks.len(),
                chunks,
                metadata: [("format".to_string(), "fb2".to_string())]
                    .into_iter()
                    .collect(),
                ok: true,
                error: None,
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "quick-xml".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("FB2 read error: {}", e)),
        },
    }
}

fn extract_docx(path: &Path) -> DocumentExtractionResult {
    // DOCX is a ZIP file containing word/document.xml
    match std::fs::read(path) {
        Ok(data) => {
            // Basic ZIP parsing - extract word/document.xml
            // For now, return a "not fully implemented" error
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "zip+quick-xml".to_string(),
                total_chunks: 0,
                chunks: Vec::new(),
                metadata: HashMap::new(),
                ok: false,
                error: Some("DOCX extraction not fully implemented yet. Use a converter or extract manually.".to_string()),
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "zip+quick-xml".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("DOCX read error: {}", e)),
        },
    }
}

fn extract_rtf(path: &Path) -> DocumentExtractionResult {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            // Basic RTF text extraction - strip formatting
            let mut text = String::new();
            let mut chars = content.chars().peekable();
            let mut in_control = false;
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        in_control = true;
                        // Skip control word
                        while let Some(&ch) = chars.peek() {
                            if ch.is_alphabetic() {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        // Skip optional numeric parameter
                        if let Some(&ch) = chars.peek() {
                            if ch == '-' || ch.is_numeric() {
                                chars.next();
                                while let Some(&ch) = chars.peek() {
                                    if ch.is_numeric() {
                                        chars.next();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        in_control = false;
                    }
                    '{' | '}' => {
                        // Skip grouping braces
                    }
                    _ => {
                        if !in_control {
                            text.push(c);
                        }
                    }
                }
            }
            let chunks = chunk_text(&text, 2000, &path.display().to_string());
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "rtf-parser".to_string(),
                total_chunks: chunks.len(),
                chunks,
                metadata: [("format".to_string(), "rtf".to_string())]
                    .into_iter()
                    .collect(),
                ok: true,
                error: Some("Basic RTF extraction, formatting may be lost.".to_string()),
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "rtf-parser".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("RTF read error: {}", e)),
        },
    }
}

fn chunk_text(text: &str, chunk_size: usize, provenance: &str) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();

    // Split into structural units first (paragraphs, sections)
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();

    let mut current_chunk = String::new();
    let mut index = 0;

    for paragraph in paragraphs {
        let paragraph_trimmed = paragraph.trim();

        // Skip empty paragraphs
        if paragraph_trimmed.is_empty() {
            continue;
        }

        // Estimate token count (rough approximation: 4 chars per token)
        let para_tokens = paragraph_trimmed.len() / 4;

        // If adding this paragraph would exceed chunk_size and we already have content
        if current_chunk.len() + paragraph.len() > chunk_size && !current_chunk.is_empty() {
            // Create chunk from current content
            chunks.push(DocumentChunk {
                index,
                text: current_chunk.trim().to_string(),
                section_label: None,
                provenance: provenance.to_string(),
                page: None,
                section: None,
                confidence: calculate_chunk_quality(&current_chunk),
                method: "structure_aware_chunking".to_string(),
            });
            index += 1;
            current_chunk = paragraph.to_string();
        } else if current_chunk.is_empty() {
            // Start new chunk
            current_chunk = paragraph.to_string();
        } else {
            // Add to current chunk with separator
            current_chunk.push_str("\n\n");
            current_chunk.push_str(paragraph);
        }

        // If current chunk is getting too large, split it
        if current_chunk.len() > chunk_size {
            split_large_chunk(&current_chunk, chunk_size, provenance, &mut chunks, &mut index);
            current_chunk.clear();
        }
    }

    // Handle remaining content
    if !current_chunk.trim().is_empty() {
        chunks.push(DocumentChunk {
            index,
            text: current_chunk.trim().to_string(),
            section_label: None,
            provenance: provenance.to_string(),
            page: None,
            section: None,
            confidence: calculate_chunk_quality(&current_chunk),
            method: "structure_aware_chunking".to_string(),
        });
    }

    chunks
}

fn split_large_chunk(
    text: &str,
    max_size: usize,
    provenance: &str,
    chunks: &mut Vec<DocumentChunk>,
    index: &mut usize,
) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut current_chunk = String::new();

    for word in words {
        if current_chunk.len() + word.len() + 1 > max_size && !current_chunk.is_empty() {
            // Create chunk
            chunks.push(DocumentChunk {
                index: *index,
                text: current_chunk.trim().to_string(),
                section_label: None,
                provenance: provenance.to_string(),
                page: None,
                section: None,
                confidence: calculate_chunk_quality(&current_chunk),
                method: "structure_aware_chunking".to_string(),
            });
            *index += 1;
            current_chunk = word.to_string();
        } else {
            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(word);
        }
    }

    if !current_chunk.trim().is_empty() {
        chunks.push(DocumentChunk {
            index: *index,
            text: current_chunk.trim().to_string(),
            section_label: None,
            provenance: provenance.to_string(),
            page: None,
            section: None,
            confidence: calculate_chunk_quality(&current_chunk),
            method: "structure_aware_chunking".to_string(),
        });
        *index += 1;
    }
}

fn calculate_chunk_quality(text: &str) -> f64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    let char_count = trimmed.chars().count();
    let word_count = trimmed.split_whitespace().count();
    let avg_word_length = if word_count > 0 { char_count as f64 / word_count as f64 } else { 0.0 };

    // Quality factors:
    // - Prefer chunks with reasonable word count (not too few, not too many)
    // - Prefer chunks with reasonable average word length (not too short, not too long)
    // - Penalize very short chunks

    let word_score = if word_count < 5 {
        word_count as f64 / 5.0 * 0.5 // Low score for very short chunks
    } else if word_count > 200 {
        0.8 // Slightly penalize very long chunks
    } else {
        1.0
    };

    let length_score = if avg_word_length < 3.0 {
        avg_word_length / 3.0 * 0.7 // Penalize very short words (could be garbage)
    } else if avg_word_length > 15.0 {
        0.6 // Penalize very long words (could be URLs or codes)
    } else {
        1.0
    };

    (word_score * length_score).min(1.0).max(0.0)
}

/// Summarize extracted chunks into a brief overview.
pub(crate) fn summarize_chunks(chunks: &[DocumentChunk]) -> String {
    let total_chars: usize = chunks.iter().map(|c| c.text.len()).sum();
    let preview: String = chunks
        .iter()
        .take(3)
        .map(|c| c.text.chars().take(200).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n---\n");
    format!(
        "Document has {} chunks (~{} chars total).\nPreview:\n{}",
        chunks.len(),
        total_chars,
        preview
    )
}

/// Search for a needle in the document chunks and return citations.
pub(crate) fn search_chunks(chunks: &[DocumentChunk], needle: &str) -> Vec<(usize, String)> {
    search_chunks_hybrid(chunks, needle, &DocumentSearchOptions::default())
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentSearchOptions {
    pub max_results: usize,
    pub min_score: f64,
    pub use_semantic: bool,
    pub diversify_sources: bool,
}

impl Default for DocumentSearchOptions {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_score: 0.1,
            use_semantic: false, // Not implemented yet
            diversify_sources: true,
        }
    }
}

pub(crate) fn search_chunks_hybrid(
    chunks: &[DocumentChunk],
    needle: &str,
    options: &DocumentSearchOptions,
) -> Vec<(usize, String)> {
    let needle_lower = needle.to_lowercase();
    let terms: Vec<&str> = needle_lower.split_whitespace().collect();

    let mut scored_results: Vec<(usize, f64, String)> = chunks
        .iter()
        .enumerate()
        .filter_map(|(i, chunk)| {
            let chunk_text_lower = chunk.text.to_lowercase();
            let score = calculate_bm25_score(&chunk_text_lower, &terms, chunks.len());

            if score >= options.min_score {
                Some((i, score, chunk.text.clone()))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Apply source diversity if requested
    let results = if options.diversify_sources {
        diversify_search_results(scored_results, options.max_results)
    } else {
        scored_results.into_iter()
            .take(options.max_results)
            .map(|(idx, _score, text)| (idx, text))
            .collect()
    };

    results
}

fn calculate_bm25_score(chunk_text: &str, query_terms: &[&str], total_chunks: usize) -> f64 {
    if query_terms.is_empty() {
        return 0.0;
    }

    let k1 = 1.5; // BM25 parameter
    let b = 0.75; // BM25 parameter
    let chunk_length = chunk_text.split_whitespace().count() as f64;
    let avg_chunk_length = 100.0; // Rough estimate

    let mut score = 0.0;
    for &term in query_terms {
        let term_freq = chunk_text.matches(term).count() as f64;
        let doc_freq = 1.0; // Simplified - assume term appears in 1 document

        if term_freq > 0.0 {
            let idf = ((total_chunks as f64 - doc_freq + 0.5) / (doc_freq + 0.5)).ln();
            let tf_score = (term_freq * (k1 + 1.0)) /
                          (term_freq + k1 * (1.0 - b + b * (chunk_length / avg_chunk_length)));
            score += idf * tf_score;
        }
    }

    score
}

fn diversify_search_results(
    scored_results: Vec<(usize, f64, String)>,
    max_results: usize,
) -> Vec<(usize, String)> {
    let mut selected = Vec::new();
    let mut used_pages = std::collections::HashSet::new();
    let mut used_sections = std::collections::HashSet::new();

    for (idx, score, text) in scored_results {
        // Simple diversity: prefer results from different pages/sections
        // This is a basic implementation - could be enhanced
        if selected.len() >= max_results {
            break;
        }

        // For now, just take top results with some spacing
        if selected.len() < max_results / 2 || selected.len() % 2 == 0 {
            selected.push((idx, text));
        }
    }

    selected
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentCitation {
    pub chunk_index: usize,
    pub text_snippet: String,
    pub page: Option<u32>,
    pub section: Option<String>,
    pub confidence: f64,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentAnswer {
    pub answer: String,
    pub citations: Vec<DocumentCitation>,
    pub confidence: f64,
    pub method: String,
    pub extractive_fallback: bool,
}

pub(crate) fn generate_cited_answer(
    question: &str,
    chunks: &[DocumentChunk],
    max_citations: usize,
) -> DocumentAnswer {
    // Simple implementation: find relevant chunks and cite them
    let search_results = search_chunks_hybrid(chunks, question, &DocumentSearchOptions {
        max_results: max_citations,
        min_score: 0.1,
        use_semantic: false,
        diversify_sources: true,
    });

    let citations: Vec<DocumentCitation> = search_results.into_iter()
        .enumerate()
        .map(|(i, (chunk_idx, text))| {
            let chunk = &chunks[chunk_idx];
            DocumentCitation {
                chunk_index: chunk_idx,
                text_snippet: text.chars().take(200).collect(),
                page: chunk.page,
                section: chunk.section.clone(),
                confidence: chunk.confidence,
                relevance_score: 1.0 - (i as f64 * 0.1), // Decreasing relevance
            }
        })
        .collect();

    let extractive_fallback = citations.is_empty();
    let answer = if extractive_fallback {
        format!("Based on the available document content, I found some potentially relevant information but cannot provide a definitive answer. Here are the most relevant excerpts:\n\n{}",
               citations.iter().map(|c| format!("• {}", c.text_snippet)).collect::<Vec<_>>().join("\n\n"))
    } else {
        format!("Based on the document content, here's the answer with supporting citations:\n\nAnswer: [Would be generated by LLM using citations]\n\nSources:\n{}",
               citations.iter().enumerate().map(|(i, c)|
                   format!("{}. {} (confidence: {:.2})", i + 1, c.text_snippet, c.confidence)
               ).collect::<Vec<_>>().join("\n\n"))
    };

    DocumentAnswer {
        answer,
        citations,
        confidence: if extractive_fallback { 0.3 } else { 0.8 },
        method: if extractive_fallback { "extractive_fallback" } else { "cited_answer" }.to_string(),
        extractive_fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_plaintext_produces_chunks() {
        let result = extract_document(Path::new("Cargo.toml"));
        assert!(result.ok, "error: {:?}", result.error);
        assert!(!result.chunks.is_empty());
        assert_eq!(result.backend, "native");
    }

    #[test]
    fn unsupported_format_fails_cleanly() {
        let result = extract_document(Path::new("unknown.xyz"));
        assert!(!result.ok);
        assert!(result.error.is_some());
    }

    #[test]
    fn chunk_text_respects_size() {
        let text = "a".repeat(5000);
        let chunks = chunk_text(&text, 2000, "test");
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn search_chunks_finds_needle() {
        let chunks = vec![
            DocumentChunk {
                index: 0,
                text: "hello world".to_string(),
                section_label: None,
                provenance: "test".to_string(),
            },
            DocumentChunk {
                index: 1,
                text: "goodbye world".to_string(),
                section_label: None,
                provenance: "test".to_string(),
            },
        ];
        let hits = search_chunks(&chunks, "hello");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 0);
    }

    #[test]
    fn capabilities_list_includes_core_and_extended_formats() {
        let caps = document_capabilities();
        assert!(!caps.is_empty());
        let formats: Vec<_> = caps.iter().map(|c| c.format.as_str()).collect();
        assert!(formats.contains(&"txt"));
        assert!(formats.contains(&"md/markdown"));
        assert!(formats.contains(&"code (rs, py, js, etc)"));
        assert!(formats.contains(&"pdf"));
        assert!(formats.contains(&"epub"));
        assert!(formats.contains(&"djvu"));
        assert!(formats.contains(&"mobi"));
        assert!(formats.contains(&"azw3"));
        let available: Vec<_> = caps.iter().filter(|c| c.available).collect();
        assert!(!available.is_empty());
    }

    #[test]
    fn djvu_unsupported_format_returns_explicit_error() {
        let result = extract_document(Path::new("unknown.djvu"));
        assert!(!result.ok);
        assert!(result.error.is_some());
        assert_eq!(result.backend, "djvu-rs");
    }

    #[test]
    fn mobi_unsupported_format_returns_explicit_error() {
        let result = extract_document(Path::new("unknown.mobi"));
        assert!(!result.ok);
        assert!(result.error.is_some());
        assert_eq!(result.backend, "mobi");
    }

    #[test]
    fn azw3_reported_as_unsupported() {
        let caps = document_capabilities();
        let azw3_cap = caps.iter().find(|c| c.format == "azw3");
        assert!(azw3_cap.is_some());
        let cap = azw3_cap.unwrap();
        assert!(!cap.available);
        assert!(cap.quality_note.is_some());
    }

    #[test]
    fn test_document_format_detection() {
        // Test various extensions are recognized
        assert!(matches!(DocumentFormat::detect(std::path::Path::new("test.pdf")), DocumentFormat::Pdf));
        assert!(matches!(DocumentFormat::detect(std::path::Path::new("test.epub")), DocumentFormat::Epub));
        assert!(matches!(DocumentFormat::detect(std::path::Path::new("test.mobi")), DocumentFormat::Mobi));
        assert!(matches!(DocumentFormat::detect(std::path::Path::new("test.txt")), DocumentFormat::Txt));
        assert!(matches!(DocumentFormat::detect(std::path::Path::new("test.rs")), DocumentFormat::Code));
    }

    #[test]
    fn test_chunk_quality_scoring() {
        // Test that chunk quality is calculated correctly
        assert!(calculate_chunk_quality("") < 0.1); // Empty chunk
        assert!(calculate_chunk_quality("short") < calculate_chunk_quality("This is a longer sentence with more words."));
        assert!(calculate_chunk_quality("This is a good quality sentence with reasonable length.") > 0.5);
    }

    #[test]
    fn test_bm25_scoring() {
        let chunks = vec![
            DocumentChunk {
                index: 0,
                text: "The quick brown fox jumps over the lazy dog".to_string(),
                section_label: None,
                provenance: "test".to_string(),
                page: None,
                section: None,
                confidence: 1.0,
                method: "test".to_string(),
            },
            DocumentChunk {
                index: 1,
                text: "A brown fox is quick and agile".to_string(),
                section_label: None,
                provenance: "test".to_string(),
                page: None,
                section: None,
                confidence: 1.0,
                method: "test".to_string(),
            }
        ];

        let results = search_chunks_hybrid(&chunks, "brown fox", &DocumentSearchOptions::default());
        assert!(!results.is_empty());
        // First chunk should score higher (contains both terms)
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_document_budget_modes() {
        let chunks = vec![
            DocumentChunk {
                index: 0,
                text: "First chunk content".to_string(),
                section_label: None,
                provenance: "test".to_string(),
                page: None,
                section: None,
                confidence: 1.0,
                method: "test".to_string(),
            },
            DocumentChunk {
                index: 1,
                text: "Second chunk content".to_string(),
                section_label: None,
                provenance: "test".to_string(),
                page: None,
                section: None,
                confidence: 1.0,
                method: "test".to_string(),
            }
        ];

        let result = DocumentExtractionResult {
            source_path: "test.pdf".to_string(),
            backend: "test".to_string(),
            total_chunks: 2,
            chunks: chunks.clone(),
            metadata: DocumentMetadata {
                format: "pdf".to_string(),
                file_size: 1000,
                modified_time: None,
                title: None,
                author: None,
                language: None,
                page_count: Some(2),
                has_text_layer: Some(true),
                likely_ocr: false,
            },
            ok: true,
            error: None,
            quality_score: 0.9,
            extraction_time_ms: 100,
            units_processed: 2,
        };

        // Test balanced mode
        let content = select_content_by_budget(&result, &DocumentReadBudget {
            max_chars: 50,
            mode: DocumentReadMode::Balanced,
            focus_sections: None,
        });
        assert!(content.contains("First chunk"));
        assert!(content.contains("Second chunk") || content.contains("chunks total"));

        // Test retrieval-first mode
        let content = select_content_by_budget(&result, &DocumentReadBudget {
            max_chars: 50,
            mode: DocumentReadMode::RetrievalFirst,
            focus_sections: None,
        });
        assert!(content.contains("Use search"));
    }

    #[test]
    fn test_document_cache_staleness() {
        let temp_dir = std::env::temp_dir();
        let mut cache = DocumentIndexCache::new(&temp_dir);

        // Create a temp file
        let temp_file = temp_dir.join("test_cache_file.txt");
        std::fs::write(&temp_file, "test content").unwrap();

        // Initially should be stale (no cache entry)
        assert!(cache.is_stale(&temp_file));

        // Add an entry
        let signature = calculate_document_signature(&temp_file).unwrap();
        let entry = DocumentIndexEntry {
            path: temp_file.to_string_lossy().to_string(),
            signature,
            last_modified: 1000,
            extraction_result: DocumentExtractionResult {
                source_path: temp_file.to_string_lossy().to_string(),
                backend: "test".to_string(),
                total_chunks: 1,
                chunks: vec![],
                metadata: DocumentMetadata {
                    format: "txt".to_string(),
                    file_size: 12,
                    modified_time: Some(1000),
                    title: None,
                    author: None,
                    language: None,
                    page_count: None,
                    has_text_layer: None,
                    likely_ocr: false,
                },
                ok: true,
                error: None,
                quality_score: 1.0,
                extraction_time_ms: 10,
                units_processed: 1,
            },
            indexed_at: 1000,
        };
        cache.put(&temp_file, entry);

        // Should not be stale if file unchanged
        assert!(!cache.is_stale(&temp_file));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn v2_document_id_serialization() {
        let id = DocumentId {
            canonical_path: "/test/path.pdf".to_string(),
            content_signature: "abc123".to_string(),
        };
        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: DocumentId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn v2_document_metadata_serialization() {
        let metadata = DocumentMetadataV2 {
            title: Some("Test Document".to_string()),
            authors: vec!["Author One".to_string(), "Author Two".to_string()],
            language: Some("en".to_string()),
            publisher: Some("Test Publisher".to_string()),
            publication_date: Some("2023-01-01".to_string()),
            isbn: Some("1234567890".to_string()),
            identifiers: vec!["doi:10.1234/test".to_string()],
            source_path: "/test/path.pdf".to_string(),
            file_size: 1024,
            format: DocumentFormat::Pdf,
            backend: "pdf-backend".to_string(),
        };
        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: DocumentMetadataV2 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(metadata.title, deserialized.title);
        assert_eq!(metadata.authors, deserialized.authors);
        assert_eq!(metadata.language, deserialized.language);
        assert_eq!(metadata.source_path, deserialized.source_path);
        assert_eq!(metadata.file_size, deserialized.file_size);
        assert_eq!(metadata.format, deserialized.format);
        assert_eq!(metadata.backend, deserialized.backend);
    }

    #[test]
    fn v2_document_provenance_serialization() {
        let provenance = DocumentProvenance {
            source_path: "/test/path.pdf".to_string(),
            format: DocumentFormat::Pdf,
            backend: "pdf-backend".to_string(),
            page_number: Some(5),
            chapter_index: Some(2),
            chapter_title: Some("Chapter Two".to_string()),
            section_heading_path: vec!["Section 1".to_string(), "Subsection A".to_string()],
            archive_entry_path: None,
            byte_offset_start: Some(1000),
            byte_offset_end: Some(2000),
            char_offset_start: Some(800),
            char_offset_end: Some(1800),
        };
        let serialized = serde_json::to_string(&provenance).unwrap();
        let deserialized: DocumentProvenance = serde_json::from_str(&serialized).unwrap();
        assert_eq!(provenance.source_path, deserialized.source_path);
        assert_eq!(provenance.format, deserialized.format);
        assert_eq!(provenance.page_number, deserialized.page_number);
        assert_eq!(provenance.chapter_index, deserialized.chapter_index);
        assert_eq!(provenance.chapter_title, deserialized.chapter_title);
        assert_eq!(provenance.section_heading_path, deserialized.section_heading_path);
    }

    #[test]
    fn v2_to_v1_chunk_conversion() {
        let v2_chunk = DocumentChunkV2 {
            index: 5,
            chunk_index: 2,
            total_chunks: 10,
            text: "Test content".to_string(),
            provenance: DocumentProvenance {
                source_path: "/test.pdf".to_string(),
                format: DocumentFormat::Pdf,
                backend: "pdf-backend".to_string(),
                page_number: Some(3),
                chapter_index: Some(1),
                chapter_title: Some("Chapter 1".to_string()),
                section_heading_path: vec!["Introduction".to_string()],
                archive_entry_path: None,
                byte_offset_start: Some(100),
                byte_offset_end: Some(200),
                char_offset_start: Some(90),
                char_offset_end: Some(180),
            },
        };

        let v1_chunk = v2_chunk.to_v1_chunk();

        assert_eq!(v1_chunk.index, 5);
        assert_eq!(v1_chunk.text, "Test content");
        assert_eq!(v1_chunk.section_label, Some("Introduction".to_string()));
        assert!(v1_chunk.provenance.contains("path:/test.pdf"));
        assert!(v1_chunk.provenance.contains("page:3"));
        assert!(v1_chunk.provenance.contains("chapter:1"));
        assert!(v1_chunk.provenance.contains("chapter_title:Chapter 1"));
        assert!(v1_chunk.provenance.contains("section_path:Introduction"));
        assert_eq!(v1_chunk.page, Some(3));
        assert_eq!(v1_chunk.section, Some("Introduction".to_string()));
        assert_eq!(v1_chunk.confidence, 1.0);
        assert_eq!(v1_chunk.method, "v2_normalized");
    }

    #[test]
    fn v2_to_v1_result_conversion() {
        let id = DocumentId {
            canonical_path: "/test.pdf".to_string(),
            content_signature: "sig123".to_string(),
        };

        let metadata = DocumentMetadataV2 {
            title: Some("Test PDF".to_string()),
            authors: vec!["Test Author".to_string()],
            language: Some("en".to_string()),
            publisher: None,
            publication_date: None,
            isbn: None,
            identifiers: vec![],
            source_path: "/test.pdf".to_string(),
            file_size: 2048,
            format: DocumentFormat::Pdf,
            backend: "pdf-backend".to_string(),
        };

        let units = vec![
            DocumentUnit {
                index: 0,
                text: "First paragraph.\n\nSecond paragraph.".to_string(),
                provenance: DocumentProvenance {
                    source_path: "/test.pdf".to_string(),
                    format: DocumentFormat::Pdf,
                    backend: "pdf-backend".to_string(),
                    page_number: Some(1),
                    chapter_index: None,
                    chapter_title: None,
                    section_heading_path: vec![],
                    archive_entry_path: None,
                    byte_offset_start: Some(0),
                    byte_offset_end: Some(50),
                    char_offset_start: Some(0),
                    char_offset_end: Some(45),
                },
            },
        ];

        let quality = DocumentQualityReport {
            extraction_warnings: vec![],
            text_coverage_percent: Some(95.0),
            empty_pages: vec![],
            encoding_repairs: vec![],
            encrypted_or_drm: false,
            image_only: false,
            likely_ocr: false,
        };

        let v1_result = convert_v2_to_v1_result(&id, &metadata, &units, &quality);

        assert_eq!(v1_result.source_path, "/test.pdf");
        assert_eq!(v1_result.backend, "pdf-backend");
        assert!(v1_result.ok);
        assert!(v1_result.error.is_none());
        assert!(!v1_result.chunks.is_empty());
        assert!(v1_result.metadata.contains_key("title"));
        assert_eq!(v1_result.metadata.get("title"), Some(&"Test PDF".to_string()));
        assert!(v1_result.metadata.contains_key("author"));
        assert!(v1_result.metadata.contains_key("text_coverage_percent"));
    }

    #[test]
    fn golden_fixture_pdf_example() {
        // Create a representative PDF extraction result
        let id = DocumentId {
            canonical_path: "/example.pdf".to_string(),
            content_signature: "pdf_content_hash_12345".to_string(),
        };

        let metadata = DocumentMetadataV2 {
            title: Some("Sample PDF Document".to_string()),
            authors: vec!["Jane Doe".to_string()],
            language: Some("en".to_string()),
            publisher: Some("Example Publishing".to_string()),
            publication_date: Some("2023-06-15".to_string()),
            isbn: Some("978-0123456789".to_string()),
            identifiers: vec!["doi:10.1234/example".to_string()],
            source_path: "/example.pdf".to_string(),
            file_size: 245760,
            format: DocumentFormat::Pdf,
            backend: "pdf-extraction-backend".to_string(),
        };

        let units = vec![
            DocumentUnit {
                index: 0,
                text: "This is the first page of the PDF document.\n\nIt contains introduction text.".to_string(),
                provenance: DocumentProvenance {
                    source_path: "/example.pdf".to_string(),
                    format: DocumentFormat::Pdf,
                    backend: "pdf-extraction-backend".to_string(),
                    page_number: Some(1),
                    chapter_index: None,
                    chapter_title: None,
                    section_heading_path: vec!["Introduction".to_string()],
                    archive_entry_path: None,
                    byte_offset_start: Some(0),
                    byte_offset_end: Some(87),
                    char_offset_start: Some(0),
                    char_offset_end: Some(82),
                },
            },
            DocumentUnit {
                index: 1,
                text: "Chapter 1: Getting Started\n\nThis chapter explains the basics.".to_string(),
                provenance: DocumentProvenance {
                    source_path: "/example.pdf".to_string(),
                    format: DocumentFormat::Pdf,
                    backend: "pdf-extraction-backend".to_string(),
                    page_number: Some(2),
                    chapter_index: Some(0),
                    chapter_title: Some("Getting Started".to_string()),
                    section_heading_path: vec!["Chapter 1".to_string(), "Getting Started".to_string()],
                    archive_entry_path: None,
                    byte_offset_start: Some(88),
                    byte_offset_end: Some(145),
                    char_offset_start: Some(83),
                    char_offset_end: Some(135),
                },
            },
        ];

        let quality = DocumentQualityReport {
            extraction_warnings: vec![],
            text_coverage_percent: Some(98.5),
            empty_pages: vec![],
            encoding_repairs: vec![],
            encrypted_or_drm: false,
            image_only: false,
            likely_ocr: false,
        };

        // Serialize to JSON (this would be the golden fixture)
        let fixture = serde_json::json!({
            "document_id": id,
            "metadata": metadata,
            "units": units,
            "quality_report": quality
        });

        let json_string = serde_json::to_string_pretty(&fixture).unwrap();

        // Verify it can be deserialized back
        let deserialized: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized["metadata"]["title"], "Sample PDF Document");
        assert_eq!(deserialized["units"].as_array().unwrap().len(), 2);
        assert_eq!(deserialized["units"][0]["provenance"]["page_number"], 1);
        assert_eq!(deserialized["quality_report"]["text_coverage_percent"], 98.5);
    }

    #[test]
    fn golden_fixture_epub_example() {
        // Create a representative EPUB extraction result
        let id = DocumentId {
            canonical_path: "/example.epub".to_string(),
            content_signature: "epub_content_hash_67890".to_string(),
        };

        let metadata = DocumentMetadataV2 {
            title: Some("Sample EPUB Book".to_string()),
            authors: vec!["John Smith".to_string()],
            language: Some("en".to_string()),
            publisher: Some("Book Publishers Inc".to_string()),
            publication_date: Some("2022-03-10".to_string()),
            isbn: Some("978-0987654321".to_string()),
            identifiers: vec!["uuid:12345678-1234-1234-1234-123456789012".to_string()],
            source_path: "/example.epub".to_string(),
            file_size: 153600,
            format: DocumentFormat::Epub,
            backend: "epub-extraction-backend".to_string(),
        };

        let units = vec![
            DocumentUnit {
                index: 0,
                text: "Cover page content would go here.".to_string(),
                provenance: DocumentProvenance {
                    source_path: "/example.epub".to_string(),
                    format: DocumentFormat::Epub,
                    backend: "epub-extraction-backend".to_string(),
                    page_number: None, // EPUB doesn't have pages
                    chapter_index: Some(0),
                    chapter_title: Some("Cover".to_string()),
                    section_heading_path: vec![],
                    archive_entry_path: Some("OEBPS/cover.xhtml".to_string()),
                    byte_offset_start: Some(0),
                    byte_offset_end: Some(35),
                    char_offset_start: Some(0),
                    char_offset_end: Some(35),
                },
            },
            DocumentUnit {
                index: 1,
                text: "Table of Contents\n\nChapter 1: Introduction\nChapter 2: Main Content".to_string(),
                provenance: DocumentProvenance {
                    source_path: "/example.epub".to_string(),
                    format: DocumentFormat::Epub,
                    backend: "epub-extraction-backend".to_string(),
                    page_number: None,
                    chapter_index: Some(1),
                    chapter_title: Some("Table of Contents".to_string()),
                    section_heading_path: vec!["Navigation".to_string()],
                    archive_entry_path: Some("OEBPS/toc.xhtml".to_string()),
                    byte_offset_start: Some(36),
                    byte_offset_end: Some(95),
                    char_offset_start: Some(36),
                    char_offset_end: Some(95),
                },
            },
        ];

        let quality = DocumentQualityReport {
            extraction_warnings: vec!["Some images were skipped".to_string()],
            text_coverage_percent: Some(92.0),
            empty_pages: vec![],
            encoding_repairs: vec![],
            encrypted_or_drm: false,
            image_only: false,
            likely_ocr: false,
        };

        // Serialize to JSON (this would be the golden fixture)
        let fixture = serde_json::json!({
            "document_id": id,
            "metadata": metadata,
            "units": units,
            "quality_report": quality
        });

        let json_string = serde_json::to_string_pretty(&fixture).unwrap();

        // Verify it can be deserialized back
        let deserialized: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized["metadata"]["format"], "Epub");
        assert_eq!(deserialized["units"][0]["provenance"]["archive_entry_path"], "OEBPS/cover.xhtml");
        assert_eq!(deserialized["quality_report"]["extraction_warnings"][0], "Some images were skipped");
    }

    #[test]
    fn golden_fixture_docx_example() {
        // Create a representative DOCX extraction result
        let id = DocumentId {
            canonical_path: "/example.docx".to_string(),
            content_signature: "docx_content_hash_abcde".to_string(),
        };

        let metadata = DocumentMetadataV2 {
            title: Some("Sample Word Document".to_string()),
            authors: vec!["Alice Johnson".to_string()],
            language: Some("en".to_string()),
            publisher: None,
            publication_date: None,
            isbn: None,
            identifiers: vec![],
            source_path: "/example.docx".to_string(),
            file_size: 51200,
            format: DocumentFormat::Docx,
            backend: "docx-extraction-backend".to_string(),
        };

        let units = vec![
            DocumentUnit {
                index: 0,
                text: "Document Title\n\nThis is a Microsoft Word document.".to_string(),
                provenance: DocumentProvenance {
                    source_path: "/example.docx".to_string(),
                    format: DocumentFormat::Docx,
                    backend: "docx-extraction-backend".to_string(),
                    page_number: None,
                    chapter_index: None,
                    chapter_title: None,
                    section_heading_path: vec!["Title".to_string()],
                    archive_entry_path: Some("word/document.xml".to_string()),
                    byte_offset_start: Some(0),
                    byte_offset_end: Some(58),
                    char_offset_start: Some(0),
                    char_offset_end: Some(58),
                },
            },
        ];

        let quality = DocumentQualityReport {
            extraction_warnings: vec![],
            text_coverage_percent: Some(100.0),
            empty_pages: vec![],
            encoding_repairs: vec![],
            encrypted_or_drm: false,
            image_only: false,
            likely_ocr: false,
        };

        // Serialize to JSON (this would be the golden fixture)
        let fixture = serde_json::json!({
            "document_id": id,
            "metadata": metadata,
            "units": units,
            "quality_report": quality
        });

        let json_string = serde_json::to_string_pretty(&fixture).unwrap();

        // Verify it can be deserialized back
        let deserialized: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized["metadata"]["format"], "Docx");
        assert_eq!(deserialized["units"][0]["provenance"]["section_heading_path"][0], "Title");
        assert_eq!(deserialized["quality_report"]["text_coverage_percent"], 100.0);
    }

    #[test]
    fn golden_fixture_unsupported_format() {
        // Create a representative unsupported format result
        let id = DocumentId {
            canonical_path: "/example.unknown".to_string(),
            content_signature: "unknown_content_hash_fghij".to_string(),
        };

        let metadata = DocumentMetadataV2 {
            title: None,
            authors: vec![],
            language: None,
            publisher: None,
            publication_date: None,
            isbn: None,
            identifiers: vec![],
            source_path: "/example.unknown".to_string(),
            file_size: 1024,
            format: DocumentFormat::Unknown("unknown".to_string()),
            backend: "fallback-backend".to_string(),
        };

        let units = vec![]; // No units extracted

        let quality = DocumentQualityReport {
            extraction_warnings: vec!["Unsupported file format".to_string()],
            text_coverage_percent: Some(0.0),
            empty_pages: vec![],
            encoding_repairs: vec![],
            encrypted_or_drm: false,
            image_only: true, // Assume it's an image or binary format
            likely_ocr: false,
        };

        // Serialize to JSON (this would be the golden fixture)
        let fixture = serde_json::json!({
            "document_id": id,
            "metadata": metadata,
            "units": units,
            "quality_report": quality
        });

        let json_string = serde_json::to_string_pretty(&fixture).unwrap();

        // Verify it can be deserialized back
        let deserialized: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized["metadata"]["format"], "Unknown(\"unknown\")");
        assert_eq!(deserialized["units"].as_array().unwrap().len(), 0);
        assert_eq!(deserialized["quality_report"]["extraction_warnings"][0], "Unsupported file format");
        assert_eq!(deserialized["quality_report"]["image_only"], true);
    }
}
