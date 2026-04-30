//! Strict parsing primitives for compact DSL model output.
//!
//! The helpers in this module are intentionally small and explicit. They are
//! designed to be reused by later DSL families without turning the parser into
//! a permissive extractor.

use crate::dsl::error::{DslError, DslErrorCode, DslResult, ParseContext};
use crate::dsl::sanitize::{sanitize_control, CRLF_TO_LF};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslField {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DslLine {
    Command { name: String, fields: Vec<DslField> },
    Marker { marker: String },
    Text { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslBlock {
    pub command: String,
    pub fields: Vec<DslField>,
    pub lines: Vec<DslLine>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct DslBlockParser {
    context: ParseContext,
}

impl DslBlockParser {
    pub fn new(context: ParseContext) -> Self {
        Self { context }
    }

    pub fn parse_single_line(&self, raw: &str) -> DslResult<DslBlock> {
        let sanitized = sanitize_control(raw, &self.context)?;
        let trimmed = sanitized.trim();
        reject_wrapped_output(trimmed, &self.context)?;
        if trimmed.is_empty() {
            return Err(DslError::empty(self.context.clone()));
        }

        let (header, remainder) = strip_first_line(trimmed);
        if !remainder.is_empty() {
            return Err(DslError::prose_after(
                self.context.clone(),
                preview_for_error(remainder),
            ));
        }

        let (command, fields) = parse_header_line(header, &self.context)?;
        Ok(DslBlock {
            command: command.clone(),
            fields: fields.clone(),
            lines: vec![DslLine::Command {
                name: command,
                fields,
            }],
            body: String::new(),
        })
    }

    pub fn parse_block(&self, raw: &str, terminator: &str) -> DslResult<DslBlock> {
        let sanitized = sanitize_control(raw, &self.context)?;
        let trimmed = sanitized.trim();
        reject_wrapped_output(trimmed, &self.context)?;
        if trimmed.is_empty() {
            return Err(DslError::empty(self.context.clone()));
        }

        let (header, remainder) = strip_first_line(trimmed);
        let (command, fields) = parse_header_line(header, &self.context)?;
        let (body, trailing) = extract_block_body(remainder, terminator, &self.context)?;
        if !trailing.trim().is_empty() {
            return Err(DslError::prose_after(
                self.context.clone(),
                preview_for_error(&trailing),
            ));
        }

        let mut lines = Vec::new();
        for (offset, line) in body.split('\n').enumerate() {
            let line_context = self.context.with_line(offset + 2);
            lines.push(parse_line(line, &line_context)?);
        }

        Ok(DslBlock {
            command,
            fields,
            lines,
            body,
        })
    }
}

pub fn strip_first_line(text: &str) -> (&str, &str) {
    match text.split_once('\n') {
        Some((first, rest)) => (trim_trailing_cr(first), rest),
        None => (trim_trailing_cr(text), ""),
    }
}

pub fn expect_command<'a>(
    line: &'a str,
    expected: &str,
    context: &ParseContext,
) -> DslResult<&'a str> {
    let trimmed = line.trim();
    let Some((command, rest)) = split_first_token(trimmed) else {
        return Err(DslError::invalid_dsl(
            context.clone(),
            "missing command token",
        ));
    };
    if !is_command_token(command) {
        return Err(DslError::unknown_command(context.clone(), command));
    }
    if command != expected {
        return Err(DslError::unknown_command(context.clone(), command));
    }
    Ok(rest.trim_start())
}

pub fn expect_eol(line: &str, context: &ParseContext) -> DslResult<()> {
    if line.trim().is_empty() {
        Ok(())
    } else {
        Err(DslError::prose_after(
            context.clone(),
            preview_for_error(line),
        ))
    }
}

pub fn expect_key_value(line: &str, context: &ParseContext) -> DslResult<(String, String)> {
    consume_key_value(line.trim(), context)
}

pub fn expect_field_line(
    line: &str,
    expected_key: &str,
    context: &ParseContext,
) -> DslResult<String> {
    let (key, value) = expect_key_value(line, context)?;
    if key != expected_key {
        return Err(DslError::missing_field(context.clone(), expected_key));
    }
    Ok(value)
}

pub fn expect_quoted_field(
    line: &str,
    expected_key: &str,
    context: &ParseContext,
) -> DslResult<String> {
    let trimmed = line.trim();
    let Some((key, value_raw)) = trimmed.split_once('=') else {
        return Err(DslError::missing_field(context.clone(), expected_key));
    };
    if key.trim() != expected_key {
        return Err(DslError::missing_field(context.clone(), expected_key));
    }
    if !value_raw.trim_start().starts_with('"') {
        return Err(DslError::invalid_field_value(
            context.clone(),
            format!("field {expected_key} must be quoted"),
        ));
    }
    let (_, value) = expect_key_value(line, context)?;
    Ok(value)
}

pub fn expect_terminator(line: &str, terminator: &str, context: &ParseContext) -> DslResult<()> {
    let trimmed = trim_trailing_cr(line);
    if trimmed == terminator {
        Ok(())
    } else {
        Err(DslError::missing_end_marker(
            context.clone(),
            format!("missing {terminator}"),
        ))
    }
}

pub fn extract_block_body(
    raw: &str,
    terminator: &str,
    context: &ParseContext,
) -> DslResult<(String, String)> {
    let mut body_lines = Vec::new();
    let mut after_lines = Vec::new();
    let mut found = false;

    for line in raw.split('\n') {
        let line = trim_trailing_cr(line);
        if !found {
            if line == terminator {
                found = true;
                continue;
            }
            body_lines.push(line.to_string());
        } else {
            after_lines.push(line.to_string());
        }
    }

    if !found {
        return Err(DslError::missing_end_marker(
            context.clone(),
            format!("missing {terminator}"),
        ));
    }

    let body = CRLF_TO_LF(&body_lines.join("\n"));
    let trailing = after_lines.join("\n");
    Ok((body, trailing))
}

pub fn parse_line(line: &str, context: &ParseContext) -> DslResult<DslLine> {
    let trimmed_cr = trim_trailing_cr(line);
    if trimmed_cr.is_empty() {
        return Ok(DslLine::Text {
            text: String::new(),
        });
    }
    if trimmed_cr == "---END"
        || trimmed_cr == "END"
        || (trimmed_cr.starts_with("---") && trimmed_cr.len() > 3)
    {
        return Ok(DslLine::Marker {
            marker: trimmed_cr.to_string(),
        });
    }
    if trimmed_cr.starts_with("```") || trimmed_cr.starts_with("~~~") {
        return Err(DslError::fenced(
            context.clone(),
            preview_for_error(trimmed_cr),
        ));
    }

    if trimmed_cr.chars().next().is_some_and(|c| c.is_whitespace()) {
        return Ok(DslLine::Text {
            text: trimmed_cr.to_string(),
        });
    }

    if let Some((command, rest)) = split_first_token(trimmed_cr) {
        if is_command_token(command) {
            let fields = parse_fields(rest, context)?;
            return Ok(DslLine::Command {
                name: command.to_string(),
                fields,
            });
        }
        if command
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        {
            return Err(DslError::unknown_command(context.clone(), command));
        }
        return Ok(DslLine::Text {
            text: trimmed_cr.to_string(),
        });
    }

    Ok(DslLine::Text {
        text: trimmed_cr.to_string(),
    })
}

pub fn require_field<'a>(
    fields: &'a [DslField],
    expected: &str,
    context: &ParseContext,
) -> DslResult<&'a str> {
    fields
        .iter()
        .find(|field| field.key == expected)
        .map(|field| field.value.as_str())
        .ok_or_else(|| DslError::missing_field(context.clone(), expected))
}

pub(crate) fn reject_wrapped_output(text: &str, context: &ParseContext) -> DslResult<()> {
    if text.starts_with("```") || text.starts_with("~~~") {
        return Err(DslError::fenced(context.clone(), preview_for_error(text)));
    }
    if text.starts_with('{') {
        return Err(DslError::json_output(
            context.clone(),
            preview_for_error(text),
        ));
    }
    if text.starts_with('[') {
        return Err(DslError::new(
            DslErrorCode::TomlOutput,
            context.clone(),
            preview_for_error(text),
        ));
    }
    if text.starts_with('<') {
        return Err(DslError::new(
            DslErrorCode::XmlOutput,
            context.clone(),
            preview_for_error(text),
        ));
    }
    if text.starts_with("---") && !text.starts_with("---END") {
        return Err(DslError::new(
            DslErrorCode::YamlOutput,
            context.clone(),
            preview_for_error(text),
        ));
    }
    Ok(())
}

pub(crate) fn parse_header_line(
    line: &str,
    context: &ParseContext,
) -> DslResult<(String, Vec<DslField>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(DslError::empty(context.clone()));
    }
    let Some((command, rest)) = split_first_token(trimmed) else {
        return Err(DslError::invalid_dsl(
            context.clone(),
            "missing command token",
        ));
    };
    if !is_command_token(command) {
        return Err(DslError::unknown_command(context.clone(), command));
    }

    let fields = parse_fields(rest, context)?;
    Ok((command.to_string(), fields))
}

fn parse_fields(mut input: &str, context: &ParseContext) -> DslResult<Vec<DslField>> {
    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
    input = input.trim_start();
    while !input.is_empty() {
        let (field, rest) = consume_field(input, context)?;
        if !seen.insert(field.key.clone()) {
            return Err(DslError::duplicate_field(context.clone(), &field.key));
        }
        fields.push(field);
        input = rest.trim_start();
    }
    Ok(fields)
}

fn consume_key_value(input: &str, context: &ParseContext) -> DslResult<(String, String)> {
    let (field, rest) = consume_field(input, context)?;
    if !rest.trim().is_empty() {
        return Err(DslError::prose_after(
            context.clone(),
            preview_for_error(rest),
        ));
    }
    Ok((field.key, field.value))
}

fn consume_field<'a>(input: &'a str, context: &ParseContext) -> DslResult<(DslField, &'a str)> {
    let input = input.trim_start();
    let Some((key, rest)) = input.split_once('=') else {
        return Err(DslError::invalid_dsl(
            context.clone(),
            format!("expected key=value field, got {input}"),
        ));
    };
    let key = key.trim();
    if !is_field_key(key) {
        return Err(DslError::invalid_dsl(
            context.clone(),
            format!("invalid field key {key:?}"),
        ));
    }
    if key.is_empty() || key.chars().any(|c| c.is_whitespace()) {
        return Err(DslError::invalid_dsl(
            context.clone(),
            format!("invalid field key {key:?}"),
        ));
    }

    let rest = rest.trim_start();
    let (value, remaining) = if let Some(value) = rest.strip_prefix('"') {
        consume_quoted_value(value, context)?
    } else {
        consume_bare_value(rest, context)?
    };

    let field = DslField {
        key: key.to_string(),
        value,
    };
    Ok((field, remaining))
}

fn consume_quoted_value<'a>(
    input: &'a str,
    context: &ParseContext,
) -> DslResult<(String, &'a str)> {
    let mut value = String::new();
    let mut escaped = false;
    for (idx, ch) in input.char_indices() {
        if escaped {
            let pushed = match ch {
                '"' => '"',
                '\\' => '\\',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            };
            value.push(pushed);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                let remaining = &input[idx + ch.len_utf8()..];
                return Ok((value, remaining));
            }
            '\n' | '\r' => {
                return Err(DslError::malformed_quote(
                    context.clone(),
                    preview_for_error(input),
                ))
            }
            other => value.push(other),
        }
    }
    Err(DslError::malformed_quote(
        context.clone(),
        preview_for_error(input),
    ))
}

fn consume_bare_value<'a>(input: &'a str, context: &ParseContext) -> DslResult<(String, &'a str)> {
    let mut end = input.len();
    for (idx, ch) in input.char_indices() {
        if ch.is_whitespace() {
            end = idx;
            break;
        }
    }
    if end == 0 {
        return Err(DslError::invalid_dsl(
            context.clone(),
            "missing field value",
        ));
    }
    let value = input[..end].to_string();
    let remaining = &input[end..];
    Ok((value, remaining))
}

fn split_first_token(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let mut end = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_whitespace() {
            end = idx;
            break;
        }
    }
    Some((&trimmed[..end], &trimmed[end..]))
}

fn is_command_token(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn is_field_key(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn trim_trailing_cr(text: &str) -> &str {
    text.strip_suffix('\r').unwrap_or(text)
}

fn preview_for_error(text: &str) -> String {
    const MAX: usize = 120;
    let mut out = String::new();
    for ch in text.chars().take(MAX) {
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            out.push('?');
        } else {
            out.push(ch);
        }
    }
    if text.chars().count() > MAX {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ParseContext {
        ParseContext {
            dsl_variant: "action",
            line: None,
        }
    }

    #[test]
    fn dsl_strip_first_line_splits() {
        let (first, rest) = strip_first_line("R path=\"src/main.rs\"\nextra");
        assert_eq!(first, "R path=\"src/main.rs\"");
        assert_eq!(rest, "extra");
    }

    #[test]
    fn dsl_parse_single_line_command() {
        let parser = DslBlockParser::new(ctx());
        let parsed = parser
            .parse_single_line("R path=\"src/main.rs\"")
            .expect("single line should parse");
        assert_eq!(parsed.command, "R");
        assert_eq!(parsed.fields.len(), 1);
        assert_eq!(parsed.fields[0].key, "path");
        assert_eq!(parsed.fields[0].value, "src/main.rs");
    }

    #[test]
    fn dsl_parse_block_command() {
        let parser = DslBlockParser::new(ctx());
        let parsed = parser
            .parse_block(
                "E path=\"src/main.rs\"\n---OLD\nold text\n---NEW\nnew text\n---END",
                "---END",
            )
            .expect("block should parse");
        assert_eq!(parsed.command, "E");
        assert_eq!(parsed.fields[0].key, "path");
        assert_eq!(parsed.body, "---OLD\nold text\n---NEW\nnew text");
        assert!(matches!(parsed.lines[0], DslLine::Marker { .. }));
    }

    #[test]
    fn dsl_parse_list_style_command_lines() {
        let parser = DslBlockParser::new(ctx());
        let parsed = parser
            .parse_block(
                "SCOPE objective=\"inspect\"\nF path=\"src/a.rs\"\nF path=\"src/b.rs\"\nEND",
                "END",
            )
            .expect("list-style block should parse");
        assert_eq!(parsed.command, "SCOPE");
        assert_eq!(parsed.lines.len(), 2);
        assert!(matches!(parsed.lines[0], DslLine::Command { .. }));
    }

    #[test]
    fn dsl_rejects_duplicate_fields() {
        let parser = DslBlockParser::new(ctx());
        let err = parser
            .parse_single_line("R path=\"a\" path=\"b\"")
            .unwrap_err();
        assert_eq!(err.code, DslErrorCode::DuplicateField);
    }

    #[test]
    fn dsl_rejects_malformed_quotes() {
        let parser = DslBlockParser::new(ctx());
        let err = parser
            .parse_single_line("R path=\"src/main.rs")
            .unwrap_err();
        assert_eq!(err.code, DslErrorCode::MalformedQuote);
    }

    #[test]
    fn dsl_rejects_fenced_output() {
        let parser = DslBlockParser::new(ctx());
        let err = parser
            .parse_single_line("```dsl\nR path=\"src/main.rs\"\n```")
            .unwrap_err();
        assert_eq!(err.code, DslErrorCode::FencedOutput);
    }

    #[test]
    fn dsl_rejects_json_output() {
        let parser = DslBlockParser::new(ctx());
        let err = parser
            .parse_single_line("{\"path\":\"src/main.rs\"}")
            .unwrap_err();
        assert_eq!(err.code, DslErrorCode::JsonOutput);
    }

    #[test]
    fn agent_protocol_rejects_missing_end_marker() {
        let parser = DslBlockParser::new(ctx());
        let err = parser
            .parse_block(
                "E path=\"src/main.rs\"\n---OLD\nold text\n---NEW\nnew text",
                "---END",
            )
            .unwrap_err();
        assert_eq!(err.code, DslErrorCode::MissingEndMarker);
    }

    #[test]
    fn agent_protocol_rejects_prose_after_command() {
        let parser = DslBlockParser::new(ctx());
        let err = parser
            .parse_single_line("R path=\"src/main.rs\"\nmore text")
            .unwrap_err();
        assert_eq!(err.code, DslErrorCode::ProseAfterCommand);
    }

    #[test]
    fn dsl_parse_line_text_marker_and_command() {
        let marker = parse_line("---OLD", &ctx()).unwrap();
        assert!(matches!(marker, DslLine::Marker { .. }));
        let text = parse_line("  body text", &ctx()).unwrap();
        assert!(matches!(text, DslLine::Text { .. }));
        let cmd = parse_line("F path=\"src/main.rs\"", &ctx()).unwrap();
        assert!(matches!(cmd, DslLine::Command { .. }));
    }
}
