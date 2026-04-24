# 233: Fast XML Parsing via `quick-xml`

## Status
`pending`

## Crrate
`quick-xml` — Fast XML parser and writer.

## Rationale
DOCX, OOXML, and some legacy config formats are XML-based. Elma already uses `html2text` for HTML→text conversion, but DOCX files (`.docx`) are ZIP archives containing XML. `quick-xml` gives fast, streaming XML parsing without DOM overhead — useful for extracting content from DOCX without pulling in heavy dependencies.

## Implementation Boundary
- Add `quick-xml = "0.36"` to `Cargo.toml`.
- Extend `src/archive.rs` (from Task 226) with XML helpers:

  ```rust
  use quick_xml::Reader;

  pub fn extract_docx_text(zip_data: &[u8]) -> anyhow::Result<String> {
      let archive = zip::ZipArchive::new(Cursor::new(zip_data))?;
      let doc_xml = read_zip_entry_mut(&mut archive.by_name("word/document.xml")?)?;
      let mut reader = Reader::from_bytes(&doc_xml);
      reader.config_mut().trim_text(true);
      // ... parse and extract text
  }
  ```

- Implement basic DOCX text extraction using `zip` (Task 226) + `quick-xml`.
- Keep existing PDF/EPUB/Markdown processing as primary document types.
- Do NOT implement full DOCX rendering — just text extraction for skill doc ingestion.
- Do NOT add `roxmltree`/`xmltree` — `quick-xml` covers read-only needs.

## Verification
- `cargo build` passes.
- DOCX text extraction returns readable text content.
- Existing document processing unchanged.