//! Layout components for the TUI.
//!
//! This module provides:
//! - [`render_shell`] - Main shell layout renderer
//! - [`ScreenMode`] - Split, `TimelineFocus`, `ContextFocus` modes
//! - [`FocusedPane`] - Which pane has keyboard focus

mod screen_modes;
mod shell;

pub use screen_modes::{FocusedPane, ScreenMode};
pub use shell::{render_shell, MIN_HEIGHT, MIN_WIDTH};
