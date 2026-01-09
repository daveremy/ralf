//! Context pane routing and views.
//!
//! This module provides:
//! - [`ContextView`] - View variants for the context pane
//! - [`CompletionKind`] - Done vs Abandoned completion states

mod router;

pub use router::{CompletionKind, ContextView};
