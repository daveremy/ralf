//! ralf-tui: Terminal UI for multi-model autonomous loops
//!
//! This crate provides the TUI layer for ralf, including:
//! - Spec Studio screens for spec drafting
//! - Run Dashboard for loop monitoring
//! - Shared widgets (tabs, log viewers)

// Re-export engine for convenience
pub use ralf_engine;

/// Placeholder function to verify the crate builds correctly.
///
/// This will be replaced with actual TUI implementation.
pub fn tui_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Placeholder for running the TUI.
///
/// This will be replaced with actual TUI implementation.
pub fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    println!("TUI not yet implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_version() {
        let version = tui_version();
        assert!(!version.is_empty());
        assert!(version.starts_with("0."));
    }
}
