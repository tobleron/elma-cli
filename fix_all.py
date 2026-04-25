import re

with open("src/document_adapter.rs", "r") as f:
    content = f.read()

methods = [
    "extract_plaintext", "extract_html", "extract_pdf", "extract_epub",
    "extract_djvu", "extract_mobi", "extract_fb2", "extract_docx", "extract_rtf"
]

for method in methods:
    # Match the definition with the argument
    pattern = re.compile(rf"fn {method}\(path: &Path\) -> DocumentExtractionResult {{")
    new_def = f"fn {method}(path: &Path, _metadata: DocumentMetadata, _start_time: Instant) -> DocumentExtractionResult {{"
    content = pattern.sub(new_def, content)

with open("src/document_adapter.rs", "w") as f:
    f.write(content)
