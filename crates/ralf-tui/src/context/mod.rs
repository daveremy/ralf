//! Context pane routing and views.
//!
//! This module provides:
//! - [`ContextView`] - View variants for the context pane
//! - [`CompletionKind`] - Done vs Abandoned completion states
//! - [`SpecPreview`] - Spec preview widget with markdown rendering

mod router;
mod spec_preview;

pub use router::{CompletionKind, ContextView};
pub use spec_preview::{SpecPhase, SpecPreview};
