use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "patch",
            "Apply multi-file changes atomically. Supports adding, updating, and deleting files in a single call. Use this for refactoring, renaming, or any change spanning multiple files. Format: *** Begin Patch *** / *** Add File: {path} *** / *** Delete File: {path} *** / *** Update File: {path} *** / <<<<<<< ORIGINAL / ======= / >>>>>>> UPDATED / *** End Patch ***.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "patch": {"type": "string", "description": "The patch content in the custom multi-file format"}
                },
                "required": ["patch"]
            }),
            vec![
                "apply patch to multiple files",
                "multi-file edit",
                "refactor across files",
                "rename across files",
                "batch file changes",
                "atomic multi-file update",
            ],
        )
        .not_deferred(),
    );
}

/// Parsed patch operation
#[derive(Debug, Clone)]
pub enum PatchOperation {
    AddFile {
        path: String,
        content: String,
    },
    DeleteFile {
        path: String,
    },
    UpdateFile {
        path: String,
        old_string: String,
        new_string: String,
    },
}

/// Full parsed patch
#[derive(Debug)]
pub struct ParsedPatch {
    pub operations: Vec<PatchOperation>,
}

/// Error during patch parsing or validation
#[derive(Debug)]
pub enum PatchParseError {
    EmptyPatch,
    MissingBeginMarker,
    MissingEndMarker,
    InvalidSectionHeader(String),
    MissingOriginalDelimiter(String),
    MissingSeparatorDelimiter(String),
    MissingUpdatedDelimiter(String),
    EmptyOperation,
    DuplicatePath(String),
}

impl std::fmt::Display for PatchParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPatch => write!(f, "Patch is empty"),
            Self::MissingBeginMarker => write!(f, "Invalid format: missing *** Begin Patch ***"),
            Self::MissingEndMarker => write!(f, "Invalid format: missing *** End Patch ***"),
            Self::InvalidSectionHeader(h) => write!(f, "Invalid section header: {}", h),
            Self::MissingOriginalDelimiter(p) => write!(f, "{}: missing <<<<<<< ORIGINAL", p),
            Self::MissingSeparatorDelimiter(p) => write!(f, "{}: missing =======", p),
            Self::MissingUpdatedDelimiter(p) => write!(f, "{}: missing >>>>>>> UPDATED", p),
            Self::EmptyOperation => write!(f, "Patch contains no operations"),
            Self::DuplicatePath(p) => write!(f, "Duplicate path: {}", p),
        }
    }
}

/// Parse a patch string into structured operations.
///
/// Format (documentation only, not a runnable doctest):
///
/// ```text
/// *** Begin Patch ***
/// *** Add File: path ***
/// content
/// *** Delete File: path ***
/// *** Update File: path ***
/// <<<<<<< ORIGINAL
/// old
/// =======
/// new
/// >>>>>>> UPDATED
/// *** End Patch ***
/// ```
pub fn parse_patch(input: &str) -> Result<ParsedPatch, PatchParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(PatchParseError::EmptyPatch);
    }

    // Strip *** Begin Patch *** / *** End Patch ***
    let inner = input
        .strip_prefix("*** Begin Patch ***")
        .and_then(|s| s.strip_suffix("*** End Patch ***"))
        .map(|s| s.trim());
    let inner = match inner {
        Some(i) => i,
        None => {
            if !input.starts_with("*** Begin Patch ***") {
                return Err(PatchParseError::MissingBeginMarker);
            }
            return Err(PatchParseError::MissingEndMarker);
        }
    };

    if inner.is_empty() {
        return Err(PatchParseError::EmptyOperation);
    }

    let mut operations = Vec::new();
    // Split by section headers: lines starting with "*** "
    let mut sections: Vec<&str> = Vec::new();
    let mut current_start = 0;
    for (pos, _) in inner.match_indices("\n*** ") {
        // pos is the \n before ***
        sections.push(&inner[current_start..pos]);
        current_start = pos + 1; // skip the \n
    }
    sections.push(&inner[current_start..]);

    for section in sections {
        let section = section.trim();
        if section.is_empty() {
            continue;
        }

        // Find the first line which is the header
        let header_end = section.find('\n').unwrap_or(section.len());
        let header_raw = section[..header_end].trim();
        let body = if header_end < section.len() {
            section[header_end + 1..].trim().to_string()
        } else {
            String::new()
        };

        let header_prefix = header_raw
            .strip_prefix("*** ")
            .and_then(|s| s.strip_suffix(" ***"));
        let header = match header_prefix {
            Some(h) => h,
            None => {
                return Err(PatchParseError::InvalidSectionHeader(
                    header_raw.to_string(),
                ))
            }
        };

        if let Some(path) = header.strip_prefix("Add File: ") {
            let path = path.trim().to_string();
            operations.push(PatchOperation::AddFile {
                path,
                content: body,
            });
        } else if let Some(path) = header.strip_prefix("Delete File: ") {
            let path = path.trim().to_string();
            operations.push(PatchOperation::DeleteFile { path });
        } else if let Some(path) = header.strip_prefix("Update File: ") {
            let path = path.trim().to_string();
            // Parse the <<<<<<< ORIGINAL / ======= / >>>>>>> UPDATED markers
            let original_marker = "<<<<<<< ORIGINAL";
            let sep_marker = "=======";
            let updated_marker = ">>>>>>> UPDATED";

            let pos_orig = body
                .find(original_marker)
                .ok_or_else(|| PatchParseError::MissingOriginalDelimiter(path.clone()))?;
            let after_orig = &body[pos_orig + original_marker.len()..];
            let pos_sep = after_orig
                .find(&format!("\n{}\n", sep_marker))
                .map(|p| p + pos_orig + original_marker.len() + 1)
                .or_else(|| {
                    after_orig
                        .find(sep_marker)
                        .map(|p| p + pos_orig + original_marker.len())
                })
                .ok_or_else(|| PatchParseError::MissingSeparatorDelimiter(path.clone()))?;

            let old_string = after_orig[..pos_sep - pos_orig - original_marker.len()]
                .trim()
                .to_string();
            let after_sep = &body[pos_sep + sep_marker.len()..];
            let pos_upd = after_sep
                .find(updated_marker)
                .map(|p| p + pos_sep + sep_marker.len())
                .ok_or_else(|| PatchParseError::MissingUpdatedDelimiter(path.clone()))?;

            let new_string = after_sep[..pos_upd - pos_sep - sep_marker.len()]
                .trim()
                .to_string();

            operations.push(PatchOperation::UpdateFile {
                path,
                old_string,
                new_string,
            });
        } else {
            return Err(PatchParseError::InvalidSectionHeader(header.to_string()));
        }
    }

    if operations.is_empty() {
        return Err(PatchParseError::EmptyOperation);
    }

    // Check for duplicate paths
    let mut seen = std::collections::HashSet::new();
    for op in &operations {
        let path = match op {
            PatchOperation::AddFile { path, .. } => path,
            PatchOperation::DeleteFile { path } => path,
            PatchOperation::UpdateFile { path, .. } => path,
        };
        if !seen.insert(path) {
            return Err(PatchParseError::DuplicatePath(path.clone()));
        }
    }

    Ok(ParsedPatch { operations })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_add_file() {
        let input =
            "*** Begin Patch ***\n*** Add File: src/foo.rs ***\npub fn foo() {}\n*** End Patch ***";
        let patch = parse_patch(input).unwrap();
        assert_eq!(patch.operations.len(), 1);
        match &patch.operations[0] {
            PatchOperation::AddFile { path, content } => {
                assert_eq!(path, "src/foo.rs");
                assert_eq!(content, "pub fn foo() {}");
            }
            _ => panic!("expected AddFile"),
        }
    }

    #[test]
    fn test_parse_delete_file() {
        let input = "*** Begin Patch ***\n*** Delete File: src/bar.rs ***\n*** End Patch ***";
        let patch = parse_patch(input).unwrap();
        assert_eq!(patch.operations.len(), 1);
        match &patch.operations[0] {
            PatchOperation::DeleteFile { path } => {
                assert_eq!(path, "src/bar.rs");
            }
            _ => panic!("expected DeleteFile"),
        }
    }

    #[test]
    fn test_parse_update_file() {
        let input = "*** Begin Patch ***\n*** Update File: src/main.rs ***\n<<<<<<< ORIGINAL\nold\n=======\nnew\n>>>>>>> UPDATED\n*** End Patch ***";
        let patch = parse_patch(input).unwrap();
        assert_eq!(patch.operations.len(), 1);
        match &patch.operations[0] {
            PatchOperation::UpdateFile {
                path,
                old_string,
                new_string,
            } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(old_string, "old");
                assert_eq!(new_string, "new");
            }
            _ => panic!("expected UpdateFile"),
        }
    }

    #[test]
    fn test_parse_multiple_operations() {
        let input = concat!(
            "*** Begin Patch ***\n",
            "*** Add File: src/a.rs ***\nfn a() {}\n\n",
            "*** Delete File: src/b.rs ***\n\n",
            "*** Update File: src/c.rs ***\n<<<<<<< ORIGINAL\nold\n=======\nnew\n>>>>>>> UPDATED\n",
            "*** End Patch ***"
        );
        let patch = parse_patch(input).unwrap();
        assert_eq!(patch.operations.len(), 3);
    }

    #[test]
    fn test_parse_empty_patch() {
        assert!(matches!(parse_patch(""), Err(PatchParseError::EmptyPatch)));
    }

    #[test]
    fn test_parse_missing_begin() {
        let input = "*** End Patch ***";
        assert!(matches!(
            parse_patch(input),
            Err(PatchParseError::MissingBeginMarker)
        ));
    }

    #[test]
    fn test_parse_missing_end() {
        let input = "*** Begin Patch ***\n*** Add File: src/foo.rs ***\nbar";
        assert!(matches!(
            parse_patch(input),
            Err(PatchParseError::MissingEndMarker)
        ));
    }

    #[test]
    fn test_parse_duplicate_paths() {
        let input = concat!(
            "*** Begin Patch ***\n",
            "*** Add File: src/foo.rs ***\ncontent\n\n",
            "*** Delete File: src/foo.rs ***\n\n",
            "*** End Patch ***"
        );
        assert!(matches!(
            parse_patch(input),
            Err(PatchParseError::DuplicatePath(_))
        ));
    }

    #[test]
    fn test_parse_update_missing_delimiters() {
        let input =
            "*** Begin Patch ***\n*** Update File: src/main.rs ***\njust text\n*** End Patch ***";
        assert!(matches!(
            parse_patch(input),
            Err(PatchParseError::MissingOriginalDelimiter(_))
        ));
    }
}
