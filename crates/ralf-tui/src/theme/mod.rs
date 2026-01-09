//! Theme components for the TUI.
//!
//! This module provides:
//! - [`Theme`] - Color palette (Catppuccin Mocha/Latte/High Contrast)
//! - [`IconSet`] - Icons with Nerd/Unicode/ASCII modes
//! - [`BorderSet`] - Border characters with Unicode/ASCII fallback

mod borders;
mod colors;
mod icons;

pub use borders::BorderSet;
pub use colors::Theme;
pub use icons::{IconMode, IconSet};
