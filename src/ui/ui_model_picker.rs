//! @efficiency-role: ui-component
//!
//! Model Picker — Interactive model selection overlay.
//!
//! Displays available models from config/profiles.toml with performance metrics.

use crate::ui_theme::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Model info
#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub name: String,
    pub base_url: String,
    pub max_tokens: u64,
    pub temperature: f64,
}

/// Model picker modal state
#[derive(Clone, Debug)]
pub struct ModelPicker {
    pub models: Vec<ModelInfo>,
    pub selected_index: usize,
    pub visible: bool,
}

impl ModelPicker {
    pub fn new() -> Self {
        // TODO: Load from config/profiles.toml
        let models = vec![
            ModelInfo {
                name: "Nanbeige-4.1-3B-Q6_K".to_string(),
                base_url: "http://192.168.1.186:8080".to_string(),
                max_tokens: 16384,
                temperature: 0.2,
            },
            ModelInfo {
                name: "gemma-3-12b-it".to_string(),
                base_url: "http://192.168.1.186:8080".to_string(),
                max_tokens: 8192,
                temperature: 0.2,
            },
        ];
        Self {
            models,
            selected_index: 0,
            visible: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.selected_index = 0;
    }

    pub fn select_next(&mut self) {
        if !self.models.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.models.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.models.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.models.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn selected_model(&self) -> Option<&ModelInfo> {
        self.models.get(self.selected_index)
    }

    /// Render the model picker as a ratatui widget
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
            .title("Select Model")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_primary.to_ratatui_color()));

        // Inner area
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        // Layout: models list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1)])
            .split(inner);

        // Models list
        let model_items: Vec<ListItem> = self
            .models
            .iter()
            .enumerate()
            .map(|(i, model)| {
                let style = if i == self.selected_index {
                    Style::default()
                        .bg(theme.accent_primary.to_ratatui_color())
                        .fg(Color::Black)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(&model.name, style.fg(theme.fg.to_ratatui_color())),
                    Span::raw(" ("),
                    Span::styled(format!("{} tokens", model.max_tokens), style.fg(theme.fg_dim.to_ratatui_color())),
                    Span::raw(", temp "),
                    Span::styled(format!("{:.1}", model.temperature), style.fg(theme.fg_dim.to_ratatui_color())),
                    Span::raw(")"),
                ]))
                .style(style)
            })
            .collect();

        let models_list = List::new(model_items)
            .block(Block::default().title("Available Models").borders(Borders::ALL));
        f.render_widget(models_list, chunks[0]);
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