//! Context pane routing and views.
//!
//! This module provides:
//! - [`ContextView`] - View variants for the context pane
//! - [`CompletionKind`] - Done vs Abandoned completion states
//! - [`SpecPreview`] - Spec preview widget with markdown rendering

mod markdown;
mod router;
mod spec_preview;

pub use markdown::{parse_inline, parse_markdown, InlineSegment, MarkdownBlock};
pub use router::{CompletionKind, ContextView};
pub use spec_preview::{SpecPhase, SpecPreview};
