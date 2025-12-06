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
}

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
        self.secrets_store.as_ref().map(|s| s.list_secrets().len()).unwrap_or(0)
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
                crossterm::event::KeyCode::Char('r') => {} // Handled in main.rs (run command)
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