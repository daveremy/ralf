//! Conversation pane module.
//!
//! The conversation pane is the left side of the TUI, containing:
//! - Timeline events (scrollable history)
//! - Input area (phase-aware, always present)
//!
//! This implements the "Conversation" half of the Conversation + Artifact architecture.

mod placeholder;
mod widget;

pub use placeholder::input_placeholder;
pub use widget::ConversationPane;
