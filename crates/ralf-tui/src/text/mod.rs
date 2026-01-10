//! Text rendering utilities.
//!
//! This module provides shared text rendering functionality:
//! - [`render_markdown`] - Render markdown to styled ratatui Lines
//! - [`MarkdownStyles`] - Style configuration for markdown elements
//! - [`wrap_text`], [`wrap_lines`] - Text wrapping utilities

mod markdown;
mod styles;
mod wrap;

pub use markdown::render_markdown;
pub use styles::MarkdownStyles;
pub use wrap::{wrap_lines, wrap_text};
