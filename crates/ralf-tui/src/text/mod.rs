//! Text rendering utilities.
//!
//! This module provides shared text rendering functionality:
//! - [`render_markdown`] - Render markdown to styled ratatui Lines
//! - [`MarkdownStyles`] - Style configuration for markdown elements
//! - [`wrap_text`], [`wrap_lines`] - Text wrapping utilities
//! - [`visual_width`], [`truncate_to_width`] - Unicode-aware width utilities

mod markdown;
mod styles;
mod width;
mod wrap;

pub use markdown::render_markdown;
pub use styles::MarkdownStyles;
pub use width::{truncate_to_width, visual_width};
pub use wrap::{wrap_lines, wrap_text};
