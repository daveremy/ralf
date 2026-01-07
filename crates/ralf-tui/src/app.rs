//! Application state and update logic for the ralf TUI.

use crate::event::Action;
use ralf_engine::{discover_models, get_git_info, Config, GitInfo, ModelConfig, ModelInfo, ProbeResult};
use std::path::PathBuf;

/// The current screen being displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Welcome,
    Setup,
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

        Self {
            should_quit: false,
            show_help: false,
            screen: Screen::Welcome,
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
        }
    }

    /// Handle an action.
    pub fn handle_action(&mut self, action: Action) {
        // Global actions
        match action {
            Action::Quit => {
                if self.show_help {
                    self.show_help = false;
                } else {
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
        }
    }

    fn handle_welcome_action(&mut self, action: Action) {
        if action == Action::Setup {
            self.screen = Screen::Setup;
            self.start_probing();
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
            Action::Retry => {
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
            models: enabled_model_names.iter().map(|n| ModelConfig::default_for(n)).collect(),
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
    }
}
