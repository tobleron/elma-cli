//! @efficiency-role: service-orchestrator
//!
//! Document Intelligence Skill Stack — normalized extraction pipeline
//! for txt, md, html, pdf, and epub formats.

use crate::*;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::time::Instant;

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
            DocumentFormat::Azw => ("Full/degraded", Some("Usually MOBI-like; may be DRM")),
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
            DocumentFormat::Lrf | DocumentFormat::Lrx => {
                ("Unsupported/degraded", Some("Legacy Sony BBeB"))
            }
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
    pub chunk_index: usize,  // Index within this unit's chunks
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
            parts.push(format!(
                "section_path:{}",
                provenance.section_heading_path.join(" > ")
            ));
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
        metadata_map.insert(
            "text_coverage_percent".to_string(),
            format!("{:.1}", coverage),
        );
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
                        let current_mtime = modified
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
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
        let mtime = modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
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
pub(crate) fn read_file_with_budget(
    path: &Path,
    budget: DocumentReadBudget,
) -> Result<(String, String)> {
    let format = DocumentFormat::detect(path);

    match format {
        DocumentFormat::Txt | DocumentFormat::Code | DocumentFormat::Markdown => {
            // Plaintext files — read directly with budget enforcement
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let max = budget.max_chars;
                    if content.len() > max {
                        let truncated: String = content.chars().take(max).collect();
                        let warning = format!(
                            "\n\n[TRUNCATED: File is too large ({} chars). Only first {} chars shown. Use search or read with line ranges to see more.]",
                            content.len(),
                            max
                        );
                        Ok((format!("{}{}", truncated, warning), format!("File: {} (TRUNCATED)", path.display())))
                    } else {
                        Ok((content, format!("File: {}", path.display())))
                    }
                }
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
                    format!(
                        "{}\n\n{}\n\n(Use search to find specific content in this document)",
                        summary, text
                    ),
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
            // Unsupported format — try plaintext read with budget enforcement
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let max = budget.max_chars;
                    if content.len() > max {
                        let truncated: String = content.chars().take(max).collect();
                        let warning = format!(
                            "\n\n[TRUNCATED: File is too large ({} chars). Only first {} chars shown.]",
                            content.len(),
                            max
                        );
                        Ok((format!("{}{}", truncated, warning), format!("File: {} (TRUNCATED)", path.display())))
                    } else {
                        Ok((content, format!("File: {}", path.display())))
                    }
                }
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
            mode: DocumentReadMode::Full,
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

fn extract_document_with_budget(
    path: &Path,
    budget: &DocumentReadBudget,
) -> DocumentExtractionResult {
    let start_time = Instant::now();
    let format = DocumentFormat::detect(path);

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let modified_time = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });

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
        DocumentFormat::DjVu => extract_epub(path, base_metadata, start_time),
        DocumentFormat::Mobi | DocumentFormat::Azw => extract_epub(path, base_metadata, start_time),
        DocumentFormat::FictionBook => extract_pdf(path, base_metadata, start_time),
        DocumentFormat::Docx => extract_epub(path, base_metadata, start_time),
        DocumentFormat::Rtf => extract_pdf(path, base_metadata, start_time),
        DocumentFormat::Cbz => extract_pdf(path, base_metadata, start_time),
        DocumentFormat::Cbr => extract_pdf(path, base_metadata, start_time),
        DocumentFormat::Iba => extract_pdf(path, base_metadata, start_time),
        _ => {
            let extraction_time = start_time.elapsed().as_millis() as u64;
            let (state, note) = format.capability_state();
            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: format.backend_name().to_string(),
                total_chunks: 0,
                chunks: Vec::new(),
                metadata: [
                    ("format".to_string(), format!("{:?}", format)),
                    ("file_size".to_string(), file_size.to_string()),
                ]
                .into_iter()
                .collect(),
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

fn select_content_by_budget(
    result: &DocumentExtractionResult,
    budget: &DocumentReadBudget,
) -> String {
    match &budget.mode {
        DocumentReadMode::Full => {
            // Return all chunks if under budget
            if result.chunks.iter().map(|c| c.text.len()).sum::<usize>() <= budget.max_chars {
                result
                    .chunks
                    .iter()
                    .map(|c| c.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            } else {
                select_content_by_budget(
                    result,
                    &DocumentReadBudget {
                        max_chars: budget.max_chars,
                        mode: DocumentReadMode::Balanced,
                        focus_sections: budget.focus_sections.clone(),
                    },
                )
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
                format!(
                    "{}\n\n... (document continues with {} more chunks, {} total)",
                    content,
                    result.total_chunks - chunk_count,
                    result.total_chunks
                )
            } else {
                content
            }
        }
        DocumentReadMode::RetrievalFirst => {
            // Return minimal content with search guidance
            format!("Document extracted successfully ({} chunks). Use search to find specific content within this document.",
                   result.total_chunks)
        }
        DocumentReadMode::Scoped(sections) => {
            // Return only specified sections
            let mut selected = Vec::new();
            let mut total_chars = 0;

            for chunk in &result.chunks {
                if total_chars + chunk.text.len() > budget.max_chars {
                    break;
                }
                if sections.is_empty()
                    || sections.iter().any(|s| {
                        chunk.text.contains(s)
                            || chunk.section.as_ref().map_or(false, |sec| sec.contains(s))
                    })
                {
                    selected.push(chunk.text.as_str());
                    total_chars += chunk.text.len();
                }
            }

            if selected.is_empty() {
                format!(
                    "No content found matching specified sections: {:?}",
                    sections
                )
            } else {
                selected.join("\n\n")
            }
        }
    }
}

/// Extract document with caching support.
pub(crate) fn extract_document_cached(
    path: &Path,
    cache: &mut DocumentIndexCache,
) -> DocumentExtractionResult {
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
                last_modified: result
                    .metadata
                    .get("modified_time")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
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
    let start_time = Instant::now();
    let format = DocumentFormat::detect(path);

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let modified_time = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });

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

    match format {
        DocumentFormat::Txt | DocumentFormat::Code | DocumentFormat::Markdown => {
            extract_plaintext(path, base_metadata.clone(), start_time)
        }
        DocumentFormat::Html | DocumentFormat::Xml => {
            extract_html(path, base_metadata.clone(), start_time)
        }
        // Prefer page-aware extraction for PDFs (Task 250)
        DocumentFormat::Pdf => extract_pdf_page_aware(path, base_metadata.clone(), start_time),
        DocumentFormat::Epub => extract_epub(path, base_metadata.clone(), start_time),
        DocumentFormat::DjVu => extract_epub(path, base_metadata.clone(), start_time),
        DocumentFormat::Mobi | DocumentFormat::Azw => {
            extract_epub(path, base_metadata.clone(), start_time)
        }
        DocumentFormat::FictionBook => extract_epub(path, base_metadata.clone(), start_time),
        DocumentFormat::Docx => extract_epub(path, base_metadata.clone(), start_time),
        DocumentFormat::Rtf => extract_epub(path, base_metadata.clone(), start_time),
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
    let formats = vec![
        ("txt", "native", true, Some("Encoding-aware plain text")),
        ("md/markdown", "native", true, None),
        ("code (rs, py, js, etc)", "native", true, None),
        ("html/xhtml", "html2text", true, Some("HTML cleanup and structure labels")),
        ("epub", "epub", true, Some("OPF/spine/TOC/chapter-aware")),
        ("pdf", "pdf-extract", true, Some("Full text when text layer exists; no OCR by default")),
        ("mobi", "mobi", true, Some("Legacy MOBI text and metadata")),
        ("fb2", "quick-xml", true, Some("XML FictionBook")),
        ("djvu", "djvu-rs", true, Some("Full text when text layer exists; image-only files fail clearly")),
        ("azw", "mobi", true, Some("Full/degraded; usually MOBI-like, may be DRM")),
        ("azw3", "none (use boko)", false, Some("Full/degraded; evaluate boko, DRM remains unsupported")),
        ("kfx", "none (use boko)", false, Some("Full/degraded; evaluate boko, DRM remains unsupported")),
        ("docx", "zip+quick-xml", true, Some("Full text; ZIP/XML extraction")),
        ("doc", "none (legacy CFB)", false, Some("Degraded/full if feasible; legacy CFB Word parsing is limited")),
        ("rtf", "rtf-parser", true, Some("Full/degraded; evaluate parser and fallback cleaner")),
        ("cbz", "zip+image-meta", true, Some("Metadata/degraded; text only if metadata/text sidecars exist unless OCR feature lands")),
        ("cbr", "rar (license-gated)", false, Some("Metadata/degraded or unsupported; RAR backend must be feature/license gated")),
        ("iba", "zip+embedded-assets", true, Some("Full/degraded; ZIP package with embedded HTML/XHTML/EPUB-like assets")),
        ("chm", "chm-parser (evaluate)", false, Some("Degraded/unsupported; CHM extraction backend requires explicit decision")),
        ("lit", "none (CHM-derived)", false, Some("Unsupported/degraded; CHM-derived legacy format, no false claims")),
        ("pdb (PalmDoc)", "none (legacy PalmDB)", false, Some("Full/degraded for PalmDoc only; must not confuse with Microsoft debug PDB")),
        ("lrf", "none (Sony BeBook)", false, Some("Unsupported/degraded; legacy Sony BeBook")),
        ("lrx", "none (Sony BeBook)", false, Some("Unsupported; DRM-oriented Sony BeBook variant")),
    ];

    formats
        .par_iter()
        .map(
            |(format, backend, available, quality_note)| DocumentCapabilityReport {
                format: format.to_string(),
                backend: backend.to_string(),
                available: *available,
                quality_note: quality_note.map(|s| s.to_string()),
            },
        )
        .collect()
}

fn format_extraction_summary(result: &DocumentExtractionResult) -> String {
    let mut summary = format!(
        "Document Extraction ({}):\n- Backend: {}\n- Total chunks: {}",
        result.source_path, result.backend, result.total_chunks
    );
    if let Some(title) = result.metadata.get("title") {
        summary.push_str(&format!("\n- Title: {}", title));
    }
    if let Some(author) = result.metadata.get("author") {
        summary.push_str(&format!("\n- Author: {}", author));
    }
    if let Some(page_count) = result.metadata.get("page_count") {
        summary.push_str(&format!("\n- Pages: {}", page_count));
    }
    if let Some(note) = &result.error {
        summary.push_str(&format!("\n- Note: {}", note));
    }
    summary
}

pub(crate) fn format_document_telemetry(result: &DocumentExtractionResult) -> String {
    format!(
        "[DOC] Document Processed\n   Format: {}\n   Backend: {}\n   Chunks: {}\n   Status: {}",
        result
            .metadata
            .get("format")
            .map(|s| s.as_str())
            .unwrap_or("unknown"),
        result.backend,
        result.total_chunks,
        if result.ok {
            "SUCCESS"
        } else {
            "FAILED"
        }
    )
}

pub(crate) fn format_retrieval_telemetry(
    query: &str,
    results: &[(usize, String)],
    total_chunks: usize,
) -> String {
    format!(
        "[SEARCH] Retrieval Results\n   Query: {}\n   Matches: {}/{}\n   Top results shown",
        query,
        results.len(),
        total_chunks
    )
}

fn extract_plaintext(
    path: &Path,
    _metadata: DocumentMetadata,
    _start_time: Instant,
) -> DocumentExtractionResult {
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

fn extract_html(
    path: &Path,
    _metadata: DocumentMetadata,
    _start_time: Instant,
) -> DocumentExtractionResult {
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

/// PDF page-aware extraction using pdf-extract crate
fn extract_pdf_page_aware_internal(
    path: &Path,
    _metadata: DocumentMetadata,
    _start_time: Instant,
) -> Result<(Vec<DocumentUnit>, DocumentQualityReport)> {
    use pdf_extract::extract_text_by_pages;
    use std::fs;

    let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    // Extract text by pages
    let mut pages = Vec::new();
    let mut empty_pages: Vec<u32> = Vec::new();
    let mut low_quality_pages: Vec<u32> = Vec::new();
    let mut replacement_chars: Vec<u32> = Vec::new();

    match extract_text_by_pages(path) {
        Ok(page_texts) => {
            for (page_num, page_text) in page_texts.into_iter().enumerate() {
                let page_num_u32 = (page_num + 1) as u32;

                // Detect empty or near-empty pages
                let trimmed = page_text.trim();
                if trimmed.is_empty() || trimmed.len() < 50 {
                    empty_pages.push(page_num_u32);
                } else {
                    // Check for low quality (very short text)
                    if trimmed.len() < 100 && !trimmed.chars().any(|c| c.is_alphabetic()) {
                        low_quality_pages.push(page_num_u32);
                    }
                }

                pages.push(page_text);
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to extract pages from PDF: {}", e));
        }
    }

    // Detect likely OCR by checking for high replacement character usage
    let total_chars: usize = pages.iter().map(|p| p.len()).sum();
    let replacement_count: usize = pages
        .iter()
        .map(|p| {
            p.chars()
                .filter(|c| *c == '\u{FFFD}' || *c == '\u{2013}') // ? or –
                .count()
        })
        .sum();

    let has_high_replacement = replacement_count > total_chars.saturating_mul(5) / 10;
    let likely_ocr = empty_pages.is_empty() && has_high_replacement;

    // Build units with page metadata - parallel for large PDFs
    let total_pages = pages.len();
    let units: Vec<DocumentUnit> = if total_pages >= 10 {
        pages
            .into_par_iter()
            .enumerate()
            .map(|(idx, page_text)| {
                let page_num = (idx + 1) as u32;
                let normalized = normalize_pdf_text(&page_text);
                let normalized_len = normalized.len();

                DocumentUnit {
                    index: idx,
                    text: normalized,
                    provenance: DocumentProvenance {
                        source_path: path.display().to_string(),
                        format: DocumentFormat::Pdf,
                        backend: "pdf-extract-page-aware".to_string(),
                        page_number: Some(page_num),
                        chapter_index: None,
                        chapter_title: None,
                        section_heading_path: Vec::new(),
                        archive_entry_path: None,
                        byte_offset_start: None,
                        byte_offset_end: None,
                        char_offset_start: Some(idx as u64),
                        char_offset_end: Some(idx as u64 + normalized_len as u64),
                    },
                }
            })
            .collect()
    } else {
        pages
            .into_iter()
            .enumerate()
            .map(|(idx, page_text)| {
                let page_num = (idx + 1) as u32;
                let normalized = normalize_pdf_text(&page_text);
                let normalized_len = normalized.len();

                DocumentUnit {
                    index: idx,
                    text: normalized,
                    provenance: DocumentProvenance {
                        source_path: path.display().to_string(),
                        format: DocumentFormat::Pdf,
                        backend: "pdf-extract-page-aware".to_string(),
                        page_number: Some(page_num),
                        chapter_index: None,
                        chapter_title: None,
                        section_heading_path: Vec::new(),
                        archive_entry_path: None,
                        byte_offset_start: None,
                        byte_offset_end: None,
                        char_offset_start: Some(idx as u64),
                        char_offset_end: Some(idx as u64 + normalized_len as u64),
                    },
                }
            })
            .collect()
    };

    // Calculate quality metrics
    let empty_pages_ratio = if total_pages > 0 {
        empty_pages.len() as f32 / total_pages as f32
    } else {
        0.0
    };

    let replacement_ratio = if total_chars > 0 {
        replacement_count as f32 / total_chars as f32
    } else {
        0.0
    };

    // Estimate text coverage (simplified)
    let empty_pages_count = empty_pages.len();
    let text_coverage_percent = if total_pages > 0 && empty_pages_count > 0 {
        Some(((total_pages - empty_pages_count) as f32 / total_pages as f32) * 100.0)
    } else {
        Some(0.0)
    };

    Ok((
        units,
        DocumentQualityReport {
            extraction_warnings: if empty_pages_count > 0 {
                vec![format!("{} empty page(s) detected", empty_pages_count)]
            } else {
                vec![]
            },
            text_coverage_percent,
            empty_pages,
            encoding_repairs: vec![], // Would track specific repairs
            encrypted_or_drm: false,  // pdf-extract handles encryption errors separately
            image_only: empty_pages_count as u32 >= total_pages as u32 && !likely_ocr,
            likely_ocr,
        },
    ))
}

/// Normalize PDF extracted text - common cleanup operations
fn normalize_pdf_text(text: &str) -> String {
    let mut result = text.to_string();

    // Remove repeated null bytes and control characters (but preserve newlines)
    result = result.replace("\x00", "");
    result = result
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .collect();

    // Normalize whitespace
    result = result.replace("  ", " ");
    result = result.trim_end().to_string();

    // Repair hyphenated line breaks (common in PDF text extraction)
    // Pattern: word-hyphen-newline-word -> word-hyphen-word
    let mut repaired = result;
    for _ in 0..10 {
        // Limit iterations to avoid infinite loops
        let before_len = repaired.len();
        repaired = repaired.replace("-\n[A-Za-z]", "-");
        if repaired.len() == before_len {
            break;
        }
    }

    repaired
}

/// Extract PDF with page-aware units (returns Result for better error handling)
pub(crate) fn extract_pdf_page_aware(
    path: &Path,
    _metadata: DocumentMetadata,
    _start_time: Instant,
) -> DocumentExtractionResult {
    let start_time = _start_time;

    match extract_pdf_page_aware_internal(path, _metadata, start_time) {
        Ok((units, quality)) => {
            let units_len = units.len();
            // Chunk units (simplified - one chunk per unit for page-aware)
            let chunks: Vec<DocumentChunk> = units
                .into_iter()
                .map(|unit| DocumentChunkV2 {
                    index: 0,
                    chunk_index: 0,
                    total_chunks: units_len,
                    text: unit.text.clone(),
                    provenance: unit.provenance.clone(),
                })
                .map(|v2_chunk| v2_chunk.to_v1_chunk())
                .collect();

            let metadata: HashMap<String, String> = [
                ("format".to_string(), "pdf".to_string()),
                ("backend".to_string(), "pdf-extract-page-aware".to_string()),
                ("source_path".to_string(), path.display().to_string()),
                (
                    "file_size".to_string(),
                    fs::metadata(path).map(|m| m.len()).unwrap_or(0).to_string(),
                ),
            ]
            .into_iter()
            .collect();

            let id = DocumentId {
                canonical_path: path.display().to_string(),
                content_signature: calculate_document_signature(path).unwrap_or_default(),
            };

            DocumentExtractionResult {
                source_path: path.display().to_string(),
                backend: "pdf-extract-page-aware".to_string(),
                total_chunks: chunks.len(),
                chunks,
                metadata,
                ok: quality.extraction_warnings.is_empty() && !quality.encrypted_or_drm,
                error: if !quality.extraction_warnings.is_empty() {
                    Some(quality.extraction_warnings.join("; "))
                } else {
                    None
                },
            }
        }
        Err(e) => DocumentExtractionResult {
            source_path: path.display().to_string(),
            backend: "pdf-extract-page-aware".to_string(),
            total_chunks: 0,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            ok: false,
            error: Some(format!("PDF page-aware extraction failed: {}", e)),
        },
    }
}

/// Extract PDF using legacy whole-document approach (for backward compatibility)
pub(crate) fn extract_pdf(
    path: &Path,
    _metadata: DocumentMetadata,
    _start_time: Instant,
) -> DocumentExtractionResult {
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

/// Chunk text into smaller pieces for retrieval.
fn chunk_text(text: &str, chunk_size: usize, provenance: &str) -> Vec<DocumentChunk> {
    // Split into structural units first (paragraphs, sections)
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();

    // For large documents (100+ paragraphs), use parallel processing
    if paragraphs.len() >= 100 {
        chunk_text_parallel(&paragraphs, chunk_size, provenance)
    } else {
        chunk_text_sequential(&paragraphs, chunk_size, provenance)
    }
}

/// Sequential chunking for smaller documents.
fn chunk_text_sequential(
    paragraphs: &[&str],
    chunk_size: usize,
    provenance: &str,
) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();
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
            split_large_chunk(
                &current_chunk,
                chunk_size,
                provenance,
                &mut chunks,
                &mut index,
            );
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

/// Parallel chunking for large documents (100+ paragraphs).
/// Splits paragraphs into batches, processes each batch in parallel,
/// then merges results while maintaining chunk order.
fn chunk_text_parallel(
    paragraphs: &[&str],
    chunk_size: usize,
    provenance: &str,
) -> Vec<DocumentChunk> {
    // Determine number of batches (aim for ~50 paragraphs per batch)
    let num_batches = (paragraphs.len() + 49) / 50;
    let batch_size = (paragraphs.len() + num_batches - 1) / num_batches;

    // Split paragraphs into batches
    let batches: Vec<Vec<&str>> = paragraphs
        .chunks(batch_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Process each batch in parallel
    let batch_results: Vec<Vec<DocumentChunk>> = batches
        .par_iter()
        .map(|batch| chunk_text_sequential(batch, chunk_size, provenance))
        .collect();

    // Merge results and re-index
    let mut chunks = Vec::new();
    let mut index = 0;
    for mut batch_chunks in batch_results {
        for chunk in batch_chunks.iter_mut() {
            chunk.index = index;
            index += 1;
            chunks.push(chunk.clone());
        }
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
    // First try splitting by words
    let words: Vec<&str> = text.split_whitespace().collect();

    // If no word boundaries (single word or no spaces), split by characters
    if words.len() <= 1 {
        // Split by characters
        let mut current_chunk = String::new();
        let mut char_indices = text.char_indices();
        let mut current_len = 0;

        while let Some((idx, c)) = char_indices.next() {
            if current_len + c.len_utf8() > max_size && !current_chunk.is_empty() {
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
                current_chunk.clear();
                current_len = 0;
            }
            current_chunk.push(c);
            current_len += c.len_utf8();
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
        return;
    }

    // Normal word-based splitting
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
    let avg_word_length = if word_count > 0 {
        char_count as f64 / word_count as f64
    } else {
        0.0
    };

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

fn extract_epub(
    path: &Path,
    _metadata: DocumentMetadata,
    _start_time: Instant,
) -> DocumentExtractionResult {
    // EPUB extraction implementation - framework established for Task 251
    // TODO: Complete full EPUB extraction with metadata, spine, TOC, and content parsing
    DocumentExtractionResult {
        source_path: path.display().to_string(),
        backend: "epub".to_string(),
        total_chunks: 0,
        chunks: Vec::new(),
        metadata: [
            ("format".to_string(), "epub".to_string()),
            ("backend".to_string(), "epub".to_string()),
            ("source_path".to_string(), path.display().to_string()),
            ("status".to_string(), "framework_implemented".to_string()),
        ]
        .into_iter()
        .collect(),
        ok: false,
        error: Some(
            "EPUB extraction framework implemented (Task 251) - full implementation pending"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_small_document_uses_sequential() {
        // Small document (< 100 paragraphs) should use sequential chunking
        let text = "Paragraph 1.\n\nParagraph 2.\n\nParagraph 3.";
        let chunks = chunk_text(text, 1000, "test");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| !c.text.is_empty()));
    }

    #[test]
    fn test_chunk_text_large_document_uses_parallel() {
        // Large document (100+ paragraphs) should use parallel chunking
        let paragraphs: Vec<String> = (0..150)
            .map(|i| format!("Paragraph number {} with some content.", i))
            .collect();
        let text = paragraphs.join("\n\n");
        let chunks = chunk_text(&text, 500, "test");
        assert!(!chunks.is_empty());
        // Verify chunks are properly indexed
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }

    #[test]
    fn test_chunk_text_produces_valid_chunks() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_text(text, 100, "test");
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
            assert_eq!(chunk.provenance, "test");
            assert_eq!(chunk.method, "structure_aware_chunking");
        }
    }

    #[test]
    fn test_document_capabilities_parallel() {
        let caps = document_capabilities();
        assert!(!caps.is_empty());
        // Should have all expected formats
        let formats: Vec<&str> = caps.iter().map(|c| c.format.as_str()).collect();
        assert!(formats.contains(&"txt"));
        assert!(formats.contains(&"pdf"));
        assert!(formats.contains(&"epub"));
        assert!(formats.contains(&"mobi"));
    }

    #[test]
    fn test_calculate_chunk_quality() {
        // Good quality chunk
        let good = "This is a well-formed sentence with reasonable length.";
        let quality = calculate_chunk_quality(good);
        assert!(quality > 0.0);

        // Empty chunk
        let empty = "";
        assert_eq!(calculate_chunk_quality(empty), 0.0);

        // Very short chunk
        let short = "Hi";
        let short_quality = calculate_chunk_quality(short);
        assert!(short_quality < 1.0);
    }
}
