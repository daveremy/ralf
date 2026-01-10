//! Text rendering utilities.
//!
//! This module provides shared text rendering functionality:
//! - [`render_markdown`] - Render markdown to styled ratatui Lines
//! - [`MarkdownStyles`] - Style configuration for markdown elements

mod markdown;
mod styles;

pub use markdown::render_markdown;
pub use styles::MarkdownStyles;
