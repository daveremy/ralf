//! Application state and update logic for the ralf TUI.

use crate::event::Action;
use crate::ui::widgets::TextInputState;
use ralf_engine::{
    discover_models, draft_has_promise, get_git_info, parse_criteria, save_draft_snapshot,
    ChatMessage, Config, GitInfo, ModelConfig, ModelInfo, ProbeResult, RunConfig, RunEvent,
    RunHandle, Thread,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

/// Maximum number of events to keep in the event log.
const MAX_EVENTS: usize = 100;

/// The current screen being displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Welcome,
    Setup,
    SpecStudio,
    FinalizeConfirm,
    FinalizeError,
    RunDashboard,
}

/// Status of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunStatus {
    /// No run has started yet.
    #[default]
    Idle,
    /// Run is currently active.
    Running,
    /// Run completed successfully.
    Completed,
    /// Run was cancelled by user.
    Cancelled,
    /// Run failed with an error.
    Failed,
}

/// State for an active or completed run.
#[derive(Debug, Default)]
pub struct RunState {
    /// Current status of the run.
    pub status: RunStatus,
    /// Run ID.
    pub run_id: Option<String>,
    /// Current iteration number.
    pub current_iteration: usize,
    /// Maximum iterations (0 = unlimited).
    pub max_iterations: usize,
    /// Currently selected/running model.
    pub current_model: Option<String>,
    /// When the run started.
    pub started_at: Option<Instant>,
    /// Model output (preview).
    pub model_output: String,
    /// Verifier results: (name, passed, duration_ms).
    pub verifier_results: Vec<(String, bool, u64)>,
    /// Active cooldowns: (model, remaining_secs).
    pub cooldowns: Vec<(String, u64)>,
    /// Event log messages (bounded to MAX_EVENTS).
    pub events: VecDeque<String>,
    /// Scroll offset for output.
    pub output_scroll: usize,
    /// Whether to auto-follow output (scroll to bottom on new content).
    pub follow_output: bool,
    /// Whether a cancel has been requested (prevents spamming).
    pub cancel_requested: bool,
    /// Completion reason (if completed).
    pub completion_reason: Option<String>,
    /// Error message (if failed).
    pub error_message: Option<String>,
    /// Parsed completion criteria from PROMPT.md.
    pub criteria: Vec<String>,
}

impl RunState {
    /// Push an event to the log, removing the oldest if at capacity.
    pub fn push_event(&mut self, event: String) {
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

/// Model status after probing.
#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub info: ModelInfo,
    pub probe_result: Option<ProbeResult>,
    pub probing: bool,
    pub probe_in_flight: bool,
    pub enabled: bool,
}

/// Application state.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct App {
    /// Whether the app should quit.
    pub should_quit: bool,

    /// Whether the help overlay is visible.
    pub show_help: bool,

    /// Current screen.
    pub screen: Screen,

    /// Repository path.
    pub repo_path: PathBuf,

    /// Git information.
    pub git_info: GitInfo,

    /// Whether config exists.
    pub config_exists: bool,

    /// Existing config (if any).
    pub config: Option<Config>,

    /// Detected models and their status.
    pub models: Vec<ModelStatus>,

    /// Currently selected model index (for setup screen).
    pub selected_model: usize,

    /// Tick counter for animations.
    pub tick: usize,

    /// Model selection strategy (true = round-robin, false = priority).
    pub round_robin: bool,

    /// Promise tag for config.
    pub promise_tag: String,

    /// Notification message (displayed temporarily, cleared after some ticks).
    pub notification: Option<String>,

    /// Ticks remaining until notification is cleared.
    notification_ttl: usize,

    // === Spec Studio state ===
    /// Current conversation thread.
    pub thread: Thread,

    /// Text input state for the chat input.
    pub input_state: TextInputState,

    /// Index of the currently selected model for chat.
    pub chat_model_index: usize,

    /// Whether a chat request is in progress.
    pub chat_in_progress: bool,

    /// Scroll offset for the transcript pane.
    pub transcript_scroll: usize,

    /// Scroll offset for the draft pane.
    pub draft_scroll: usize,

    // === Run Dashboard state ===
    /// State for the current or last run.
    pub run_state: RunState,

    /// Handle for cancelling the run (if active).
    pub run_handle: Option<RunHandle>,

    /// Channel receiver for run events.
    pub run_event_rx: Option<mpsc::UnboundedReceiver<RunEvent>>,

    /// Channel receiver for background git info updates.
    git_info_rx: Option<oneshot::Receiver<GitInfo>>,
}

impl App {
    /// Create a new app instance.
    pub fn new(repo_path: PathBuf) -> Self {
        let git_info = get_git_info();
        let config_path = repo_path.join(".ralf").join("config.json");
        let config_exists = config_path.exists();
        let config = Config::load(&config_path).ok();

        // Discover available models
        let discovered = discover_models();
        let models: Vec<ModelStatus> = discovered
            .models
            .into_iter()
            .filter(|m| m.callable)
            .map(|info| ModelStatus {
                info,
                probe_result: None,
                probing: false,
                probe_in_flight: false,
                enabled: true,
            })
            .collect();

        // Start on Setup if no config exists, otherwise Welcome
        let initial_screen = if config_exists {
            Screen::Welcome
        } else {
            Screen::Setup
        };

        Self {
            should_quit: false,
            show_help: false,
            screen: initial_screen,
            repo_path,
            git_info,
            config_exists,
            config,
            models,
            selected_model: 0,
            tick: 0,
            round_robin: true,
            promise_tag: "COMPLETE".to_string(),
            notification: None,
            notification_ttl: 0,
            // Spec Studio state
            thread: Thread::new(),
            input_state: TextInputState::new(),
            chat_model_index: 0,
            chat_in_progress: false,
            transcript_scroll: 0,
            draft_scroll: 0,
            // Run Dashboard state
            run_state: RunState::default(),
            run_handle: None,
            run_event_rx: None,
            git_info_rx: None,
        }
    }

    /// Handle an action.
    pub fn handle_action(&mut self, action: Action) {
        // Global actions
        match action {
            Action::Quit => {
                if self.show_help {
                    self.show_help = false;
                } else if self.screen == Screen::RunDashboard {
                    // On RunDashboard, Ctrl+C cancels if running, otherwise goes back
                    if self.run_state.status == RunStatus::Running {
                        self.request_cancel_run();
                    } else {
                        self.screen = Screen::Welcome;
                    }
                } else {
                    // Save thread if we're in SpecStudio before quitting
                    if self.screen == Screen::SpecStudio {
                        let spec_dir = self.repo_path.join(".ralf").join("spec");
                        let _ = self.thread.save(&spec_dir);
                    }
                    self.should_quit = true;
                }
                return;
            }
            Action::Help => {
                self.show_help = !self.show_help;
                return;
            }
            _ => {}
        }

        // If help is showing, any key closes it
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Screen-specific actions
        match self.screen {
            Screen::Welcome => self.handle_welcome_action(action),
            Screen::Setup => self.handle_setup_action(action),
            Screen::SpecStudio => self.handle_spec_studio_action(action),
            Screen::FinalizeConfirm => self.handle_finalize_confirm_action(action),
            Screen::FinalizeError => self.handle_finalize_error_action(action),
            Screen::RunDashboard => self.handle_run_dashboard_action(action),
        }
    }

    fn handle_welcome_action(&mut self, action: Action) {
        match action {
            Action::Setup => {
                self.screen = Screen::Setup;
                self.start_probing();
            }
            Action::Chat => {
                // Only allow chat if config exists
                if self.config_exists {
                    self.screen = Screen::SpecStudio;
                }
            }
            Action::Run => {
                // Only allow run if config exists and PROMPT.md exists
                let prompt_path = self.repo_path.join("PROMPT.md");
                if self.config_exists && prompt_path.exists() {
                    // Criteria will be loaded when run starts via start_run()
                    self.screen = Screen::RunDashboard;
                }
            }
            _ => {}
        }
    }

    fn handle_setup_action(&mut self, action: Action) {
        match action {
            Action::Back => {
                self.screen = Screen::Welcome;
            }
            Action::Up => {
                if self.selected_model > 0 {
                    self.selected_model -= 1;
                }
            }
            Action::Down => {
                if self.selected_model + 1 < self.models.len() {
                    self.selected_model += 1;
                }
            }
            Action::Disable => {
                if let Some(model) = self.models.get_mut(self.selected_model) {
                    model.enabled = !model.enabled;
                }
            }
            Action::Run => {
                // In setup context, 'r' means retry probing
                self.start_probing();
            }
            Action::Left | Action::Right => {
                self.round_robin = !self.round_robin;
            }
            Action::Select => {
                self.save_config();
            }
            _ => {}
        }
    }

    /// Start probing all models.
    pub fn start_probing(&mut self) {
        for model in &mut self.models {
            model.probing = true;
            model.probe_in_flight = false;
            model.probe_result = None;
        }
    }

    /// Update a model's probe result.
    pub fn update_probe_result(&mut self, model_name: &str, result: ProbeResult) {
        if let Some(model) = self.models.iter_mut().find(|m| m.info.name == model_name) {
            model.probe_result = Some(result);
            model.probing = false;
            model.probe_in_flight = false;
        }
    }

    /// Mark a model probe as complete (with error).
    pub fn mark_probe_error(&mut self, model_name: &str, error: &str) {
        if let Some(model) = self.models.iter_mut().find(|m| m.info.name == model_name) {
            model.probe_result = Some(ProbeResult {
                name: model_name.to_string(),
                success: false,
                response_time_ms: None,
                needs_auth: false,
                issues: vec![error.to_string()],
                suggestions: vec![],
            });
            model.probing = false;
        }
    }

    /// Check if any models are still probing.
    pub fn is_probing(&self) -> bool {
        self.models.iter().any(|m| m.probing)
    }

    /// Get model names that need probing (not already in-flight).
    pub fn models_to_probe(&self) -> Vec<String> {
        self.models
            .iter()
            .filter(|m| m.probing && !m.probe_in_flight && m.probe_result.is_none())
            .map(|m| m.info.name.clone())
            .collect()
    }

    /// Mark a model probe as started (in-flight).
    pub fn mark_probe_started(&mut self, model_name: &str) {
        if let Some(model) = self.models.iter_mut().find(|m| m.info.name == model_name) {
            model.probe_in_flight = true;
        }
    }

    /// Save the configuration and update app state.
    ///
    /// Note: This performs blocking file I/O, but config files are small (<1KB)
    /// so the brief block is acceptable. Using spawn_blocking would require
    /// async/channel complexity that isn't worth it for this use case.
    pub fn save_config(&mut self) {
        let enabled_model_names: Vec<String> = self
            .models
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.info.name.clone())
            .collect();

        if enabled_model_names.is_empty() {
            return;
        }

        let selection = if self.round_robin {
            ralf_engine::ModelSelection::RoundRobin
        } else {
            ralf_engine::ModelSelection::Priority
        };

        let config = Config {
            model_priority: enabled_model_names.clone(),
            models: enabled_model_names
                .iter()
                .map(|n| ModelConfig::default_for(n))
                .collect(),
            model_selection: selection,
            verifiers: vec![ralf_engine::VerifierConfig::default_tests()],
            completion_promise: self.promise_tag.clone(),
            ..Default::default()
        };

        let config_path = self.repo_path.join(".ralf").join("config.json");
        match config.save(&config_path) {
            Ok(()) => {
                self.config_exists = true;
                self.config = Some(config);
                self.set_notification("Config saved successfully".to_string());
                // Transition to Welcome screen after successful save
                self.screen = Screen::Welcome;
            }
            Err(e) => {
                self.set_notification(format!("Failed to save config: {e}"));
            }
        }
    }

    /// Set a temporary notification message.
    fn set_notification(&mut self, msg: String) {
        self.notification = Some(msg);
        // Display for ~3 seconds at 4 Hz tick rate (250ms) = 12 ticks
        self.notification_ttl = 12;
    }

    /// Increment tick counter and update time-based state.
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);

        // Clear notification after TTL expires
        if self.notification_ttl > 0 {
            self.notification_ttl -= 1;
            if self.notification_ttl == 0 {
                self.notification = None;
            }
        }

        // Check for background git info updates
        if let Some(rx) = &mut self.git_info_rx {
            match rx.try_recv() {
                Ok(info) => {
                    self.git_info = info;
                    self.git_info_rx = None;
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    // Sender dropped without sending, clear receiver
                    self.git_info_rx = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    // Still waiting, continue
                }
            }
        }
    }

    /// Spawn a background task to refresh git info.
    /// The result will be picked up in the next tick.
    fn spawn_git_info_update(&mut self) {
        // Don't spawn if one is already in progress
        if self.git_info_rx.is_some() {
            return;
        }

        let (tx, rx) = oneshot::channel();
        self.git_info_rx = Some(rx);

        // Spawn blocking task to run git commands without blocking the UI
        tokio::task::spawn_blocking(move || {
            let info = get_git_info();
            // Ignore error if receiver was dropped (app quit)
            let _ = tx.send(info);
        });
    }

    // === Spec Studio handlers ===

    fn handle_spec_studio_action(&mut self, action: Action) {
        match action {
            Action::Back => {
                // Save thread before leaving
                let spec_dir = self.repo_path.join(".ralf").join("spec");
                let _ = self.thread.save(&spec_dir);
                self.screen = Screen::Welcome;
            }
            Action::Finalize => {
                self.try_finalize();
            }
            Action::Export => {
                self.export_transcript();
            }
            Action::NextTab => {
                // Cycle through available models
                if !self.models.is_empty() {
                    self.chat_model_index = (self.chat_model_index + 1) % self.models.len();
                }
            }
            Action::Up => {
                // Scroll transcript up
                if self.transcript_scroll > 0 {
                    self.transcript_scroll -= 1;
                }
            }
            Action::Down => {
                // Scroll transcript down
                self.transcript_scroll += 1;
            }
            _ => {}
        }
    }

    fn handle_finalize_confirm_action(&mut self, action: Action) {
        match action {
            Action::Select => {
                // Confirm finalize - write PROMPT.md
                self.do_finalize();
            }
            Action::Back => {
                self.screen = Screen::SpecStudio;
            }
            _ => {}
        }
    }

    fn handle_finalize_error_action(&mut self, action: Action) {
        // Any key returns to spec studio
        if action == Action::Select || action == Action::Back {
            self.screen = Screen::SpecStudio;
        }
    }

    /// Attempt to finalize the specification.
    fn try_finalize(&mut self) {
        if draft_has_promise(&self.thread.draft) {
            self.screen = Screen::FinalizeConfirm;
        } else {
            self.screen = Screen::FinalizeError;
        }
    }

    /// Actually write the PROMPT.md file.
    fn do_finalize(&mut self) {
        let prompt_path = self.repo_path.join("PROMPT.md");
        match std::fs::write(&prompt_path, &self.thread.draft) {
            Ok(()) => {
                // Save final draft snapshot
                let spec_dir = self.repo_path.join(".ralf").join("spec");
                let _ = save_draft_snapshot(&spec_dir, &self.thread.draft);
                let _ = self.thread.save(&spec_dir);

                self.set_notification("PROMPT.md saved successfully".to_string());
                self.screen = Screen::Welcome;
            }
            Err(e) => {
                self.set_notification(format!("Failed to save PROMPT.md: {e}"));
                self.screen = Screen::SpecStudio;
            }
        }
    }

    /// Add a user message to the current thread.
    pub fn add_user_message(&mut self, content: String) {
        self.thread.add_message(ChatMessage::user(content));
        // Auto-scroll to show new message
        self.scroll_transcript_to_bottom();
    }

    /// Add an assistant message to the current thread.
    pub fn add_assistant_message(&mut self, content: String, model: String) {
        // Auto-update draft if response contains a promise tag or looks like a spec
        // This allows the model to "propose" a draft that the user can then finalize
        if draft_has_promise(&content) || content.starts_with('#') {
            self.thread.draft = content.clone();
        }

        self.thread
            .add_message(ChatMessage::assistant(content, model));
        // Auto-save thread after each response
        let spec_dir = self.repo_path.join(".ralf").join("spec");
        let _ = self.thread.save(&spec_dir);
        // Auto-scroll to show new message
        self.scroll_transcript_to_bottom();
    }

    /// Scroll transcript to show the latest messages.
    fn scroll_transcript_to_bottom(&mut self) {
        // Estimate lines needed (rough: 3 lines per message on average)
        let estimated_lines = self.thread.messages.len() * 3;
        // Set scroll to a high value; rendering will clamp it appropriately
        self.transcript_scroll = estimated_lines.saturating_sub(10);
    }

    /// Update the draft content.
    pub fn update_draft(&mut self, draft: String) {
        self.thread.draft = draft;
    }

    /// Get the currently selected model for chat.
    pub fn current_chat_model(&self) -> Option<&ModelStatus> {
        self.models.get(self.chat_model_index)
    }

    /// Get enabled models for chat.
    pub fn enabled_models(&self) -> Vec<&ModelStatus> {
        self.models.iter().filter(|m| m.enabled).collect()
    }

    /// Export transcript to a markdown file.
    fn export_transcript(&mut self) {
        use ralf_engine::Role;

        let mut content = String::new();
        content.push_str("# Spec Studio Transcript\n\n");
        content.push_str(&format!("Thread: {}\n", self.thread.title));
        content.push_str(&format!("Thread ID: {}\n\n", self.thread.id));
        content.push_str("---\n\n");

        for msg in &self.thread.messages {
            let role = match msg.role {
                Role::User => "**You**",
                Role::Assistant => {
                    if let Some(model) = &msg.model {
                        model.as_str()
                    } else {
                        "**Assistant**"
                    }
                }
                Role::System => "**System**",
            };
            content.push_str(&format!("### {}\n\n", role));
            content.push_str(&msg.content);
            content.push_str("\n\n");
        }

        if !self.thread.draft.is_empty() {
            content.push_str("---\n\n## Current Draft\n\n");
            content.push_str(&self.thread.draft);
        }

        let export_path = self
            .repo_path
            .join(".ralf")
            .join("spec")
            .join("transcript-export.md");
        match std::fs::write(&export_path, &content) {
            Ok(()) => {
                self.set_notification(format!("Exported to {}", export_path.display()));
            }
            Err(e) => {
                self.set_notification(format!("Export failed: {e}"));
            }
        }
    }

    // === Run Dashboard handlers ===

    fn handle_run_dashboard_action(&mut self, action: Action) {
        match action {
            Action::Back | Action::Cancel => {
                if self.run_state.status == RunStatus::Running {
                    // Cancel the run
                    self.request_cancel_run();
                } else {
                    // Go back to welcome
                    self.screen = Screen::Welcome;
                }
            }
            Action::Select => {
                if self.run_state.status != RunStatus::Running {
                    // Start a new run (resets state)
                    self.start_run();
                }
            }
            // Note: Tab actions are no-ops in multi-pane view
            Action::NextTab | Action::PrevTab | Action::Tab(_) => {}
            Action::ToggleFollow => {
                self.run_state.follow_output = !self.run_state.follow_output;
            }
            Action::Up => {
                if self.run_state.output_scroll > 0 {
                    self.run_state.output_scroll -= 1;
                    // Disable follow when user scrolls up
                    self.run_state.follow_output = false;
                }
            }
            Action::Down => {
                // Bound scroll to content length (leave at least 1 visible line)
                let total_lines = self.run_state.model_output.lines().count();
                let max_scroll = total_lines.saturating_sub(1);
                if self.run_state.output_scroll < max_scroll {
                    self.run_state.output_scroll += 1;
                }
            }
            _ => {}
        }
    }

    /// Start a new run.
    pub fn start_run(&mut self) {
        // Check prerequisites
        let Some(config) = self.config.clone() else {
            self.set_notification("No config found. Run setup first.".to_string());
            return;
        };

        let prompt_path = self.repo_path.join("PROMPT.md");
        if !prompt_path.exists() {
            self.set_notification("No PROMPT.md found. Create a spec first.".to_string());
            return;
        }

        // Parse criteria from PROMPT.md
        let criteria = if let Ok(prompt_content) = std::fs::read_to_string(&prompt_path) {
            parse_criteria(&prompt_content)
        } else {
            Vec::new()
        };

        // Reset run state
        self.run_state = RunState {
            status: RunStatus::Running,
            started_at: Some(Instant::now()),
            max_iterations: 10, // Default max iterations
            follow_output: true, // Auto-follow by default
            criteria,
            ..Default::default()
        };

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.run_event_rx = Some(event_rx);

        // Create run config
        let run_config = RunConfig {
            max_iterations: self.run_state.max_iterations,
            max_runtime_secs: 0, // No timeout for now
            prompt_path,
            repo_path: self.repo_path.clone(),
        };

        // Update git info at run start
        self.git_info = get_git_info();

        // Start the run
        let handle = ralf_engine::start_run(config, run_config, event_tx);
        self.run_handle = Some(handle);

        self.run_state.push_event("Run started".to_string());
    }

    /// Request cancellation of the current run.
    pub fn request_cancel_run(&mut self) {
        // Avoid spamming cancel requests
        if self.run_state.cancel_requested {
            return;
        }

        if let Some(handle) = &self.run_handle {
            // Use non-blocking try_cancel to send signal immediately
            if handle.try_cancel() {
                self.run_state.cancel_requested = true;
                self.run_state
                    .push_event("Cancel requested...".to_string());
            } else {
                self.run_state
                    .push_event("Cancel signal failed (channel full)".to_string());
            }
        }
    }

    /// Process any pending run events.
    pub fn process_run_events(&mut self) {
        // Collect events first to avoid borrow issues
        let events: Vec<RunEvent> = {
            let Some(rx) = &mut self.run_event_rx else {
                return;
            };
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            events
        };

        // Process collected events
        for event in events {
            self.handle_run_event(event);
        }
    }

    /// Handle a single run event.
    fn handle_run_event(&mut self, event: RunEvent) {
        match event {
            RunEvent::Started {
                run_id,
                max_iterations,
            } => {
                self.run_state.run_id = Some(run_id.clone());
                self.run_state.max_iterations = max_iterations;
                self.run_state.push_event(format!("Run {run_id} started"));
            }
            RunEvent::IterationStarted { iteration, model } => {
                self.run_state.current_iteration = iteration;
                self.run_state.current_model = Some(model.clone());
                self.run_state.model_output.clear();
                self.run_state.output_scroll = 0;
                // Clear previous iteration's results
                self.run_state.verifier_results.clear();
                // Clear stale cooldowns - the engine handles actual expiry
                self.run_state.cooldowns.clear();
                self.run_state
                    .push_event(format!("Iteration {iteration}: {model}"));
            }
            RunEvent::ModelCompleted {
                iteration,
                model,
                duration_ms,
                has_promise,
                rate_limited,
                output_preview,
            } => {
                self.run_state.model_output = output_preview;

                // Auto-scroll to bottom if follow mode is enabled
                if self.run_state.follow_output {
                    let total_lines = self.run_state.model_output.lines().count();
                    self.run_state.output_scroll = total_lines.saturating_sub(1);
                }

                let status = if rate_limited {
                    "rate limited"
                } else if has_promise {
                    "promise found"
                } else {
                    "no promise"
                };
                self.run_state.push_event(format!(
                    "Model {} completed ({}ms) - {}",
                    model, duration_ms, status
                ));

                // Note: git info is updated at run start, not after each model
                // to avoid blocking the event loop with shell commands

                // Ignore unused variable warnings
                let _ = iteration;
            }
            RunEvent::VerifierCompleted {
                iteration,
                name,
                passed,
                duration_ms,
            } => {
                self.run_state
                    .verifier_results
                    .push((name.clone(), passed, duration_ms));
                let status = if passed { "PASS" } else { "FAIL" };
                self.run_state
                    .push_event(format!("Verifier {name}: {status}"));
                let _ = iteration;
            }
            RunEvent::CooldownStarted {
                model,
                duration_secs,
            } => {
                self.run_state
                    .cooldowns
                    .push((model.clone(), duration_secs));
                self.run_state
                    .push_event(format!("{model} in cooldown ({duration_secs}s)"));
            }
            RunEvent::IterationCompleted {
                iteration,
                all_verifiers_passed,
            } => {
                // Keep verifier_results visible - they're cleared on next IterationStarted
                let status = if all_verifiers_passed {
                    "all passed"
                } else {
                    "some failed"
                };
                self.run_state.push_event(format!(
                    "Iteration {iteration} complete - verifiers: {status}"
                ));
            }
            RunEvent::Completed { iteration, reason } => {
                self.run_state.status = RunStatus::Completed;
                self.run_state.completion_reason = Some(reason.clone());
                self.run_state
                    .push_event(format!("Completed at iteration {iteration}: {reason}"));
                self.run_handle = None;
                self.run_event_rx = None;
                // Refresh git info in background to show final state
                self.spawn_git_info_update();
            }
            RunEvent::Failed { iteration, error } => {
                self.run_state.status = RunStatus::Failed;
                self.run_state.error_message = Some(error.clone());
                self.run_state
                    .push_event(format!("Failed at iteration {iteration}: {error}"));
                self.run_handle = None;
                self.run_event_rx = None;
                // Refresh git info in background to show final state
                self.spawn_git_info_update();
            }
            RunEvent::Cancelled { iteration } => {
                self.run_state.status = RunStatus::Cancelled;
                self.run_state
                    .push_event(format!("Cancelled at iteration {iteration}"));
                self.run_handle = None;
                self.run_event_rx = None;
                // Refresh git info in background to show final state
                self.spawn_git_info_update();
            }
            RunEvent::Status { message } => {
                self.run_state.push_event(message);
            }
        }
    }

    /// Check if PROMPT.md exists.
    pub fn prompt_exists(&self) -> bool {
        self.repo_path.join("PROMPT.md").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_state_default() {
        let state = RunState::default();
        assert_eq!(state.status, RunStatus::Idle);
        assert!(state.run_id.is_none());
        assert_eq!(state.current_iteration, 0);
        assert_eq!(state.max_iterations, 0);
        assert!(state.current_model.is_none());
        assert!(state.started_at.is_none());
        assert!(state.model_output.is_empty());
        assert!(state.verifier_results.is_empty());
        assert!(state.cooldowns.is_empty());
        assert!(state.events.is_empty());
        assert!(state.criteria.is_empty());
    }

    #[test]
    fn test_screen_enum() {
        assert_eq!(Screen::default(), Screen::Welcome);
        assert_ne!(Screen::Welcome, Screen::Setup);
        assert_ne!(Screen::Setup, Screen::SpecStudio);
        assert_ne!(Screen::SpecStudio, Screen::RunDashboard);
    }

    #[test]
    fn test_model_status_creation() {
        let info = ModelInfo {
            name: "test".to_string(),
            found: true,
            callable: true,
            path: Some("/usr/bin/test".to_string()),
            version: None,
            issues: vec![],
        };
        let status = ModelStatus {
            info,
            probe_result: None,
            probing: false,
            probe_in_flight: false,
            enabled: true,
        };
        assert_eq!(status.info.name, "test");
        assert!(status.enabled);
        assert!(!status.probing);
    }
}
