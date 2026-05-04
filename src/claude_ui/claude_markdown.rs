//! @efficiency-role: ui-component
//!
//! Claude Code-style Terminal Markdown Renderer
//!
//! Full markdown parsing using pulldown-cmark with syntect syntax highlighting for code blocks.

use crate::ui_theme::*;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::prelude::*;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AssistantContent {
    pub raw_markdown: String,
    pub blocks: Vec<AssistantBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AssistantBlock {
    Paragraph(String),
    List(String),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    CommandSuggestion {
        language: String,
        commands: Vec<String>,
    },
    Table(String),
    Rule,
    Callout(String),
}

// ─── Structured Markdown IR ───────────────────────────────────────────────

/// Intermediate representation of a rendered markdown block.
/// Separates parsing from rendering: `parse_markdown` produces these blocks,
/// then `render_blocks_to_lines` converts them to Ratatui `Line`s.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RenderBlock {
    Paragraph(Vec<Line<'static>>),
    Heading {
        level: u8,
        content: Vec<Line<'static>>,
    },
    CodeBlock {
        language: Option<String>,
        lines: Vec<String>,
    },
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<Vec<Line<'static>>>,
    },
    BlockQuote(Vec<Line<'static>>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Rule,
}

impl AssistantContent {
    pub(crate) fn from_markdown(text: &str) -> Self {
        let normalized = normalize_terminal_markdown(text);
        let blocks = parse_assistant_blocks(&normalized);
        Self {
            raw_markdown: normalized,
            blocks,
        }
    }
}

pub(crate) fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

pub(crate) fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

fn trim_blank_lines(lines: &mut Vec<Line<'static>>) {
    while matches!(lines.first(), Some(line) if line.spans.is_empty()) {
        lines.remove(0);
    }
    while matches!(lines.last(), Some(line) if line.spans.is_empty()) {
        lines.pop();
    }
}

// ─── Parser Phase: Markdown → Vec<RenderBlock> ──────────────────────────

/// Parse markdown into structured blocks with inline styles applied to `Span`s.
/// Code blocks store raw lines (syntect applied later by renderer).
/// Tables store raw cell strings (column alignment applied later).
pub(crate) fn parse_markdown(text: &str) -> Vec<RenderBlock> {
    let theme = current_theme();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(text, options);
    let mut blocks = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();
    let mut code_block_lang: Option<String> = None;
    let mut in_list = false;
    let mut list_ordered = false;
    let mut list_start: Option<u64> = None;
    let mut list_items: Vec<Vec<Line<'static>>> = Vec::new();
    let mut current_item_lines: Vec<Line<'static>> = Vec::new();
    let mut in_blockquote = false;
    let mut blockquote_lines: Vec<Line<'static>> = Vec::new();
    let mut in_table = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut pending_cell = String::new();
    let mut seen_first_table_row = false; // Header cells come before the first TableRow
    let mut in_bold = false;
    let mut in_italic = false;
    let mut in_strikethrough = false;
    let mut in_link = false;
    let mut link_text = String::new();
    let mut link_url = String::new();
    let mut pending_text = String::new();

    // Helper: flush pending inline text into spans with current style
    fn do_flush(
        pending: &mut String,
        spans: &mut Vec<Span<'static>>,
        bold: bool,
        italic: bool,
        strikethrough: bool,
        theme: &Theme,
    ) {
        if !pending.is_empty() {
            spans.push(Span::styled(
                std::mem::take(pending),
                get_current_style(bold, italic, strikethrough, theme),
            ));
        }
    }

    // Helper: push accumulated spans as a line
    fn do_push(spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>) {
        if !spans.is_empty() {
            lines.push(Line::from(std::mem::take(spans)));
        }
    }

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_block_lang = match &kind {
                        CodeBlockKind::Fenced(lang) => {
                            if lang.is_empty() {
                                None
                            } else {
                                Some(lang.to_string())
                            }
                        }
                        CodeBlockKind::Indented => None,
                    };
                    code_block_lines.clear();
                }
                Tag::Heading { level, .. } => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    do_push(&mut current_spans, &mut current_lines);
                    current_spans.push(Span::styled(
                        String::new(),
                        Style::default()
                            .fg(theme.fg.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Tag::Paragraph => {}
                Tag::List(start) => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    do_push(&mut current_spans, &mut current_lines);
                    if !current_lines.is_empty() {
                        blocks.push(RenderBlock::Paragraph(std::mem::take(&mut current_lines)));
                    }
                    in_list = true;
                    list_ordered = start.is_some();
                    list_start = start;
                    list_items.clear();
                }
                Tag::Item => {
                    do_push(&mut current_spans, &mut current_item_lines);
                    current_item_lines.clear();
                }
                Tag::BlockQuote(_) => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    do_push(&mut current_spans, &mut current_lines);
                    if !current_lines.is_empty() {
                        blocks.push(RenderBlock::Paragraph(std::mem::take(&mut current_lines)));
                    }
                    in_blockquote = true;
                    blockquote_lines.clear();
                }
                Tag::Table(_) => {
                    in_table = true;
                    seen_first_table_row = false;
                    table_headers.clear();
                    table_rows.clear();
                }
                Tag::TableHead => {}
                Tag::TableRow => {
                    seen_first_table_row = true;
                }
                Tag::TableCell => {
                    pending_cell.clear();
                }
                Tag::Emphasis => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    in_italic = true;
                }
                Tag::Strong => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    in_bold = true;
                }
                Tag::Strikethrough => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    in_strikethrough = true;
                }
                Tag::Link { dest_url, .. } => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    in_link = true;
                    link_url = dest_url.to_string();
                    link_text.clear();
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    // Trim trailing newlines from last line
                    if let Some(last) = code_block_lines.last_mut() {
                        while last.ends_with('\n') {
                            last.pop();
                        }
                    }
                    // Remove empty lines at end
                    while code_block_lines.last().map_or(false, |l| l.is_empty()) {
                        code_block_lines.pop();
                    }
                    blocks.push(RenderBlock::CodeBlock {
                        language: code_block_lang.take(),
                        lines: std::mem::take(&mut code_block_lines),
                    });
                }
                TagEnd::Heading(level) => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    do_push(&mut current_spans, &mut current_lines);
                    let heading_lines = std::mem::take(&mut current_lines);
                    blocks.push(RenderBlock::Heading {
                        level: match level {
                            HeadingLevel::H1 => 1,
                            HeadingLevel::H2 => 2,
                            HeadingLevel::H3 => 3,
                            HeadingLevel::H4 => 4,
                            HeadingLevel::H5 => 5,
                            HeadingLevel::H6 => 6,
                        },
                        content: heading_lines,
                    });
                }
                TagEnd::Paragraph => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    do_push(&mut current_spans, &mut current_lines);
                    if in_blockquote {
                        blockquote_lines.append(&mut current_lines);
                    } else if !current_lines.is_empty() {
                        blocks.push(RenderBlock::Paragraph(std::mem::take(&mut current_lines)));
                    }
                }
                TagEnd::List(_) => {
                    in_list = false;
                    // Flush last item
                    if !current_item_lines.is_empty() {
                        list_items.push(std::mem::take(&mut current_item_lines));
                    }
                    if !list_items.is_empty() {
                        blocks.push(RenderBlock::List {
                            ordered: list_ordered,
                            start: list_start,
                            items: std::mem::take(&mut list_items),
                        });
                    }
                }
                TagEnd::Item => {
                    do_push(&mut current_spans, &mut current_item_lines);
                    if !current_item_lines.is_empty() || !pending_text.is_empty() {
                        do_flush(
                            &mut pending_text,
                            &mut current_spans,
                            in_bold,
                            in_italic,
                            in_strikethrough,
                            theme,
                        );
                        do_push(&mut current_spans, &mut current_item_lines);
                    }
                    if !current_item_lines.is_empty() {
                        list_items.push(std::mem::take(&mut current_item_lines));
                    }
                    current_item_lines.clear();
                }
                TagEnd::BlockQuote(_) => {
                    in_blockquote = false;
                    if !blockquote_lines.is_empty() {
                        blocks.push(RenderBlock::BlockQuote(std::mem::take(
                            &mut blockquote_lines,
                        )));
                    }
                }
                TagEnd::Table => {
                    in_table = false;
                    // Flush any pending row
                    if !current_row.is_empty() {
                        table_rows.push(std::mem::take(&mut current_row));
                    }
                    blocks.push(RenderBlock::Table {
                        headers: std::mem::take(&mut table_headers),
                        rows: std::mem::take(&mut table_rows),
                    });
                }
                TagEnd::TableRow => {
                    if !current_row.is_empty() {
                        table_rows.push(std::mem::take(&mut current_row));
                    }
                    pending_cell.clear();
                }
                TagEnd::TableHead => {}
                TagEnd::TableCell => {
                    if !seen_first_table_row {
                        table_headers.push(std::mem::take(&mut pending_cell));
                    } else {
                        current_row.push(std::mem::take(&mut pending_cell));
                    }
                }
                TagEnd::Emphasis => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    in_italic = false;
                }
                TagEnd::Strong => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    in_bold = false;
                }
                TagEnd::Strikethrough => {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    in_strikethrough = false;
                }
                TagEnd::Link => {
                    in_link = false;
                    current_spans.push(Span::styled(
                        if link_text.is_empty() {
                            link_url.clone()
                        } else {
                            std::mem::take(&mut link_text)
                        },
                        Style::default()
                            .fg(theme.accent_secondary.to_ratatui_color())
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    link_url.clear();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_block_lines.push(text.to_string());
                } else if in_link {
                    link_text.push_str(&text);
                } else if in_blockquote {
                    // Accumulate blockquote text as lines; inline styling applied normally
                    pending_text.push_str(&text);
                } else if in_table {
                    pending_cell.push_str(&text);
                } else {
                    pending_text.push_str(&text);
                }
            }
            Event::Code(text) => {
                if in_code_block {
                    code_block_lines.push(text.to_string());
                } else {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    current_spans.push(Span::styled(
                        format!("`{}`", text),
                        Style::default()
                            .fg(theme.accent_secondary.to_ratatui_color())
                            .add_modifier(Modifier::DIM),
                    ));
                }
            }
            Event::SoftBreak => {
                if in_code_block {
                    // Append to last line (code block lines are raw)
                    if let Some(last) = code_block_lines.last_mut() {
                        last.push('\n');
                    }
                } else {
                    pending_text.push(' ');
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    if let Some(last) = code_block_lines.last_mut() {
                        last.push('\n');
                    }
                } else {
                    do_flush(
                        &mut pending_text,
                        &mut current_spans,
                        in_bold,
                        in_italic,
                        in_strikethrough,
                        theme,
                    );
                    do_push(&mut current_spans, &mut current_lines);
                }
            }
            Event::Rule => {
                do_flush(
                    &mut pending_text,
                    &mut current_spans,
                    in_bold,
                    in_italic,
                    in_strikethrough,
                    theme,
                );
                do_push(&mut current_spans, &mut current_lines);
                if !current_lines.is_empty() {
                    blocks.push(RenderBlock::Paragraph(std::mem::take(&mut current_lines)));
                }
                blocks.push(RenderBlock::Rule);
            }
            Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            _ => {}
        }
    }

    // Flush any remaining content
    if in_blockquote {
        do_flush(
            &mut pending_text,
            &mut current_spans,
            in_bold,
            in_italic,
            in_strikethrough,
            theme,
        );
        do_push(&mut current_spans, &mut blockquote_lines);
        if !blockquote_lines.is_empty() {
            blocks.push(RenderBlock::BlockQuote(std::mem::take(
                &mut blockquote_lines,
            )));
        }
    } else {
        do_flush(
            &mut pending_text,
            &mut current_spans,
            in_bold,
            in_italic,
            in_strikethrough,
            theme,
        );
        do_push(&mut current_spans, &mut current_lines);
        if !current_lines.is_empty() {
            blocks.push(RenderBlock::Paragraph(std::mem::take(&mut current_lines)));
        }
    }

    // If no blocks produced, create a single paragraph from the raw text
    if blocks.is_empty() {
        blocks.push(RenderBlock::Paragraph(vec![Line::from(vec![
            Span::styled(
                text.to_string(),
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            ),
        ])]));
    }

    blocks
}

// ─── Renderer Phase: Vec<RenderBlock> → Vec<Line<'static>> ──────────────

/// Render structured blocks to Ratatui `Line`s with block-level styling:
/// code highlighting, table borders, heading spacing, list markers, etc.
pub(crate) fn render_blocks_to_lines(
    blocks: &[RenderBlock],
    theme: &Theme,
    width: usize,
) -> Vec<Line<'static>> {
    let mut output = Vec::new();

    for block in blocks {
        if !output.is_empty() {
            // Don't add blank line for the first block
            match block {
                RenderBlock::Paragraph(_) => {}
                _ => {
                    output.push(Line::default());
                }
            }
        }

        match block {
            RenderBlock::Paragraph(lines) => {
                output.extend(lines.clone());
            }
            RenderBlock::Heading { level, content } => {
                let heading_style = Style::default()
                    .fg(match level {
                        1 => theme.accent_primary.to_ratatui_color(),
                        _ => theme.fg.to_ratatui_color(),
                    })
                    .add_modifier(Modifier::BOLD);
                for line in content {
                    let styled_spans: Vec<Span<'static>> = line
                        .spans
                        .iter()
                        .map(|s| Span::styled(s.content.clone(), heading_style))
                        .collect();
                    output.push(Line::from(styled_spans));
                }
            }
            RenderBlock::CodeBlock { language, lines } => {
                let code_text = lines.join("\n");
                let code_output = render_code_block(
                    &code_text,
                    language.as_deref().unwrap_or(""),
                    theme,
                    width,
                    false,
                );
                output.extend(code_output);
            }
            RenderBlock::List {
                ordered,
                start,
                items,
            } => {
                let mut counter = start.unwrap_or(1);
                for item in items {
                    let marker = if *ordered {
                        let m = format!("{}. ", counter);
                        counter += 1;
                        m
                    } else {
                        format!("{} ", BULLET)
                    };
                    if let Some(first_line) = item.first() {
                        let mut spans = vec![Span::styled(
                            marker,
                            Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                        )];
                        spans.extend(first_line.spans.clone());
                        output.push(Line::from(spans));
                        for remaining in &item[1..] {
                            let mut indent_spans = vec![Span::raw("  ")];
                            indent_spans.extend(remaining.spans.clone());
                            output.push(Line::from(indent_spans));
                        }
                    }
                }
            }
            RenderBlock::BlockQuote(lines) => {
                for line in lines {
                    output.push(Line::from(vec![
                        Span::styled(
                            format!("{} ", BLOCKQUOTE_BAR),
                            Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                        ),
                        Span::styled(
                            line.spans
                                .iter()
                                .map(|s| s.content.as_ref())
                                .collect::<Vec<_>>()
                                .join(""),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]));
                }
            }
            RenderBlock::Table { headers, rows } => {
                let all_rows: Vec<Vec<String>> = if headers.is_empty() {
                    rows.clone()
                } else {
                    let mut h = vec![headers.clone()];
                    h.extend(rows.clone());
                    h
                };
                let table_lines = render_table_boxed(&all_rows, headers.is_empty(), theme, width);
                output.extend(table_lines);
            }
            RenderBlock::Rule => {
                output.push(Line::from(vec![Span::styled(
                    "─".repeat(width.saturating_sub(2).max(6)),
                    Style::default().fg(theme.fg_dim.to_ratatui_color()),
                )]));
            }
        }
    }

    // Add trailing blank line
    if !output.is_empty() {
        output.push(Line::default());
    }

    trim_blank_lines(&mut output);
    collapse_blank_runs(output)
}

// ─── New table renderer with box-drawing borders ───────────────────────

fn render_table_boxed(
    rows: &[Vec<String>],
    _no_header: bool,
    theme: &Theme,
    width: usize,
) -> Vec<Line<'static>> {
    if rows.is_empty() {
        return Vec::new();
    }

    let num_cols = rows[0].len();
    if num_cols == 0 {
        return Vec::new();
    }

    // Compute column widths capped by available width
    let max_col_width = (width.saturating_sub(3 * num_cols + 1) / num_cols).max(3);
    let mut col_widths = vec![0; num_cols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell.len().min(max_col_width));
            }
        }
    }
    // Ensure minimum width
    for w in col_widths.iter_mut() {
        *w = (*w).max(1);
    }

    let border_style = Style::default().fg(theme.fg_dim.to_ratatui_color());
    let header_style = Style::default()
        .fg(theme.fg.to_ratatui_color())
        .add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(theme.fg.to_ratatui_color());

    let mut output = Vec::new();

    // Build a horizontal border line
    let make_border = |left: &str, mid: &str, right: &str| -> Line<'static> {
        let mut spans = Vec::new();
        spans.push(Span::styled(left.to_string(), border_style));
        for (i, w) in col_widths.iter().enumerate() {
            spans.push(Span::styled("─".repeat(*w + 2), border_style));
            if i < num_cols - 1 {
                spans.push(Span::styled(mid.to_string(), border_style));
            }
        }
        spans.push(Span::styled(right.to_string(), border_style));
        Line::from(spans)
    };

    // Top border
    output.push(make_border("┌", "┬", "┐"));

    // Rows
    for (row_idx, row) in rows.iter().enumerate() {
        let mut spans = Vec::new();
        spans.push(Span::styled("│ ", border_style));
        for (col_idx, cell) in row.iter().enumerate() {
            let width = col_widths[col_idx];
            let truncated: String = cell.chars().take(width).collect();
            let padded = format!("{:<width$}", truncated, width = width);
            let style = if row_idx == 0 {
                header_style
            } else {
                cell_style
            };
            spans.push(Span::styled(padded, style));
            if col_idx < num_cols - 1 {
                spans.push(Span::styled(" │ ", border_style));
            }
        }
        spans.push(Span::styled(" │", border_style));
        output.push(Line::from(spans));

        // Separator after header row
        if row_idx == 0 && rows.len() > 1 {
            output.push(make_border("├", "┼", "┤"));
        }
    }

    // Bottom border
    output.push(make_border("└", "┴", "┘"));

    output
}

// ─── Refactored render_markdown_ratatui (thin wrapper) ──────────────────

fn collapse_blank_runs(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let mut normalized = Vec::new();
    let mut prev_blank = false;
    for line in lines {
        let is_blank = line.spans.is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        normalized.push(line);
    }
    normalized
}

pub(crate) fn render_markdown_ratatui(text: &str) -> Vec<Line<'static>> {
    render_markdown_ratatui_with_width(text, 80)
}

pub(crate) fn render_markdown_ratatui_with_width(text: &str, width: usize) -> Vec<Line<'static>> {
    let blocks = parse_markdown(text);
    render_blocks_to_lines(&blocks, current_theme(), width)
}

fn get_current_style(bold: bool, italic: bool, strikethrough: bool, theme: &Theme) -> Style {
    let mut style = Style::default().fg(theme.fg_dim.to_ratatui_color());
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if strikethrough {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }
    style
}

fn render_code_block(
    content: &str,
    lang: &str,
    theme: &Theme,
    _width: usize,
    shell_like: bool,
) -> Vec<Line<'static>> {
    let ss = get_syntax_set();
    let ts = get_theme_set();

    let syntax = if lang.is_empty() {
        ss.find_syntax_by_name("Plain Text").unwrap()
    } else {
        ss.find_syntax_by_token(lang)
            .or_else(|| ss.find_syntax_by_name(lang))
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    };

    let theme_name = "base16-ocean.dark";
    let highlight_theme = ts
        .themes
        .get(theme_name)
        .unwrap_or(&ts.themes["InspiredGitHub"]);

    let mut highlighter = HighlightLines::new(syntax, highlight_theme);
    let lines: Vec<&str> = content.lines().collect();

    let mut output = Vec::new();
    let accent = if shell_like {
        theme.accent_secondary.to_ratatui_color()
    } else {
        theme.accent_primary.to_ratatui_color()
    };
    let lang_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let header_text = if lang.is_empty() { "text" } else { lang };
    let header_label = if shell_like {
        format!(
            "command {}",
            if header_text.is_empty() {
                "shell"
            } else {
                header_text
            }
        )
    } else {
        header_text.to_string()
    };
    output.push(Line::from(vec![Span::styled(header_label, lang_style)]));

    for line in lines {
        let highlighted = highlighter.highlight_line(line, ss).unwrap_or_default();
        let spans: Vec<Span<'static>> = highlighted
            .iter()
            .filter_map(|(style, text)| {
                if text.is_empty() {
                    return None;
                }
                let fg = style.foreground;
                let ratatui_fg = Color::Rgb(fg.r, fg.g, fg.b);
                Some(Span::styled(
                    text.to_string(),
                    Style::default().fg(ratatui_fg),
                ))
            })
            .collect();

        let mut line_spans = vec![Span::raw("  ")];
        line_spans.extend(spans);
        let line_content = Line::from(line_spans);
        output.push(line_content);
    }

    output
}

pub(crate) fn render_assistant_content(
    content: &AssistantContent,
    width: usize,
) -> Vec<Line<'static>> {
    let theme = current_theme();
    let mut lines = Vec::new();
    let mut prev_block: Option<&AssistantBlock> = None;

    for block in content.blocks.iter() {
        if prev_block.is_some_and(|prev| needs_block_separator(prev, block)) {
            lines.push(Line::default());
        }
        let mut rendered = match block {
            AssistantBlock::Paragraph(text)
            | AssistantBlock::List(text)
            | AssistantBlock::Table(text)
            | AssistantBlock::Callout(text) => render_markdown_ratatui_with_width(text, width),
            AssistantBlock::Rule => vec![Line::from(Span::styled(
                "─".repeat(width.saturating_sub(2).max(6)),
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            ))],
            AssistantBlock::CodeBlock { language, code } => {
                render_code_block(code, language.as_deref().unwrap_or(""), theme, width, false)
            }
            AssistantBlock::CommandSuggestion { language, commands } => {
                render_command_suggestion(commands, language, width, theme)
            }
        };
        trim_blank_lines(&mut rendered);
        lines.extend(rendered);
        prev_block = Some(block);
    }

    trim_blank_lines(&mut lines);
    collapse_blank_runs(lines)
}

fn needs_block_separator(prev: &AssistantBlock, current: &AssistantBlock) -> bool {
    matches!(
        (prev, current),
        (AssistantBlock::Rule, _)
            | (_, AssistantBlock::Rule)
            | (AssistantBlock::CodeBlock { .. }, _)
            | (_, AssistantBlock::CodeBlock { .. })
            | (AssistantBlock::CommandSuggestion { .. }, _)
            | (_, AssistantBlock::CommandSuggestion { .. })
            | (AssistantBlock::Table(_), _)
            | (_, AssistantBlock::Table(_))
            | (AssistantBlock::Callout(_), _)
            | (_, AssistantBlock::Callout(_))
    )
}

fn render_command_suggestion(
    commands: &[String],
    language: &str,
    _width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let label_style = Style::default()
        .fg(theme.accent_secondary.to_ratatui_color())
        .add_modifier(Modifier::BOLD);
    let command_style = Style::default().fg(theme.fg.to_ratatui_color());
    let mut out = vec![Line::from(vec![
        Span::styled("suggested ", label_style),
        Span::styled(
            language.to_string(),
            command_style.add_modifier(Modifier::BOLD),
        ),
    ])];
    for cmd in commands {
        out.push(Line::from(vec![
            Span::styled("$ ", label_style),
            Span::styled(cmd.clone(), command_style),
        ]));
    }
    out
}

fn normalize_terminal_markdown(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let mut filtered = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let next = lines.get(i + 1).map(|l| l.trim()).unwrap_or_default();
        if (trimmed.eq_ignore_ascii_case("command") || trimmed.eq_ignore_ascii_case("commands"))
            && next.starts_with("```")
        {
            i += 1;
            continue;
        }
        filtered.push(line.trim_end());
        i += 1;
    }
    filtered.join("\n").trim().to_string()
}

fn parse_assistant_blocks(text: &str) -> Vec<AssistantBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;

    while i < lines.len() {
        while i < lines.len() && lines[i].trim().is_empty() {
            i += 1;
        }
        if i >= lines.len() {
            break;
        }

        let line = lines[i].trim_end();
        if let Some(lang) = parse_fence_language(line) {
            i += 1;
            let mut code = Vec::new();
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                code.push(lines[i].trim_end().to_string());
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            let code_text = code.join("\n");
            if is_shell_language(&lang) {
                let commands = code
                    .iter()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>();
                blocks.push(AssistantBlock::CommandSuggestion {
                    language: if lang.is_empty() {
                        "shell".to_string()
                    } else {
                        lang
                    },
                    commands,
                });
            } else {
                blocks.push(AssistantBlock::CodeBlock {
                    language: if lang.is_empty() { None } else { Some(lang) },
                    code: code_text,
                });
            }
            continue;
        }

        if is_rule_line(line) {
            blocks.push(AssistantBlock::Rule);
            i += 1;
            continue;
        }

        let mut block_lines = vec![line.to_string()];
        let current_is_list = starts_with_list_item(line);
        i += 1;
        while i < lines.len()
            && !lines[i].trim().is_empty()
            && parse_fence_language(lines[i]).is_none()
            && !is_rule_line(lines[i].trim_end())
        {
            let next_line = lines[i].trim_end();
            // Split at heading boundaries
            if starts_with_heading(next_line) {
                break;
            }
            // Split when a list item appears after a non-list block
            if !current_is_list && starts_with_list_item(next_line) {
                break;
            }
            block_lines.push(next_line.to_string());
            i += 1;
        }
        let block_text = block_lines.join("\n").trim().to_string();
        if block_text.is_empty() {
            continue;
        }
        blocks.push(classify_markdown_block(&block_text));
    }

    blocks
}

fn parse_fence_language(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("```") {
        return None;
    }
    Some(trimmed.trim_start_matches("```").trim().to_lowercase())
}

fn is_shell_language(lang: &str) -> bool {
    matches!(lang, "" | "bash" | "sh" | "shell" | "zsh" | "fish")
}

fn starts_with_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    let hash_count = trimmed.chars().take_while(|c| *c == '#').count();
    hash_count >= 1 && hash_count <= 6
        && trimmed.len() > hash_count
        && trimmed.chars().nth(hash_count) == Some(' ')
}

fn starts_with_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ")
        || trimmed.starts_with("+ ")
    {
        return true;
    }
    // * item but NOT **bold** (two asterisks without space)
    if let Some(rest) = trimmed.strip_prefix("* ") {
        // "* " at index 0 and NOT preceded by another * (i.e., not "**ing**")
        if trimmed.len() > 2
            && !trimmed.starts_with("**")
        {
            return true;
        }
        return true;
    }
    // Ordered list: "1. ", "123. " with space
    let chars: Vec<char> = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !chars.is_empty() {
        let rest = &trimmed[chars.len()..];
        rest.starts_with(". ")
    } else {
        false
    }
}

fn is_rule_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3
        && trimmed
            .chars()
            .all(|ch| ch == '-' || ch == '*' || ch == '_')
}

fn classify_markdown_block(text: &str) -> AssistantBlock {
    let first = text.lines().next().unwrap_or_default().trim_start();
    if starts_with_list_item(first) {
        AssistantBlock::List(text.to_string())
    } else if first.starts_with(">") {
        AssistantBlock::Callout(text.to_string())
    } else if text.lines().take(2).all(|line| line.contains('|')) {
        AssistantBlock::Table(text.to_string())
    } else {
        AssistantBlock::Paragraph(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[test]
    fn test_render_headers() {
        let lines = render_markdown_ratatui("# Hello World");
        assert!(!lines.is_empty());
        let text = extract_text(&lines);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_render_code_block() {
        let md = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let lines = render_markdown_ratatui(md);
        assert!(lines.len() > 3);
        let text = extract_text(&lines);
        assert!(text.contains("rust"));
        assert!(text.contains("fn main()"));
    }

    #[test]
    fn test_render_bullet_list() {
        let md = "- Item 1\n- Item 2\n- Item 3";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        assert!(text.contains("• Item 1"));
        assert!(text.contains("• Item 2"));
    }

    #[test]
    fn test_render_numbered_list() {
        let md = "1. First\n2. Second\n3. Third";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        assert!(text.contains("1. First"));
        assert!(text.contains("2. Second"));
    }

    #[test]
    fn test_render_link() {
        let md = "[Click here](https://example.com)";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        assert!(text.contains("Click here"));
    }

    #[test]
    fn test_render_bold_and_italic() {
        let md = "**bold** and *italic* text";
        let lines = render_markdown_ratatui(md);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_inline_code() {
        let md = "Use `println!` macro";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        assert!(text.contains("Use `println!` macro"));
    }

    #[test]
    fn test_render_inline_code_preserves_order() {
        let md = "A `code` B";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        assert!(text.contains("A `code` B"));
    }

    #[test]
    fn test_soft_break_creates_space() {
        let md = "First line\nSecond line";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        assert!(text.contains("First line Second line"));
    }

    #[test]
    fn test_render_blockquote() {
        let md = "> This is a quote";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        assert!(text.contains("This is a quote"));
    }

    #[test]
    fn test_render_table() {
        let md = "| Name | Value |\n|------|-------|\n| Foo | 42 |\n| Bar | 99 |";
        let lines = render_markdown_ratatui(md);
        assert!(lines.len() >= 3);
        let text = extract_text(&lines);
        println!("Table output:\n{}", text);
        assert!(text.contains("Name") || text.contains("Foo"));
    }

    #[test]
    fn test_render_horizontal_rule() {
        let md = "Before\n\n---\n\nAfter";
        let lines = render_markdown_ratatui(md);
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_assistant_content_extracts_shell_commands() {
        let content = AssistantContent::from_markdown("```bash\nchafa image.png\n```");
        assert!(matches!(
            content.blocks.first(),
            Some(AssistantBlock::CommandSuggestion { commands, .. }) if commands == &vec!["chafa image.png".to_string()]
        ));
    }

    #[test]
    fn test_assistant_content_removes_orphan_command_label() {
        let content = AssistantContent::from_markdown("command\n```bash\nfile image.png\n```");
        assert_eq!(content.raw_markdown, "```bash\nfile image.png\n```");
    }

    #[test]
    fn test_render_assistant_content_trims_leading_blank_blocks() {
        let content = AssistantContent::from_markdown("\n\n```bash\nexa\n```");
        let lines = render_assistant_content(&content, 60);
        assert!(!lines.is_empty());
        assert!(!lines[0].spans.is_empty());
    }

    #[test]
    fn test_render_assistant_content_collapses_blank_runs() {
        let content = AssistantContent::from_markdown("First\n\n\nSecond");
        let lines = render_assistant_content(&content, 60);
        let blank_count = lines.iter().filter(|line| line.spans.is_empty()).count();
        assert_eq!(blank_count, 0);
    }

    #[test]
    fn test_command_suggestion_render_is_compact() {
        let content = AssistantContent::from_markdown("```bash\nexa -la\n```");
        let lines = render_assistant_content(&content, 60);
        let text = extract_text(&lines);
        assert!(text.contains("suggested bash"));
        assert!(text.contains("$ exa -la"));
        assert!(lines.len() <= 2);
    }

    #[test]
    fn test_paragraph_to_list_has_no_extra_separator() {
        let content = AssistantContent::from_markdown("Intro text\n\n- one\n- two");
        let lines = render_assistant_content(&content, 60);
        let blank_count = lines.iter().filter(|line| line.spans.is_empty()).count();
        assert!(blank_count <= 1);
    }

    // ─── New pipeline tests (parse_markdown + render_blocks_to_lines) ───

    #[test]
    fn test_parse_bold_italic_nesting() {
        let md = "**bold *and italic***";
        let blocks = parse_markdown(md);
        assert!(!blocks.is_empty(), "Should produce at least one block");
        // Check that we got a paragraph with styled spans
        match &blocks[0] {
            RenderBlock::Paragraph(lines) => {
                let text = lines
                    .iter()
                    .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
                    .collect::<String>();
                assert!(text.contains("bold"));
                assert!(text.contains("and italic"));
            }
            _ => panic!("Expected Paragraph, got {:?}", blocks[0]),
        }
    }

    #[test]
    fn test_parse_headings_levels() {
        let md = "# H1\n## H2\n### H3";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0], RenderBlock::Heading { level: 1, .. }));
        assert!(matches!(blocks[1], RenderBlock::Heading { level: 2, .. }));
        assert!(matches!(blocks[2], RenderBlock::Heading { level: 3, .. }));
    }

    #[test]
    fn test_parse_ordered_list() {
        let md = "1. First\n2. Second\n3. Third";
        let blocks = parse_markdown(md);
        assert!(
            matches!(&blocks[0], RenderBlock::List { ordered: true, items, .. } if items.len() == 3),
            "Expected ordered list with 3 items"
        );
    }

    #[test]
    fn test_parse_unordered_list() {
        let md = "- Item A\n- Item B";
        let blocks = parse_markdown(md);
        assert!(
            matches!(&blocks[0], RenderBlock::List { ordered: false, items, .. } if items.len() == 2),
            "Expected unordered list with 2 items"
        );
    }

    #[test]
    fn test_parse_table_headers_and_rows() {
        let md = "| Name | Value |\n|------|-------|\n| Foo  | 42    |\n| Bar  | 99    |";
        let blocks = parse_markdown(md);
        eprintln!("Table blocks: {:?}", blocks);
        match &blocks[0] {
            RenderBlock::Table { headers, rows } => {
                assert_eq!(headers, &["Name", "Value"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], &["Foo", "42"]);
            }
            other => panic!("Expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_render_table_boxed_borders() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let lines = render_markdown_ratatui(md);
        let text = extract_text(&lines);
        // Should contain box-drawing characters
        assert!(
            text.contains('│'),
            "Should have vertical box borders, got: {}",
            text
        );
        assert!(
            text.contains('─'),
            "Should have horizontal box borders, got: {}",
            text
        );
        assert!(text.contains('A'), "Should contain header cell A");
        assert!(text.contains('1'), "Should contain data cell 1");
    }

    #[test]
    fn test_parse_code_block_language() {
        let md = "```rust\nfn main() {}\n```";
        let blocks = parse_markdown(md);
        match &blocks[0] {
            RenderBlock::CodeBlock { language, lines } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert_eq!(lines, &["fn main() {}"]);
            }
            other => panic!("Expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let md = "> This is a quote";
        let blocks = parse_markdown(md);
        eprintln!("Blocks for blockquote: {:?}", blocks);
        assert!(
            matches!(&blocks[0], RenderBlock::BlockQuote(_)),
            "Expected BlockQuote, got {:?}",
            blocks
        );
    }

    #[test]
    fn test_parse_hard_break() {
        let md = "Line one  \nLine two";
        let blocks = parse_markdown(md);
        // Trailing double-space + newline = HardBreak → two lines
        match &blocks[0] {
            RenderBlock::Paragraph(lines) => {
                assert_eq!(lines.len(), 2, "Hard break should produce two lines");
            }
            _ => panic!("Expected Paragraph"),
        }
    }

    #[test]
    fn test_parse_soft_break() {
        let md = "Line one\nLine two";
        let blocks = parse_markdown(md);
        match &blocks[0] {
            RenderBlock::Paragraph(lines) => {
                assert_eq!(lines.len(), 1, "Soft break should produce one line");
            }
            _ => panic!("Expected Paragraph"),
        }
    }

    #[test]
    fn test_parse_rule() {
        let md = "Before\n\n---\n\nAfter";
        let blocks = parse_markdown(md);
        assert!(
            blocks.iter().any(|b| matches!(b, RenderBlock::Rule)),
            "Should contain a Rule block"
        );
        // Should have Para, Rule, Para = 3 blocks
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn test_blocks_to_lines_produces_output() {
        let blocks = parse_markdown("Hello world");
        let lines = render_blocks_to_lines(&blocks, current_theme(), 80);
        assert!(!lines.is_empty());
        assert!(!lines[0].spans.is_empty());
    }

    #[test]
    fn test_no_ansi_in_ratatui_pipeline() {
        let lines = render_markdown_ratatui("**bold** `code`");
        let joined: String = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();

        assert!(!joined.contains("\x1b["));
    }

    #[test]
    fn test_assistant_content_uses_actual_width_for_rules() {
        let content = AssistantContent::from_markdown("---");
        let lines = render_assistant_content(&content, 24);
        let rendered: String = lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert_eq!(rendered.chars().count(), 22);
    }

    #[test]
    fn test_width_aware_wrapper_controls_table_width() {
        let md = "| Header | Value |\n|---|---|\n| AlphaBetaGamma | DeltaEpsilon |";
        let narrow = render_markdown_ratatui_with_width(md, 24);
        let wide = render_markdown_ratatui_with_width(md, 80);

        let narrow_top: String = narrow[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        let wide_top: String = wide[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert!(narrow_top.chars().count() < wide_top.chars().count());
        assert!(narrow_top.chars().count() <= 24);
    }
}
