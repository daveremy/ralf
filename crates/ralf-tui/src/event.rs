//! Event handling for the ralf TUI.

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Events that can occur in the TUI.
#[derive(Debug, Clone)]
pub enum Event {
    /// A key was pressed.
    Key(KeyEvent),
    /// A mouse event occurred.
    Mouse(MouseEvent),
    /// A tick event for UI updates.
    Tick,
    /// Terminal was resized.
    Resize(u16, u16),
}

/// Event handler that runs in a background task.
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate.
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        // Spawn blocking thread for event polling (crossterm uses blocking I/O)
        std::thread::spawn(move || {
            let tick_rate = Duration::from_millis(tick_rate_ms);
            loop {
                // Poll for events with timeout
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(evt) = event::read() {
                        let event = match evt {
                            CrosstermEvent::Key(key) => Some(Event::Key(key)),
                            CrosstermEvent::Mouse(mouse) => Some(Event::Mouse(mouse)),
                            CrosstermEvent::Resize(w, h) => Some(Event::Resize(w, h)),
                            _ => None,
                        };
                        if let Some(e) = event {
                            if tx_clone.send(e).is_err() {
                                break;
                            }
                        }
                    }
                } else {
                    // No event, send tick
                    if tx_clone.send(Event::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Get the next event, blocking until one is available.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

/// Key action that can be performed in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Help,
    Setup,
    Chat,
    Run,
    Cancel,
    Finalize,
    Export,
    Back,
    Select,
    Up,
    Down,
    Left,
    Right,
    NextTab,
    PrevTab,
    Tab(usize),
    Retry,
    Disable,
    ToggleFollow,
    None,
}

/// Convert a key event to an action based on context.
pub fn key_to_action(key: KeyEvent) -> Action {
    // Check for Ctrl+C first
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Action::Quit;
    }

    // Ctrl+F for finalize
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f') {
        return Action::Finalize;
    }

    // Ctrl+E for export transcript
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
        return Action::Export;
    }

    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('?') => Action::Help,
        KeyCode::Char('s') => Action::Setup,
        KeyCode::Char('c') => Action::Chat,
        KeyCode::Char('r') => Action::Run, // Also used as Retry in Setup context
        KeyCode::Char('d') => Action::Disable,
        KeyCode::Char('f') => Action::ToggleFollow, // Toggle output follow mode
        KeyCode::Esc => Action::Back,
        KeyCode::Enter => Action::Select,
        KeyCode::Up | KeyCode::Char('k') => Action::Up,
        KeyCode::Down | KeyCode::Char('j') => Action::Down,
        KeyCode::Left | KeyCode::Char('h') => Action::Left,
        KeyCode::Right | KeyCode::Char('l') => Action::Right,
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Action::PrevTab
            } else {
                Action::NextTab
            }
        }
        KeyCode::Char('1') => Action::Tab(0),
        KeyCode::Char('2') => Action::Tab(1),
        KeyCode::Char('3') => Action::Tab(2),
        KeyCode::Char('4') => Action::Tab(3),
        _ => Action::None,
    }
}
