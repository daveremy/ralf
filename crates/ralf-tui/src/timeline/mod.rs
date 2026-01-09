//! Timeline module for displaying thread event history.
//!
//! This module provides:
//! - [`TimelineEvent`] - Event data model with 4 types (Spec, Run, Review, System)
//! - [`TimelineState`] - State management for events, selection, and scrolling
//! - [`TimelineWidget`] - Widget for rendering the timeline pane

mod event;
mod state;
mod widget;

pub use event::{
    EventKind, ReviewEvent, ReviewResult, RunEvent, SpecEvent, SystemEvent, SystemLevel,
    TimelineEvent, COLLAPSED_HEIGHT, MAX_EXPANDED_LINES,
};
pub use state::{TimelineState, SCROLL_SPEED};
pub use widget::TimelineWidget;
