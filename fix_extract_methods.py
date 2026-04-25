with open("src/document_adapter.rs", "r") as f:
    content = f.read()

# Replace:
# extract_plaintext(path, base_metadata, start_time) -> extract_plaintext(path)
# for all extraction methods

import re

# We need to find all calling sites of extract_plaintext etc and fix them,
# or better yet, fix the definitions to accept arguments if they should,
# but the errors say they take 1 argument.

# Let's fix the calls in the match statement in extract_document_with_budget (lines 700-720)
# And the definitions themselves.

# Actually, the easiest way is to adjust all definitions of extract_*
# to take (path: &Path, metadata: DocumentMetadata, start_time: Instant)
# instead of just (path: &Path).

# Let's fix all definitions to take all 3 args.

new_definitions = [
    ("fn extract_plaintext", "fn extract_plaintext(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_html", "fn extract_html(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_pdf", "fn extract_pdf(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_epub", "fn extract_epub(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_djvu", "fn extract_djvu(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_mobi", "fn extract_mobi(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_fb2", "fn extract_fb2(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_docx", "fn extract_docx(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
    ("fn extract_rtf", "fn extract_rtf(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {"),
]

for old_def, new_def in new_definitions:
    content = content.replace(old_def + "(path: &Path) -> DocumentExtractionResult {", new_def)

# Add Instant import
if "use std::time::Instant;" not in content:
    content = "use std::time::Instant;\n" + content

with open("src/document_adapter.rs", "w") as f:
    f.write(content)
