//! UI widgets for the TUI.
//!
//! This module provides:
//! - [`StatusBar`] - Top status bar with phase, title, model info
//! - [`FooterHints`] - Bottom keybinding hints
//! - [`Pane`] - Generic pane with border and optional title

mod footer_hints;
mod pane;
mod status_bar;

pub use footer_hints::{FooterHints, KeyHint};
pub use pane::Pane;
pub use status_bar::{StatusBar, StatusBarContent};
