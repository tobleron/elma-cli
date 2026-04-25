with open("src/document_adapter.rs", "r") as f:
    content = f.read()

new_struct = """pub(crate) struct DocumentExtractionResult {
    pub source_path: String,
    pub backend: String,
    pub total_chunks: usize,
    pub chunks: Vec<DocumentChunk>,
    pub metadata: DocumentMetadata,
    pub ok: bool,
    pub error: Option<String>,
    pub quality_score: f64,
    pub extraction_time_ms: u64,
    pub units_processed: usize,
}"""

# Replace the old struct
import re
pattern = re.compile(r"pub\(crate\) struct DocumentExtractionResult \{.*?\}", re.DOTALL)
new_content = pattern.sub(new_struct, content)

with open("src/document_adapter.rs", "w") as f:
    f.write(new_content)
