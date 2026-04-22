//! @efficiency-role: ui-component
//!
//! Global Search Dialog — Modal search interface for project history and context.
//!
//! Uses ratatui popup for dynamic filtering and result display.

use crate::ui_theme::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use std::collections::VecDeque;

/// Search result item
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub content: String,
}

/// Global search modal state
#[derive(Clone, Debug)]
pub struct SearchModal {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub visible: bool,
}

impl SearchModal {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_index: 0,
            visible: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
        self.results.clear();
        self.selected_index = 0;
    }

    pub fn update_query(&mut self, query: String) {
        self.query = query;
        // TODO: Perform search and update results
        self.selected_index = 0;
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.results.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected_index)
    }

    /// Render the search modal as a ratatui widget
    pub fn render(&self, area: Rect, f: &mut Frame) {
        if !self.visible {
            return;
        }

        let theme = current_theme();

        // Create popup area
        let popup_area = centered_rect(60, 20, area);

        // Clear background
        f.render_widget(Clear, popup_area);

        // Main block
        let block = Block::default()
            .title("Global Search")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_primary.to_ratatui_color()));

        // Inner area
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        // Layout: query input at top, results below
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Query input
                Constraint::Min(1),    // Results
            ])
            .split(inner);

        // Query input
        let query_block = Block::default().title("Query").borders(Borders::ALL);
        let query_text = Paragraph::new(Line::from(vec![
            Span::styled(
                ">",
                Style::default().fg(theme.accent_primary.to_ratatui_color()),
            ),
            Span::raw(" "),
            Span::raw(&self.query),
        ]))
        .block(query_block);
        f.render_widget(query_text, chunks[0]);

        // Results list
        let result_items: Vec<ListItem> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let style = if i == self.selected_index {
                    Style::default()
                        .bg(theme.accent_primary.to_ratatui_color())
                        .fg(Color::Black)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(&result.file, style.fg(theme.fg.to_ratatui_color())),
                    Span::raw(":"),
                    Span::styled(
                        result.line.to_string(),
                        style.fg(theme.fg_dim.to_ratatui_color()),
                    ),
                    Span::raw(" "),
                    Span::styled(&result.content, style),
                ]))
                .style(style)
            })
            .collect();

        let results_list =
            List::new(result_items).block(Block::default().title("Results").borders(Borders::ALL));
        f.render_widget(results_list, chunks[1]);
    }
}

/// Helper to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
