//! Screen definitions for the ralf TUI.

pub mod settings;
pub mod spec_studio;
pub mod status;

use crate::app::App;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

/// Trait for screens that can be rendered.
pub trait Screen {
    /// Render the screen to the buffer.
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer);
}

/// Render the help overlay.
pub fn render_help_overlay(area: Rect, buf: &mut Buffer) {
    use crate::ui::centered_fixed;
    use crate::ui::theme::Styles;
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let help_text = r"
  Navigation
    Tab / Shift+Tab   Next/prev section
    j/k or Up/Down    Scroll
    Enter             Select/confirm
    Esc               Back/cancel
    q                 Quit
    ?                 Toggle this help

  [Press any key to close]
";

    // Calculate overlay size
    let width = 50.min(area.width.saturating_sub(4));
    let height = 14.min(area.height.saturating_sub(4));
    let overlay_area = centered_fixed(width, height, area);

    // Clear the area
    Clear.render(overlay_area, buf);

    // Render the help block
    let block = Block::default()
        .title(" Help ")
        .title_style(Styles::title())
        .borders(Borders::ALL)
        .border_style(Styles::border_active())
        .style(Styles::default());

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .style(Styles::default());

    paragraph.render(overlay_area, buf);
}
