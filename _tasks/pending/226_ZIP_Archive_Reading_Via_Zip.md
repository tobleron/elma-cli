# 226: ZIP Archive Reading via `zip`

## Status
`pending`

## Crrate
`zip` — Read/write ZIP archives.

## Rationale
DOCX (`.docx`), OOXML, EPUB, and many archive formats are ZIP-based. Elma already extracts EPUB via `epub` but `zip` gives direct low-level access to ZIP members without going through higher-level crates. Useful for: reading skill archives (bundled skill packs), extracting DOCX content, reading EPUB via `zip` instead of the `epub` crate where the latter struggles.

## Implementation Boundary
- Add `zip = "2.0"` to `Cargo.toml`.
- Create `src/archive.rs` with ZIP reader:

  ```rust
  use std::io::{Read, Seek};
  use zip::{ZipArchive, ZipReader};

  pub fn read_zip_entry<R: Read + Seek>(
      archive: &mut ZipArchive<R>,
      name: &str,
  ) -> anyhow::Result<Vec<u8>> {
      let mut file = archive.by_name(name)?;
      let mut data = Vec::new();
      file.read_to_end(&mut data)?;
      Ok(data)
  }

  pub fn list_zip_members<R: Read + Seek>(
      archive: &mut ZipArchive<R>,
  ) -> Vec<String> {
      archive.file_names().map(String::from).collect()
  }
  ```

- Integrate where Elma reads EPUB/DOCX (consider replacing `epub` crate usage with `zip` for simpler cases).
- Use for skill bundle/`.elmaskill` archive reading.
- Do NOT replace PDF extraction or DjVu/Mobi processing — those are format-specific.

## Verification
- `cargo build` passes.
- A ZIP file's member list is correctly enumerated.
- Member content is correctly extracted as bytes.
- Existing epub/DOCX processing behavior unchanged.