//! UI widgets for the TUI.
//!
//! This module provides:
//! - [`StatusBar`] - Top status bar with phase, title, model info
//! - [`FooterHints`] - Bottom keybinding hints
//! - [`Pane`] - Generic pane with border and optional title
//! - [`ModelsPanel`] - Models panel showing model status
//! - [`InputBar`] - Full-width input bar for text entry

mod footer_hints;
mod input_bar;
mod models_panel;
mod pane;
mod status_bar;

pub use footer_hints::{hints_for_state, FooterHints, KeyHint};
pub use input_bar::InputBar;
pub use models_panel::ModelsPanel;
pub use pane::Pane;
pub use status_bar::{StatusBar, StatusBarContent};
