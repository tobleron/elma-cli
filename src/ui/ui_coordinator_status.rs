//! @efficiency-role: ui-component
//!
//! Task U003: Coordinator Agent Status
//!
//! Provides a persistent, high-frequency status indicator
//! showing the orchestrator's current activity.

use crate::ui::ui_theme::*;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub(crate) struct CoordinatorStatus {
    pub(crate) task_description: String,
    pub(crate) is_active: bool,
}

impl CoordinatorStatus {
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.is_active {
            return;
        }

        let block = Block::default()
            .borders(Borders::NONE);
        
        let label = format!(" 󰒲  {} ", self.task_description);
        let styled_label = elma_accent(&label);

        Paragraph::new(styled_label)
            .block(block)
            .render(area, buf);
    }
}
