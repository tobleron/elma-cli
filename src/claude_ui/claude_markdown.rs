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

fn push_blank_line(lines: &mut Vec<Line<'static>>) {
    if !matches!(lines.last(), Some(last) if last.spans.is_empty()) {
        lines.push(Line::default());
    }
}

fn push_current_line(lines: &mut Vec<Line<'static>>, current_spans: &mut Vec<Span<'static>>) {
    if !current_spans.is_empty() {
        lines.push(Line::from(std::mem::take(current_spans)));
    }
}

fn trim_blank_lines(lines: &mut Vec<Line<'static>>) {
    while matches!(lines.first(), Some(line) if line.spans.is_empty()) {
        lines.remove(0);
    }
    while matches!(lines.last(), Some(line) if line.spans.is_empty()) {
        lines.pop();
    }
}

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

fn flush_pending_text(
    pending_text: &mut String,
    current_spans: &mut Vec<Span<'static>>,
    style: Style,
) {
    if !pending_text.is_empty() {
        current_spans.push(Span::styled(std::mem::take(pending_text), style));
    }
}

pub(crate) fn render_markdown_ratatui(text: &str) -> Vec<Line<'static>> {
    let theme = current_theme();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(text, options);
    let mut output_lines = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_lang = String::new();
    let mut in_list = false;
    let mut list_level = 0;
    let mut list_counter: Vec<u64> = Vec::new();
    let mut in_blockquote = false;
    let mut blockquote_lines: Vec<String> = Vec::new();
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut in_bold = false;
    let mut in_italic = false;
    let mut in_strikethrough = false;
    let mut in_link = false;
    let mut link_text = String::new();
    let mut link_url = String::new();
    let mut pending_text = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_block_lang = match &kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code_block_content.clear();
                }
                Tag::Heading { level, .. } => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    push_current_line(&mut output_lines, &mut current_spans);
                    push_blank_line(&mut output_lines);
                    current_spans.push(Span::styled(
                        String::new(),
                        Style::default()
                            .fg(theme.fg.to_ratatui_color())
                            .add_modifier(Modifier::BOLD)
                            .bg(match level {
                                HeadingLevel::H1 => Color::Reset,
                                _ => Color::Reset,
                            }),
                    ));
                }
                Tag::Paragraph => {}
                Tag::List(start) => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    push_current_line(&mut output_lines, &mut current_spans);
                    push_blank_line(&mut output_lines);
                    in_list = true;
                    list_level += 1;
                    if let Some(n) = start {
                        while list_counter.len() < list_level {
                            list_counter.push(0);
                        }
                        list_counter[list_level - 1] = n;
                    } else {
                        while list_counter.len() < list_level {
                            list_counter.push(0);
                        }
                        list_counter[list_level - 1] = 0;
                    }
                }
                Tag::Item => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    push_current_line(&mut output_lines, &mut current_spans);

                    // We rely on ratatui wrapping, so we just add the marker at the beginning
                    // of the item. Any subsequent paragraphs inside the item will just start
                    // at the beginning of the line, which is acceptable in a simple terminal
                    // renderer.

                    let indent = "  ".repeat(list_level.saturating_sub(1));
                    let counter = list_counter.get(list_level - 1).copied().unwrap_or(0);
                    if counter > 0 {
                        list_counter[list_level - 1] = counter + 1;
                        let marker = format!("{}{}. ", indent, counter);
                        current_spans.push(Span::styled(
                            marker,
                            Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                        ));
                    } else {
                        let marker = format!("{}{} ", indent, BULLET);
                        current_spans.push(Span::styled(
                            marker,
                            Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                        ));
                    }
                }
                Tag::BlockQuote(_) => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    push_current_line(&mut output_lines, &mut current_spans);
                    push_blank_line(&mut output_lines);
                    in_blockquote = true;
                    blockquote_lines.clear();
                }
                Tag::Table(_) => {
                    in_table = true;
                    table_rows.clear();
                }
                Tag::TableHead => {}
                Tag::TableRow => {}
                Tag::TableCell => {
                    pending_text.clear();
                }
                Tag::Emphasis => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    in_italic = true;
                }
                Tag::Strong => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    in_bold = true;
                }
                Tag::Strikethrough => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    in_strikethrough = true;
                }
                Tag::Link { dest_url, .. } => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
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
                    let code_lines =
                        render_code_block(&code_block_content, &code_block_lang, theme, 80, false);
                    push_blank_line(&mut output_lines);
                    output_lines.extend(code_lines);
                    code_block_content.clear();
                    code_block_lang.clear();
                }
                TagEnd::Heading(_) => {
                    push_current_line(&mut output_lines, &mut current_spans);
                }
                TagEnd::Paragraph => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    push_current_line(&mut output_lines, &mut current_spans);
                }
                TagEnd::List(_) => {
                    in_list = false;
                    if list_level > 0 {
                        list_level -= 1;
                    }
                }
                TagEnd::BlockQuote(_) => {
                    in_blockquote = false;
                    for line in &blockquote_lines {
                        output_lines.push(Line::from(vec![
                            Span::styled(
                                format!("{} ", BLOCKQUOTE_BAR),
                                Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                            ),
                            Span::styled(
                                line.clone(),
                                Style::default().fg(theme.fg_dim.to_ratatui_color()),
                            ),
                        ]));
                    }
                    blockquote_lines.clear();
                }
                TagEnd::Table => {
                    in_table = false;
                    let table_lines = render_table_ratatui(&table_rows, theme);
                    push_blank_line(&mut output_lines);
                    output_lines.extend(table_lines);
                    table_rows.clear();
                }
                TagEnd::TableRow => {
                    if !current_row.is_empty() {
                        table_rows.push(current_row.clone());
                        current_row.clear();
                    }
                    pending_text.clear();
                }
                TagEnd::TableCell => {
                    current_row.push(pending_text.clone());
                    pending_text.clear();
                }
                TagEnd::Emphasis => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    in_italic = false;
                }
                TagEnd::Strong => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    in_bold = false;
                }
                TagEnd::Strikethrough => {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    in_strikethrough = false;
                }
                TagEnd::Link => {
                    in_link = false;
                    current_spans.push(Span::styled(
                        if link_text.is_empty() {
                            link_url.clone()
                        } else {
                            link_text.clone()
                        },
                        Style::default()
                            .fg(theme.accent_secondary.to_ratatui_color())
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    link_text.clear();
                    link_url.clear();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(&text);
                } else if in_link {
                    link_text.push_str(&text);
                } else if in_blockquote {
                    blockquote_lines.push(text.to_string());
                } else if in_table {
                    pending_text.push_str(&text);
                } else {
                    pending_text.push_str(&text);
                }
            }
            Event::Code(text) => {
                if in_code_block {
                    code_block_content.push_str(&text);
                } else {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
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
                    code_block_content.push('\n');
                } else {
                    pending_text.push(' ');
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                } else {
                    flush_pending_text(
                        &mut pending_text,
                        &mut current_spans,
                        get_current_style(in_bold, in_italic, in_strikethrough, theme),
                    );
                    push_current_line(&mut output_lines, &mut current_spans);
                }
            }
            Event::Rule => {
                flush_pending_text(
                    &mut pending_text,
                    &mut current_spans,
                    get_current_style(in_bold, in_italic, in_strikethrough, theme),
                );
                push_current_line(&mut output_lines, &mut current_spans);
                push_blank_line(&mut output_lines);
                output_lines.push(Line::from(vec![Span::styled(
                    "─".repeat(40),
                    Style::default().fg(theme.fg_dim.to_ratatui_color()),
                )]));
                push_blank_line(&mut output_lines);
            }
            Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            _ => {}
        }
    }

    flush_pending_text(
        &mut pending_text,
        &mut current_spans,
        get_current_style(in_bold, in_italic, in_strikethrough, theme),
    );
    push_current_line(&mut output_lines, &mut current_spans);

    if output_lines.is_empty() {
        output_lines.push(Line::from(vec![Span::styled(
            text.to_string(),
            Style::default().fg(theme.fg.to_ratatui_color()),
        )]));
    }

    trim_blank_lines(&mut output_lines);
    collapse_blank_runs(output_lines)
}

fn get_current_style(bold: bool, italic: bool, strikethrough: bool, theme: &Theme) -> Style {
    let mut style = Style::default().fg(theme.fg.to_ratatui_color());
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
            | AssistantBlock::Callout(text) => render_markdown_ratatui(text),
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
        i += 1;
        while i < lines.len()
            && !lines[i].trim().is_empty()
            && parse_fence_language(lines[i]).is_none()
            && !is_rule_line(lines[i].trim_end())
        {
            block_lines.push(lines[i].trim_end().to_string());
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

fn is_rule_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3
        && trimmed
            .chars()
            .all(|ch| ch == '-' || ch == '*' || ch == '_')
}

fn classify_markdown_block(text: &str) -> AssistantBlock {
    let first = text.lines().next().unwrap_or_default().trim_start();
    if first.starts_with("- ")
        || first.starts_with("* ")
        || first.starts_with("+ ")
        || first
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count()
            .gt(&0)
            && first.contains(". ")
    {
        AssistantBlock::List(text.to_string())
    } else if first.starts_with(">") {
        AssistantBlock::Callout(text.to_string())
    } else if text.lines().take(2).all(|line| line.contains('|')) {
        AssistantBlock::Table(text.to_string())
    } else {
        AssistantBlock::Paragraph(text.to_string())
    }
}

fn render_table_ratatui(rows: &[Vec<String>], theme: &Theme) -> Vec<Line<'static>> {
    if rows.is_empty() {
        return Vec::new();
    }

    let num_cols = rows[0].len();
    if num_cols == 0 {
        return Vec::new();
    }

    let mut col_widths = vec![0; num_cols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    let mut output = Vec::new();
    let border_style = Style::default().fg(theme.fg_dim.to_ratatui_color());
    let header_style = Style::default()
        .fg(theme.fg.to_ratatui_color())
        .add_modifier(Modifier::BOLD);

    for (row_idx, row) in rows.iter().enumerate() {
        let mut spans = Vec::new();
        spans.push(Span::styled("| ", border_style));

        for (col_idx, cell) in row.iter().enumerate() {
            let width = col_widths.get(col_idx).copied().unwrap_or(0);
            let padded = format!("{:<width$}", cell, width = width);
            let style = if row_idx == 0 {
                header_style
            } else {
                Style::default().fg(theme.fg.to_ratatui_color())
            };
            spans.push(Span::styled(padded, style));

            if col_idx < num_cols - 1 {
                spans.push(Span::styled(" | ", border_style));
            }
        }

        spans.push(Span::styled(" |", border_style));
        output.push(Line::from(spans));

        if row_idx == 0 {
            let mut separator = Vec::new();
            separator.push(Span::styled("|-", border_style));
            for (col_idx, width) in col_widths.iter().enumerate() {
                separator.push(Span::styled("-".repeat(*width), border_style));
                if col_idx < num_cols - 1 {
                    separator.push(Span::styled("-|-", border_style));
                } else {
                    separator.push(Span::styled("-|", border_style));
                }
            }
            output.push(Line::from(separator));
        }
    }

    output
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
}
