use std::collections::HashMap;
use zeroize::Zeroize;
use crate::core::store::SecretsStore;

/// Main application mode (single main view with overlaid modals)
#[derive(Debug, PartialEq, Clone)]
pub enum Mode {
    /// Passphrase input at startup
    InitPassphrase,
    /// Main view with secrets list
    Normal,
}

/// Modal overlaid on the main view
#[derive(Debug, PartialEq, Clone)]
pub enum Modal {
    None,
    /// Add secret form
    AddSecret,
    /// Delete confirmation
    DeleteConfirm,
    /// Help with hotkey list
    Help,
    /// Command input (vim-style :command)
    Command,
}

/// Available commands for the command modal
pub const COMMANDS: &[(&str, &str)] = &[
    ("env", "Generate .env file with secrets in plain text"),
    ("bash", "Export secrets to ~/.bashrc"),
    ("zsh", "Export secrets to ~/.zshrc"),
    ("fish", "Export secrets to ~/.config/fish/config.fish"),
    ("json", "Export secrets as JSON file"),
    ("clear", "Clear all shell exports from profile files"),
];

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Field {
    Name,
    Value,
    Expiration,
}

pub struct App {
    pub should_quit: bool,
    pub initialized: bool,
    pub mode: Mode,
    pub modal: Modal,
    pub passphrase: Vec<u8>,
    pub error_message: Option<String>,
    pub secrets_store: Option<SecretsStore>,
    // Fields for AddSecret modal
    pub new_secret_name: String,
    pub new_secret_value: String,
    pub new_secret_expiration: String, // Number of days (empty = permanent)
    pub current_field: Field,
    // Navigation in the secrets list
    pub selected_index: usize,
    // Display decrypted token
    pub revealed_secret: Option<String>,
    // Files using the selected token
    pub token_usages: Vec<crate::core::executor::TokenUsage>,
    // Temporary status message
    pub status_message: Option<String>,
    // Agent mode: if true, secrets are decrypted via agent
    pub agent_mode: bool,
    // Secrets from agent (name -> value), used when agent_mode is true
    pub agent_secrets: Option<HashMap<String, String>>,
    // Command input for command modal
    pub command_input: String,
    // Selected command suggestion index
    pub command_suggestion_index: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            initialized: false,
            mode: Mode::Normal,
            modal: Modal::None,
            passphrase: Vec::new(),
            error_message: None,
            secrets_store: None,
            new_secret_name: String::new(),
            new_secret_value: String::new(),
            new_secret_expiration: String::new(),
            current_field: Field::Name,
            selected_index: 0,
            revealed_secret: None,
            token_usages: Vec::new(),
            status_message: None,
            agent_mode: false,
            agent_secrets: None,
            command_input: String::new(),
            command_suggestion_index: 0,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn enter_init_mode(&mut self) {
        self.mode = Mode::InitPassphrase;
        self.error_message = None;
    }

    pub fn open_add_modal(&mut self) {
        self.modal = Modal::AddSecret;
        self.new_secret_name.clear();
        self.new_secret_value.clear();
        self.new_secret_expiration.clear();
        self.current_field = Field::Name;
    }

    pub fn open_delete_modal(&mut self) {
        self.modal = Modal::DeleteConfirm;
    }

    pub fn open_help_modal(&mut self) {
        self.modal = Modal::Help;
    }

    pub fn open_command_modal(&mut self) {
        self.modal = Modal::Command;
        self.command_input.clear();
        self.command_suggestion_index = 0;
    }

    /// Get filtered command suggestions based on current input
    pub fn get_command_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        let input = self.command_input.to_lowercase();
        COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(&input))
            .copied()
            .collect()
    }

    /// Get the currently selected command (if any)
    pub fn get_selected_command(&self) -> Option<&'static str> {
        let suggestions = self.get_command_suggestions();
        suggestions.get(self.command_suggestion_index).map(|(cmd, _)| *cmd)
    }

    pub fn close_modal(&mut self) {
        self.modal = Modal::None;
        self.revealed_secret = None;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Returns the name of the currently selected secret
    pub fn get_selected_secret_name(&self) -> Option<String> {
        // Agent mode: use agent_secrets
        if let Some(ref secrets) = self.agent_secrets {
            let mut names: Vec<_> = secrets.keys().collect();
            names.sort();
            if self.selected_index < names.len() {
                return Some(names[self.selected_index].clone());
            }
        }
        // Normal mode: use store
        if let Some(ref store) = self.secrets_store {
            let secrets = store.list_secrets();
            if self.selected_index < secrets.len() {
                return Some(secrets[self.selected_index].name.clone());
            }
        }
        None
    }

    /// Number of secrets in the store
    pub fn secrets_count(&self) -> usize {
        // Agent mode
        if let Some(ref secrets) = self.agent_secrets {
            return secrets.len();
        }
        // Normal mode
        self.secrets_store.as_ref().map(|s| s.list_secrets().len()).unwrap_or(0)
    }
    
    /// Returns list of secret names (sorted)
    pub fn get_secret_names(&self) -> Vec<String> {
        if let Some(ref secrets) = self.agent_secrets {
            let mut names: Vec<_> = secrets.keys().cloned().collect();
            names.sort();
            return names;
        }
        if let Some(ref store) = self.secrets_store {
            return store.list_secrets().iter().map(|s| s.name.clone()).collect();
        }
        Vec::new()
    }
    
    /// Gets decrypted value from agent_secrets cache
    pub fn get_agent_secret_value(&self, name: &str) -> Option<String> {
        self.agent_secrets.as_ref().and_then(|s| s.get(name).cloned())
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.revealed_secret = None;
        }
    }

    pub fn move_selection_down(&mut self) {
        let count = self.secrets_count();
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
            self.revealed_secret = None;
        }
    }

    pub fn handle_key(&mut self, key_code: crossterm::event::KeyCode) {
        // If a modal is open, handle its events
        match self.modal {
            Modal::AddSecret => {
                match key_code {
                    crossterm::event::KeyCode::Char(c) => {
                        match self.current_field {
                            Field::Name => self.new_secret_name.push(c),
                            Field::Value => self.new_secret_value.push(c),
                            Field::Expiration => {
                                // Only accept digits for expiration
                                if c.is_ascii_digit() {
                                    self.new_secret_expiration.push(c);
                                }
                            }
                        }
                    }
                    crossterm::event::KeyCode::Backspace => {
                        match self.current_field {
                            Field::Name => { self.new_secret_name.pop(); }
                            Field::Value => { self.new_secret_value.pop(); }
                            Field::Expiration => { self.new_secret_expiration.pop(); }
                        }
                    }
                    crossterm::event::KeyCode::Tab => {
                        self.current_field = match self.current_field {
                            Field::Name => Field::Value,
                            Field::Value => Field::Expiration,
                            Field::Expiration => Field::Name,
                        };
                    }
                    crossterm::event::KeyCode::Enter => {
                        // Enter avance au champ suivant, sauf sur Expiration où il valide
                        match self.current_field {
                            Field::Name => self.current_field = Field::Value,
                            Field::Value => self.current_field = Field::Expiration,
                            Field::Expiration => {} // Handled in main.rs (validation)
                        }
                    }
                    crossterm::event::KeyCode::Esc => self.close_modal(),
                    _ => {}
                }
                return;
            }
            Modal::DeleteConfirm => {
                match key_code {
                    crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Enter => {} // Handled in main.rs
                    crossterm::event::KeyCode::Char('n') | crossterm::event::KeyCode::Esc => self.close_modal(),
                    _ => {}
                }
                return;
            }
            Modal::Help => {
                match key_code {
                    crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('h') | crossterm::event::KeyCode::Enter => {
                        self.close_modal();
                    }
                    _ => {}
                }
                return;
            }
            Modal::Command => {
                match key_code {
                    crossterm::event::KeyCode::Char(c) => {
                        self.command_input.push(c);
                        self.command_suggestion_index = 0; // Reset selection on input change
                    }
                    crossterm::event::KeyCode::Backspace => {
                        self.command_input.pop();
                        self.command_suggestion_index = 0;
                    }
                    crossterm::event::KeyCode::Tab | crossterm::event::KeyCode::Down => {
                        let suggestions = self.get_command_suggestions();
                        if !suggestions.is_empty() {
                            self.command_suggestion_index = 
                                (self.command_suggestion_index + 1) % suggestions.len();
                        }
                    }
                    crossterm::event::KeyCode::Up => {
                        let suggestions = self.get_command_suggestions();
                        if !suggestions.is_empty() {
                            self.command_suggestion_index = 
                                self.command_suggestion_index.checked_sub(1)
                                    .unwrap_or(suggestions.len() - 1);
                        }
                    }
                    crossterm::event::KeyCode::Enter => {} // Handled in main.rs
                    crossterm::event::KeyCode::Esc => self.close_modal(),
                    _ => {}
                }
                return;
            }
            Modal::None => {}
        }

        // Handle main modes
        match self.mode {
            Mode::InitPassphrase => match key_code {
                crossterm::event::KeyCode::Char(c) => {
                    self.passphrase.push(c as u8);
                    self.error_message = None;
                }
                crossterm::event::KeyCode::Backspace => {
                    self.passphrase.pop();
                    self.error_message = None;
                }
                crossterm::event::KeyCode::Enter => {} // Handled in main.rs
                crossterm::event::KeyCode::Esc => self.quit(),
                _ => {}
            },
            Mode::Normal => match key_code {
                crossterm::event::KeyCode::Char('q') => self.quit(),
                crossterm::event::KeyCode::Char('a') => self.open_add_modal(),
                crossterm::event::KeyCode::Char('d') => {
                    if self.secrets_count() > 0 {
                        self.open_delete_modal();
                    }
                }
                crossterm::event::KeyCode::Char('h') => self.open_help_modal(),
                crossterm::event::KeyCode::Char('e') => {} // Handled in main.rs (decrypt)
                crossterm::event::KeyCode::Char(':') => self.open_command_modal(),
                crossterm::event::KeyCode::Char('y') => {} // Handled in main.rs (copy)
                crossterm::event::KeyCode::Up => self.move_selection_up(),
                crossterm::event::KeyCode::Down => self.move_selection_down(),
                _ => {}
            },
        }
    }

    /// Parse the number of expiration days from input
    pub fn get_expiration_days(&self) -> Option<u32> {
        if self.new_secret_expiration.is_empty() {
            None
        } else {
            self.new_secret_expiration.parse().ok()
        }
    }

    /// Updates the usages of the selected token
    pub fn update_token_usages(&mut self, work_dir: &std::path::PathBuf) {
        if let Some(name) = self.get_selected_secret_name() {
            self.token_usages = crate::core::executor::find_token_usages(&name, work_dir);
        } else {
            self.token_usages.clear();
        }
    }

    /// Displays a temporary status message
    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    /// Clears the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.passphrase.zeroize();
        self.new_secret_value.zeroize();
        if let Some(ref mut revealed) = self.revealed_secret {
            revealed.zeroize();
        }
        if let Some(ref mut store) = self.secrets_store {
            store.secrets.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    // ========================
    // App initialization tests
    // ========================

    #[test]
    fn test_app_new_defaults() {
        let app = App::new();

        assert!(!app.should_quit);
        assert!(!app.initialized);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.modal, Modal::None);
        assert!(app.passphrase.is_empty());
        assert!(app.error_message.is_none());
        assert!(app.secrets_store.is_none());
        assert_eq!(app.selected_index, 0);
        assert!(app.revealed_secret.is_none());
    }

    #[test]
    fn test_app_quit() {
        let mut app = App::new();
        assert!(!app.should_quit);

        app.quit();

        assert!(app.should_quit);
    }

    // ========================
    // Mode transitions tests
    // ========================

    #[test]
    fn test_enter_init_mode() {
        let mut app = App::new();
        app.error_message = Some("Previous error".to_string());

        app.enter_init_mode();

        assert_eq!(app.mode, Mode::InitPassphrase);
        assert!(app.error_message.is_none()); // Error should be cleared
    }

    // ========================
    // Modal tests
    // ========================

    #[test]
    fn test_open_add_modal() {
        let mut app = App::new();
        app.new_secret_name = "leftover".to_string();
        app.new_secret_value = "data".to_string();
        app.new_secret_expiration = "30".to_string();
        app.current_field = Field::Value;

        app.open_add_modal();

        assert_eq!(app.modal, Modal::AddSecret);
        assert!(app.new_secret_name.is_empty()); // Fields should be cleared
        assert!(app.new_secret_value.is_empty());
        assert!(app.new_secret_expiration.is_empty());
        assert_eq!(app.current_field, Field::Name); // Reset to first field
    }

    #[test]
    fn test_open_delete_modal() {
        let mut app = App::new();

        app.open_delete_modal();

        assert_eq!(app.modal, Modal::DeleteConfirm);
    }

    #[test]
    fn test_open_help_modal() {
        let mut app = App::new();

        app.open_help_modal();

        assert_eq!(app.modal, Modal::Help);
    }

    #[test]
    fn test_close_modal() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;
        app.revealed_secret = Some("exposed_secret".to_string());

        app.close_modal();

        assert_eq!(app.modal, Modal::None);
        assert!(app.revealed_secret.is_none()); // Should clear revealed secret
    }

    // ========================
    // Error handling tests
    // ========================

    #[test]
    fn test_set_and_clear_error() {
        let mut app = App::new();

        app.set_error("Something went wrong".to_string());
        assert_eq!(app.error_message, Some("Something went wrong".to_string()));

        app.clear_error();
        assert!(app.error_message.is_none());
    }

    // ========================
    // Status message tests
    // ========================

    #[test]
    fn test_set_and_clear_status() {
        let mut app = App::new();

        app.set_status("Copied to clipboard!".to_string());
        assert_eq!(app.status_message, Some("Copied to clipboard!".to_string()));

        app.clear_status();
        assert!(app.status_message.is_none());
    }

    // ========================
    // Navigation tests (without store)
    // ========================

    #[test]
    fn test_secrets_count_without_store() {
        let app = App::new();
        assert_eq!(app.secrets_count(), 0);
    }

    #[test]
    fn test_get_selected_secret_name_without_store() {
        let app = App::new();
        assert!(app.get_selected_secret_name().is_none());
    }

    #[test]
    fn test_move_selection_empty_store() {
        let mut app = App::new();
        
        // Should not panic or change index
        app.move_selection_up();
        assert_eq!(app.selected_index, 0);

        app.move_selection_down();
        assert_eq!(app.selected_index, 0);
    }

    // ========================
    // Field navigation tests (via handle_key)
    // ========================

    #[test]
    fn test_tab_navigates_fields() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;
        app.current_field = Field::Name;

        app.handle_key(KeyCode::Tab);
        assert_eq!(app.current_field, Field::Value);

        app.handle_key(KeyCode::Tab);
        assert_eq!(app.current_field, Field::Expiration);

        app.handle_key(KeyCode::Tab);
        assert_eq!(app.current_field, Field::Name); // Cycle back
    }

    #[test]
    fn test_enter_navigates_fields_except_expiration() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;
        app.current_field = Field::Name;

        app.handle_key(KeyCode::Enter);
        assert_eq!(app.current_field, Field::Value);

        app.handle_key(KeyCode::Enter);
        assert_eq!(app.current_field, Field::Expiration);

        // Enter on Expiration does NOT cycle (handled externally for validation)
        app.handle_key(KeyCode::Enter);
        assert_eq!(app.current_field, Field::Expiration);
    }

    // ========================
    // Key handling tests (AddSecret modal)
    // ========================

    #[test]
    fn test_handle_key_add_modal_escape() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;

        app.handle_key(KeyCode::Esc);

        assert_eq!(app.modal, Modal::None);
    }

    #[test]
    fn test_handle_key_add_modal_tab_navigates() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;
        app.current_field = Field::Name;

        app.handle_key(KeyCode::Tab);

        assert_eq!(app.current_field, Field::Value);
    }

    #[test]
    fn test_handle_key_add_modal_char_input() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;
        app.current_field = Field::Name;

        app.handle_key(KeyCode::Char('A'));
        app.handle_key(KeyCode::Char('P'));
        app.handle_key(KeyCode::Char('I'));

        assert_eq!(app.new_secret_name, "API");
    }

    #[test]
    fn test_handle_key_add_modal_backspace() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;
        app.current_field = Field::Name;
        app.new_secret_name = "APIKEY".to_string();

        app.handle_key(KeyCode::Backspace);

        assert_eq!(app.new_secret_name, "APIKE");
    }

    #[test]
    fn test_handle_key_add_modal_expiration_only_digits() {
        let mut app = App::new();
        app.modal = Modal::AddSecret;
        app.current_field = Field::Expiration;

        app.handle_key(KeyCode::Char('3'));
        app.handle_key(KeyCode::Char('0'));
        app.handle_key(KeyCode::Char('a')); // Should be ignored

        assert_eq!(app.new_secret_expiration, "30");
    }

    // ========================
    // Key handling tests (DeleteConfirm modal)
    // ========================

    #[test]
    fn test_handle_key_delete_modal_escape() {
        let mut app = App::new();
        app.modal = Modal::DeleteConfirm;

        app.handle_key(KeyCode::Esc);

        assert_eq!(app.modal, Modal::None);
    }

    #[test]
    fn test_handle_key_delete_modal_n_closes() {
        let mut app = App::new();
        app.modal = Modal::DeleteConfirm;

        app.handle_key(KeyCode::Char('n'));

        assert_eq!(app.modal, Modal::None);
    }

    // ========================
    // Key handling tests (Help modal)
    // ========================

    #[test]
    fn test_handle_key_help_modal_specific_keys_close() {
        // Help modal closes only on Esc, 'h', or Enter
        let mut app = App::new();
        app.modal = Modal::Help;

        // Random key should NOT close the modal
        app.handle_key(KeyCode::Char('x'));
        assert_eq!(app.modal, Modal::Help);

        // 'h' should close
        app.handle_key(KeyCode::Char('h'));
        assert_eq!(app.modal, Modal::None);

        // Reset and test Enter
        app.modal = Modal::Help;
        app.handle_key(KeyCode::Enter);
        assert_eq!(app.modal, Modal::None);
    }

    #[test]
    fn test_handle_key_help_modal_escape_closes() {
        let mut app = App::new();
        app.modal = Modal::Help;

        app.handle_key(KeyCode::Esc);

        assert_eq!(app.modal, Modal::None);
    }

    // ========================
    // Key handling tests (Normal mode - no modal)
    // ========================

    #[test]
    fn test_handle_key_normal_mode_no_effect() {
        let mut app = App::new();
        app.modal = Modal::None;
        let initial_field = app.current_field;

        // Keys in normal mode should not affect add modal fields
        app.handle_key(KeyCode::Tab);
        
        assert_eq!(app.current_field, initial_field);
    }
}