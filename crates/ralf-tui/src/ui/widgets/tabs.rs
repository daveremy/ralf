//! Tab bar widget.

use crate::ui::theme::Styles;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Widget},
};

/// A horizontal tab bar widget.
#[derive(Debug, Clone)]
pub struct Tabs<'a> {
    titles: Vec<&'a str>,
    selected: usize,
    block: Option<Block<'a>>,
}

impl<'a> Tabs<'a> {
    /// Create a new tabs widget.
    pub fn new(titles: Vec<&'a str>) -> Self {
        Self {
            titles,
            selected: 0,
            block: None,
        }
    }

    /// Set the selected tab index.
    #[must_use]
    pub fn select(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Set the block to wrap the tabs.
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl Widget for Tabs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = match self.block {
            Some(b) => {
                let inner = b.inner(area);
                b.render(area, buf);
                inner
            }
            None => area,
        };

        if area.height < 1 {
            return;
        }

        let mut spans = Vec::new();
        for (i, title) in self.titles.iter().enumerate() {
            let is_selected = i == self.selected;

            // Add separator if not first
            if i > 0 {
                spans.push(Span::styled(" | ", Styles::dim()));
            }

            // Tab number hint
            spans.push(Span::styled(
                format!("[{}] ", i + 1),
                if is_selected {
                    Styles::highlight()
                } else {
                    Styles::dim()
                },
            ));

            // Tab title
            if is_selected {
                spans.push(Span::styled(*title, Styles::highlight()));
            } else {
                spans.push(Span::styled(*title, Styles::default()));
            }
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

/// Render a simple tab indicator bar (for status bar).
pub fn tab_indicator(titles: &[&str], selected: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, title) in titles.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        if i == selected {
            spans.push(Span::styled(
                format!("[{}]", title),
                Styles::highlight(),
            ));
        } else {
            spans.push(Span::styled(format!(" {} ", title), Styles::dim()));
        }
    }
    spans
}
