# 261 - Ebook Chat Answering With Citations And Extractive Fallback

Status: Pending
Priority: P1
Depends on: 258, 260

## Goal

Make ebook chat grounded, source-cited, and honest when the extracted context is insufficient.

## Answering Contract

For document-grounded questions, Elma must:

- answer only from extracted document context unless the user asks for outside knowledge,
- cite page/chapter/chunk provenance,
- say when the document context is insufficient,
- distinguish extraction limitations from absence of evidence,
- preserve recent chat history only when it helps the document question.

## Prompt/Workflow Requirements

- Build a compact evidence packet from retrieved chunks.
- Group evidence by document and source location.
- Include source labels that the model can cite.
- Reserve answer budget explicitly.
- Ask the model for citation-bearing answers when evidence supports them.
- If the model fails or refuses due to weak context, return an extractive fallback:
  - top relevant passages,
  - citations,
  - concise statement that a synthesized answer was not produced.

## Citation Format

Use the richest available provenance:

- PDF/DjVu: `source.pdf p. 12`
- EPUB/MOBI/AZW: `Book Title, Chapter 3`
- DOCX/HTML/FB2: `Heading > Subheading`
- Archive/package: `archive.cbz: ComicInfo.xml` or `package.iba: chapter.xhtml`
- Unknown: `chunk 42`

## Acceptance Criteria

- Answers to specific ebook questions cite sources.
- Broad summary requests cite representative sections or explain staged synthesis coverage.
- If retrieval finds weak/no evidence, Elma does not hallucinate.
- If extraction was degraded, the final answer mentions the limitation when material.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test document_answering`
- Real CLI Q&A tests over PDF, EPUB, and one degraded/unsupported format.

