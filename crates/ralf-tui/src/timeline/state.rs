//! Timeline state management.
//!
//! Handles event storage, selection, scrolling, and follow mode.

use super::event::{EventKind, TimelineEvent, COLLAPSED_HEIGHT};

/// Events scrolled per mouse wheel tick.
pub const SCROLL_SPEED: usize = 3;

/// Timeline pane state.
#[derive(Debug, Default)]
pub struct TimelineState {
    /// All events in chronological order.
    events: Vec<TimelineEvent>,
    /// Index of selected event (if any).
    selected: Option<usize>,
    /// Index of first visible event.
    scroll_offset: usize,
    /// Whether to auto-follow new events.
    follow: bool,
    /// Next event ID to assign.
    next_id: u64,
    /// Model name we're waiting for a response from (shows animated indicator).
    pending_response: Option<String>,
}

impl TimelineState {
    /// Create a new empty timeline state.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            selected: None,
            scroll_offset: 0,
            follow: true, // Start with follow enabled
            next_id: 1,
            pending_response: None,
        }
    }

    /// Get all events.
    pub fn events(&self) -> &[TimelineEvent] {
        &self.events
    }

    /// Get the currently selected event index.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Get the scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Check if follow mode is enabled.
    pub fn is_following(&self) -> bool {
        self.follow
    }

    /// Check if the timeline is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get the pending response model name (if waiting for a response).
    pub fn pending_response(&self) -> Option<&str> {
        self.pending_response.as_deref()
    }

    /// Set pending response state (shows animated indicator while waiting).
    pub fn set_pending(&mut self, model: impl Into<String>) {
        self.pending_response = Some(model.into());
    }

    /// Clear pending response state.
    pub fn clear_pending(&mut self) {
        self.pending_response = None;
    }

    /// Get the number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Add a new event to the timeline.
    ///
    /// If `follow` is true, auto-scrolls to show the new event.
    pub fn push(&mut self, kind: EventKind) {
        let event = TimelineEvent::new(self.next_id, kind);
        self.next_id += 1;
        self.events.push(event);

        // Auto-scroll if following
        if self.follow && !self.events.is_empty() {
            self.selected = Some(self.events.len() - 1);
        }
    }

    /// Add an event directly (for testing or restoring state).
    pub fn push_event(&mut self, event: TimelineEvent) {
        self.next_id = self.next_id.max(event.id + 1);
        self.events.push(event);

        if self.follow && !self.events.is_empty() {
            self.selected = Some(self.events.len() - 1);
        }
    }

    /// Clear all events from the timeline.
    pub fn clear(&mut self) {
        self.events.clear();
        self.selected = None;
        self.scroll_offset = 0;
        // Keep follow mode as-is
        // next_id not reset to avoid collisions if events are restored
    }

    /// Move selection up. Stops at first event (no wrap).
    /// Disables follow mode.
    pub fn select_prev(&mut self) {
        if self.events.is_empty() {
            return;
        }

        self.follow = false;

        match self.selected {
            Some(0) => {} // Already at top, do nothing
            Some(i) => self.selected = Some(i - 1),
            None => self.selected = Some(self.events.len().saturating_sub(1)),
        }
    }

    /// Move selection down. Stops at last event (no wrap).
    pub fn select_next(&mut self) {
        if self.events.is_empty() {
            return;
        }

        match self.selected {
            Some(i) if i >= self.events.len() - 1 => {} // Already at bottom
            Some(i) => self.selected = Some(i + 1),
            None => self.selected = Some(0),
        }
    }

    /// Jump to first event. Disables follow mode.
    pub fn jump_to_start(&mut self) {
        if self.events.is_empty() {
            return;
        }

        self.follow = false;
        self.selected = Some(0);
        self.scroll_offset = 0;
    }

    /// Jump to last event. Enables follow mode.
    pub fn jump_to_end(&mut self) {
        if self.events.is_empty() {
            return;
        }

        self.follow = true;
        self.selected = Some(self.events.len() - 1);
    }

    /// Move selection up by a page.
    pub fn page_up(&mut self, visible_count: usize) {
        if self.events.is_empty() {
            return;
        }

        self.follow = false;

        let page_size = visible_count.max(1);
        match self.selected {
            Some(i) => {
                self.selected = Some(i.saturating_sub(page_size));
            }
            None => self.selected = Some(0),
        }
    }

    /// Move selection down by a page.
    pub fn page_down(&mut self, visible_count: usize) {
        if self.events.is_empty() {
            return;
        }

        let page_size = visible_count.max(1);
        let max_idx = self.events.len().saturating_sub(1);

        match self.selected {
            Some(i) => {
                self.selected = Some((i + page_size).min(max_idx));
            }
            None => self.selected = Some(max_idx.min(page_size)),
        }
    }

    /// Scroll up by the given number of events.
    pub fn scroll_up(&mut self, amount: usize) {
        self.follow = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down by the given number of events.
    pub fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.events.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }

    /// Select a specific event by index.
    pub fn select(&mut self, index: usize) {
        if index < self.events.len() {
            self.selected = Some(index);
        }
    }

    /// Toggle collapse for selected event.
    pub fn toggle_collapse(&mut self) {
        if let Some(idx) = self.selected {
            if let Some(event) = self.events.get_mut(idx) {
                if event.is_collapsible() {
                    event.collapsed = !event.collapsed;
                }
            }
        }
    }

    /// Ensure selected event is visible, adjusting `scroll_offset` if needed.
    pub fn ensure_selection_visible(&mut self, visible_count: usize) {
        let Some(selected) = self.selected else {
            return;
        };

        if visible_count == 0 {
            return;
        }

        // If selected is before scroll_offset, scroll up
        if selected < self.scroll_offset {
            self.scroll_offset = selected;
        }

        // If selected is after visible area, scroll down
        let last_visible = self.scroll_offset + visible_count - 1;
        if selected > last_visible {
            self.scroll_offset = selected.saturating_sub(visible_count - 1);
        }
    }

    /// Calculate how many events fit in the given height.
    ///
    /// Uses fixed height: 2 lines for collapsed, up to 12 for expanded.
    pub fn events_per_page(&self, height: usize) -> usize {
        // Simplified: assume mostly collapsed events
        height / COLLAPSED_HEIGHT
    }

    /// Get the display height for an event.
    pub fn event_height(&self, index: usize) -> usize {
        self.events
            .get(index)
            .map_or(0, TimelineEvent::display_height)
    }

    /// Get visible events for current scroll position.
    ///
    /// Returns tuples of `(event_index, &event)`.
    pub fn visible_events(&self, visible_count: usize) -> Vec<(usize, &TimelineEvent)> {
        self.events
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_count)
            .collect()
    }

    /// Convert a y-coordinate (relative to timeline inner area) to an event index.
    ///
    /// Handles variable height events (expanded vs collapsed).
    /// Returns None if y is in the gap between events or past the last visible event.
    pub fn y_to_event_index(&self, y: usize) -> Option<usize> {
        if self.events.is_empty() {
            return None;
        }

        let mut current_y = 0usize;

        for idx in self.scroll_offset..self.events.len() {
            let event = &self.events[idx];
            let event_height = event.display_height();

            // Check if y falls within this event's display area [current_y, current_y + event_height)
            if y >= current_y && y < current_y + event_height {
                return Some(idx);
            }

            // Move past this event and its trailing gap (1 line between events)
            current_y += event_height + 1;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::event::SpecEvent;

    fn create_test_timeline(count: usize) -> TimelineState {
        let mut state = TimelineState::new();
        for i in 0..count {
            state.push(EventKind::Spec(SpecEvent::user(format!("Event {}", i + 1))));
        }
        state
    }

    #[test]
    fn test_new_timeline() {
        let state = TimelineState::new();
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
        assert!(state.is_following());
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn test_push_event() {
        let mut state = TimelineState::new();
        state.push(EventKind::Spec(SpecEvent::user("Hello")));

        assert_eq!(state.len(), 1);
        assert_eq!(state.selected(), Some(0)); // Auto-selected due to follow
    }

    #[test]
    fn test_select_prev() {
        let mut state = create_test_timeline(5);
        state.selected = Some(3);

        state.select_prev();
        assert_eq!(state.selected(), Some(2));
        assert!(!state.is_following()); // Follow disabled

        // At top, stays at 0
        state.selected = Some(0);
        state.select_prev();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_select_next() {
        let mut state = create_test_timeline(5);
        state.selected = Some(2);

        state.select_next();
        assert_eq!(state.selected(), Some(3));

        // At bottom, stays at last
        state.selected = Some(4);
        state.select_next();
        assert_eq!(state.selected(), Some(4));
    }

    #[test]
    fn test_jump_to_start() {
        let mut state = create_test_timeline(10);
        state.selected = Some(5);
        state.scroll_offset = 3;

        state.jump_to_start();
        assert_eq!(state.selected(), Some(0));
        assert_eq!(state.scroll_offset(), 0);
        assert!(!state.is_following());
    }

    #[test]
    fn test_jump_to_end() {
        let mut state = create_test_timeline(10);
        state.selected = Some(0);
        state.follow = false;

        state.jump_to_end();
        assert_eq!(state.selected(), Some(9));
        assert!(state.is_following());
    }

    #[test]
    fn test_page_up_down() {
        let mut state = create_test_timeline(20);
        state.selected = Some(10);

        state.page_up(5);
        assert_eq!(state.selected(), Some(5));

        state.page_down(5);
        assert_eq!(state.selected(), Some(10));

        // Page down at end clamps
        state.selected = Some(18);
        state.page_down(5);
        assert_eq!(state.selected(), Some(19));

        // Page up at start clamps
        state.selected = Some(2);
        state.page_up(5);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_scroll_up_down() {
        let mut state = create_test_timeline(20);
        state.scroll_offset = 10;

        state.scroll_up(3);
        assert_eq!(state.scroll_offset(), 7);
        assert!(!state.is_following());

        state.scroll_down(5);
        assert_eq!(state.scroll_offset(), 12);

        // Scroll up at top clamps to 0
        state.scroll_offset = 2;
        state.scroll_up(5);
        assert_eq!(state.scroll_offset(), 0);

        // Scroll down at bottom clamps
        state.scroll_offset = 18;
        state.scroll_down(5);
        assert_eq!(state.scroll_offset(), 19);
    }

    #[test]
    fn test_toggle_collapse() {
        let mut state = TimelineState::new();
        state.push(EventKind::Spec(SpecEvent::user("Line 1\nLine 2\nLine 3")));
        state.selected = Some(0);

        let initially_collapsed = state.events[0].collapsed;
        state.toggle_collapse();
        assert_ne!(state.events[0].collapsed, initially_collapsed);

        state.toggle_collapse();
        assert_eq!(state.events[0].collapsed, initially_collapsed);
    }

    #[test]
    fn test_ensure_selection_visible() {
        let mut state = create_test_timeline(20);
        state.scroll_offset = 5;
        state.selected = Some(15);

        state.ensure_selection_visible(5);
        // Selected (15) should now be visible
        assert!(state.scroll_offset() <= 15);
        assert!(state.scroll_offset() + 5 > 15);

        // Test scrolling up when selection is above viewport
        state.scroll_offset = 10;
        state.selected = Some(5);
        state.ensure_selection_visible(5);
        assert_eq!(state.scroll_offset(), 5);
    }

    #[test]
    fn test_visible_events() {
        let mut state = create_test_timeline(10);
        state.scroll_offset = 3;

        let visible = state.visible_events(4);
        assert_eq!(visible.len(), 4);
        assert_eq!(visible[0].0, 3); // First visible is index 3
        assert_eq!(visible[3].0, 6); // Last visible is index 6
    }

    #[test]
    fn test_y_to_event_index() {
        let mut state = create_test_timeline(10);
        state.scroll_offset = 2;

        // Event layout with collapsed events (2 lines each) + 1 line gap:
        // Event 2: y=[0,1]
        // Gap: y=2
        // Event 3: y=[3,4]
        // Gap: y=5
        // Event 4: y=[6,7]
        // etc.

        // y=0 -> first visible event (index 2)
        assert_eq!(state.y_to_event_index(0), Some(2));
        assert_eq!(state.y_to_event_index(1), Some(2));

        // y=2 -> gap between events (no selection)
        assert_eq!(state.y_to_event_index(2), None);

        // y=3 -> second visible event (index 3)
        assert_eq!(state.y_to_event_index(3), Some(3));
        assert_eq!(state.y_to_event_index(4), Some(3));

        // y=5 -> gap
        assert_eq!(state.y_to_event_index(5), None);

        // y=6 -> third visible event (index 4)
        assert_eq!(state.y_to_event_index(6), Some(4));

        // y way out of range
        assert_eq!(state.y_to_event_index(100), None);
    }

    #[test]
    fn test_events_per_page() {
        let state = TimelineState::new();
        // 20 lines of height, 2 lines per event
        assert_eq!(state.events_per_page(20), 10);
    }
}
