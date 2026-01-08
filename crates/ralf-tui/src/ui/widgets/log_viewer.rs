//! Log viewer widget with scrolling.
#![allow(dead_code)]

use crate::ui::theme::Styles;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Text},
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

/// A scrollable log viewer widget.
#[derive(Debug, Clone)]
pub struct LogViewer<'a> {
    lines: Vec<Line<'a>>,
    scroll: usize,
    auto_scroll: bool,
    block: Option<Block<'a>>,
}

impl<'a> LogViewer<'a> {
    /// Create a new log viewer.
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            scroll: 0,
            auto_scroll: true,
            block: None,
        }
    }

    /// Set the lines to display.
    #[must_use]
    pub fn lines(mut self, lines: Vec<Line<'a>>) -> Self {
        self.lines = lines;
        self
    }

    /// Set the scroll offset.
    #[must_use]
    pub fn scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    /// Enable or disable auto-scroll.
    #[must_use]
    pub fn auto_scroll(mut self, enabled: bool) -> Self {
        self.auto_scroll = enabled;
        self
    }

    /// Set the block to wrap the viewer.
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Get the scroll state for external tracking.
    pub fn scroll_state(&self, viewport_height: usize) -> ScrollState {
        let total = self.lines.len();
        let max_scroll = total.saturating_sub(viewport_height);
        let scroll = if self.auto_scroll {
            max_scroll
        } else {
            self.scroll.min(max_scroll)
        };
        ScrollState {
            total,
            viewport: viewport_height,
            offset: scroll,
        }
    }
}

impl Default for LogViewer<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for LogViewer<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = match &self.block {
            Some(b) => {
                let inner = b.inner(area);
                b.clone().render(area, buf);
                inner
            }
            None => area,
        };

        if area.height < 1 || area.width < 1 {
            return;
        }

        let viewport_height = area.height as usize;
        let state = self.scroll_state(viewport_height);

        // Create paragraph with scroll
        let text = Text::from(self.lines.clone());
        #[allow(clippy::cast_possible_truncation)]
        let scroll_offset = state.offset as u16;
        let paragraph = Paragraph::new(text)
            .style(Styles::default())
            .scroll((scroll_offset, 0));

        paragraph.render(area, buf);

        // Render scrollbar if content exceeds viewport
        if state.total > state.viewport {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state = ScrollbarState::new(state.total).position(state.offset);

            // Render scrollbar in the right margin
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y,
                width: 1,
                height: area.height,
            };
            scrollbar.render(scrollbar_area, buf, &mut scrollbar_state);
        }
    }
}

/// Scroll state for tracking position.
#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    pub total: usize,
    pub viewport: usize,
    pub offset: usize,
}

impl ScrollState {
    /// Check if scrolled to bottom.
    pub fn at_bottom(&self) -> bool {
        self.offset + self.viewport >= self.total
    }

    /// Calculate scroll offset for moving up.
    pub fn scroll_up(&self, amount: usize) -> usize {
        self.offset.saturating_sub(amount)
    }

    /// Calculate scroll offset for moving down.
    pub fn scroll_down(&self, amount: usize) -> usize {
        let max = self.total.saturating_sub(self.viewport);
        (self.offset + amount).min(max)
    }
}
