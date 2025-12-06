use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, Mode, Modal, Field};

// ============================================================================
// Tokyo Night Color Theme
// ============================================================================
mod theme {
    use ratatui::style::Color;

    // Tokyo Night Storm palette
    pub const BG: Color = Color::Rgb(36, 40, 59);           // #24283b
    pub const BG_DARK: Color = Color::Rgb(26, 27, 38);      // #1a1b26
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66); // #292e42
    pub const FG: Color = Color::Rgb(169, 177, 214);        // #a9b1d6
    pub const FG_DARK: Color = Color::Rgb(86, 95, 137);     // #565f89
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);     // #565f89
    
    // Accent colors
    pub const BLUE: Color = Color::Rgb(122, 162, 247);      // #7aa2f7
    pub const CYAN: Color = Color::Rgb(125, 207, 255);      // #7dcfff
    pub const PURPLE: Color = Color::Rgb(187, 154, 247);    // #bb9af7
    pub const GREEN: Color = Color::Rgb(158, 206, 106);     // #9ece6a
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);    // #e0af68
    #[allow(dead_code)]
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);    // #ff9e64
    pub const RED: Color = Color::Rgb(247, 118, 142);       // #f7768e
    #[allow(dead_code)]
    pub const MAGENTA: Color = Color::Rgb(255, 117, 127);   // #ff757f
    pub const TEAL: Color = Color::Rgb(115, 218, 202);      // #73daca
}

pub fn render(app: &App, frame: &mut Frame) {
    // Split the frame into main area and persistent footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main content area
            Constraint::Length(3), // Persistent footer
        ])
        .split(frame.area());

    // Render main content based on mode
    match app.mode {
        Mode::InitPassphrase => render_passphrase_input(app, chunks[0], frame),
        Mode::Normal => render_main(app, chunks[0], frame),
    }

    // Render modal if open (overlaid)
    match app.modal {
        Modal::AddSecret => render_add_secret_modal(app, frame),
        Modal::DeleteConfirm => render_delete_confirm_modal(app, frame),
        Modal::Help => render_help_modal(frame),
        Modal::Command => render_command_modal(app, frame),
        Modal::None => {}
    }

    // Render persistent footer
    render_footer(app, chunks[1], frame);
}

fn render_passphrase_input(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Input and error
        ])
        .split(area);

    let title = Paragraph::new("🔒 LAZY LOCKER - Initialisation 🔒")
        .style(Style::default().fg(theme::CYAN).bold())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BLUE))
                .style(Style::default().bg(theme::BG_DARK))
                .title(" Info "),
        );

    let passphrase_str = String::from_utf8_lossy(&app.passphrase);
    let masked_passphrase = "*".repeat(passphrase_str.len());
    let mut input_text = format!("Passphrase: {}", masked_passphrase);
    if let Some(ref error) = app.error_message {
        input_text.push_str(&format!("\n\n❌ Error: {}", error));
    }
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(if app.error_message.is_some() { theme::RED } else { theme::FG }))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::PURPLE))
                .style(Style::default().bg(theme::BG_DARK))
                .title(" Enter your passphrase (Enter to confirm) ")
        );

    frame.render_widget(title, chunks[0]);
    frame.render_widget(input, chunks[1]);
}

fn render_main(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Header with agent status indicator
    let agent_indicator = if app.agent_mode { " 🟢 Agent" } else { "" };
    let title = Paragraph::new(format!("🔒 LAZY LOCKER 🔒{}", agent_indicator))
        .style(Style::default().fg(theme::CYAN).bold())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BLUE))
                .style(Style::default().bg(theme::BG_DARK))
                .title(" Secrets Manager ")
        );

    frame.render_widget(title, chunks[0]);

    // Horizontal layout: secrets list (left) + usages (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    // Left panel: secrets list
    render_secrets_list(app, main_chunks[0], frame);

    // Right panel: files using the selected token
    render_token_usages(app, main_chunks[1], frame);
}

fn render_secrets_list(app: &App, area: Rect, frame: &mut Frame) {
    let count = app.secrets_count();
    
    if count == 0 {
        let empty_msg = Paragraph::new("No secrets. Press 'a' to add one.")
            .style(Style::default().fg(theme::COMMENT))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::BLUE))
                    .style(Style::default().bg(theme::BG_DARK))
                    .title(" Secrets ")
            );
        frame.render_widget(empty_msg, area);
        return;
    }

    // Build items from agent_secrets or store
    let items: Vec<ListItem> = if let Some(ref secrets) = app.agent_secrets {
        // Agent mode: display from agent_secrets
        let mut names: Vec<_> = secrets.keys().collect();
        names.sort();
        
        names.iter().enumerate().map(|(i, name)| {
            let is_selected = i == app.selected_index;
            let prefix = if is_selected { "▶ " } else { "  " };
            
            let value_display = if is_selected {
                if let Some(ref revealed) = app.revealed_secret {
                    revealed.clone()
                } else {
                    "********".to_string()
                }
            } else {
                "********".to_string()
            };
            
            let display = format!("{}{}: {} [via agent]", prefix, name, value_display);
            
            let style = if is_selected {
                Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::FG)
            };
            
            ListItem::new(display).style(style)
        }).collect()
    } else if let Some(ref store) = app.secrets_store {
        // Normal mode: display from store
        store.list_secrets().iter().enumerate().map(|(i, s)| {
            let is_selected = i == app.selected_index;
            let prefix = if is_selected { "▶ " } else { "  " };
            
            let value_display = if is_selected {
                if let Some(ref revealed) = app.revealed_secret {
                    revealed.clone()
                } else {
                    "********".to_string()
                }
            } else {
                "********".to_string()
            };
            
            let expiration = s.expiration_display();
            let display = format!("{}{}: {} [{}]", prefix, s.name, value_display, expiration);
            
            let style = if is_selected {
                Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
            } else if s.is_expired() {
                Style::default().fg(theme::RED)
            } else {
                Style::default().fg(theme::FG)
            };
            
            ListItem::new(display).style(style)
        }).collect()
    } else {
        Vec::new()
    };
    
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::PURPLE))
                .style(Style::default().bg(theme::BG_DARK))
                .title(" Secrets (↑↓ navigate) ")
        );
    frame.render_widget(list, area);
}

fn render_token_usages(app: &App, area: Rect, frame: &mut Frame) {
    let title = if let Some(name) = app.get_selected_secret_name() {
        format!(" Usage of '{}' ", name)
    } else {
        " Usage ".to_string()
    };

    if app.token_usages.is_empty() {
        let msg = if app.get_selected_secret_name().is_some() {
            "No usage found\nin the current directory."
        } else {
            "Select a secret\nto see its usages."
        };
        let paragraph = Paragraph::new(msg)
            .style(Style::default().fg(theme::COMMENT))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::TEAL))
                    .style(Style::default().bg(theme::BG_DARK))
                    .title(title)
            );
        frame.render_widget(paragraph, area);
    } else {
        let items: Vec<ListItem> = app.token_usages
            .iter()
            .take(20) // Limit to 20 results
            .map(|usage| {
                let display = format!(
                    "{}:{}\n  {}",
                    usage.file_path.split('/').last().unwrap_or(&usage.file_path),
                    usage.line_number,
                    if usage.line_content.len() > 40 {
                        format!("{}...", &usage.line_content[..40])
                    } else {
                        usage.line_content.clone()
                    }
                );
                ListItem::new(display).style(Style::default().fg(theme::FG))
            })
            .collect();
        
        let count = app.token_usages.len();
        let title_with_count = format!("{} ({} files)", title, count);
        
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::TEAL))
                    .style(Style::default().bg(theme::BG_DARK))
                    .title(title_with_count)
            );
        frame.render_widget(list, area);
    }
}

/// Calcule un rectangle centré pour les modals
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_add_secret_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 50, frame.area());
    
    // Clear the area behind the modal
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(" Add a Secret ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::GREEN))
        .style(Style::default().bg(theme::BG_HIGHLIGHT));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Nom
            Constraint::Length(3), // Valeur
            Constraint::Length(3), // Expiration
            Constraint::Min(1),    // Instructions
        ])
        .split(inner);
    
    let name_style = if app.current_field == Field::Name {
        Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::FG)
    };
    
    let value_style = if app.current_field == Field::Value {
        Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::FG)
    };
    
    let expiration_style = if app.current_field == Field::Expiration {
        Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::FG)
    };
    
    let name_input = Paragraph::new(app.new_secret_name.as_str())
        .style(name_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.current_field == Field::Name { theme::CYAN } else { theme::FG_DARK }))
                .title(" Name (Enter: next) ")
        );
    
    // Display token in plain text (not masked)
    let value_input = Paragraph::new(app.new_secret_value.as_str())
        .style(value_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.current_field == Field::Value { theme::CYAN } else { theme::FG_DARK }))
                .title(" Plain text token (Enter: next) ")
        );
    
    let expiration_display = if app.new_secret_expiration.is_empty() {
        "Permanent (empty = no expiration)".to_string()
    } else {
        format!("{} days", app.new_secret_expiration)
    };
    let expiration_input = Paragraph::new(expiration_display)
        .style(expiration_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.current_field == Field::Expiration { theme::CYAN } else { theme::FG_DARK }))
                .title(" Expiration in days (Enter: confirm) ")
        );
    
    let instructions = Paragraph::new("Tab: switch field | Enter: next/confirm | Esc: cancel")
        .style(Style::default().fg(theme::COMMENT))
        .alignment(Alignment::Center);
    
    frame.render_widget(name_input, chunks[0]);
    frame.render_widget(value_input, chunks[1]);
    frame.render_widget(expiration_input, chunks[2]);
    frame.render_widget(instructions, chunks[3]);
}

fn render_delete_confirm_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 30, frame.area());
    
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(" ⚠️ Confirm deletion ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::RED))
        .style(Style::default().bg(theme::BG_HIGHLIGHT));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let secret_name = app.get_selected_secret_name().unwrap_or_else(|| "?".to_string());
    let text = format!(
        "Do you really want to delete secret '{}' ?\n\n[Y] Yes  |  [N] No / Esc",
        secret_name
    );
    
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme::FG))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    frame.render_widget(paragraph, inner);
}

fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(" 📖 Help - Keyboard shortcuts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::PURPLE))
        .style(Style::default().bg(theme::BG_HIGHLIGHT));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let help_text = vec![
        "Navigation:",
        "  ↑/↓     Navigate between secrets",
        "",
        "Actions on secrets:",
        "  a       Add a new secret",
        "  e       Reveal/hide the selected token",
        "  y       Copy decrypted token to clipboard",
        "  d       Delete the selected secret",
        "",
        "Commands (press : to open):",
        "  :env    Generate .env file (plain text)",
        "  :bash   Export to ~/.bashrc",
        "  :zsh    Export to ~/.zshrc",
        "  :fish   Export to fish config",
        "  :json   Export as JSON file",
        "  :clear  Remove exports from shell profiles",
        "",
        "General:",
        "  h       Show this help",
        "  q       Quit application",
        "  Esc     Close modal / Cancel",
        "",
        "In the add form:",
        "  Tab     Switch field",
        "  Enter   Go to next field / Confirm",
        "",
        "Press Esc or h to close",
    ];
    
    let paragraph = Paragraph::new(help_text.join("\n"))
        .style(Style::default().fg(theme::FG))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    
    frame.render_widget(paragraph, inner);
}

fn render_command_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 40, frame.area());
    
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(" ⌨ Command ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::CYAN))
        .style(Style::default().bg(theme::BG_HIGHLIGHT));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    // Split inner area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input field
            Constraint::Min(1),    // Suggestions
        ])
        .split(inner);
    
    // Input field with colon prefix
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::PURPLE));
    let input_text = format!(":{}", app.command_input);
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(theme::FG).add_modifier(Modifier::BOLD))
        .block(input_block);
    frame.render_widget(input, chunks[0]);
    
    // Suggestions list
    let suggestions = app.get_command_suggestions();
    let items: Vec<Line> = suggestions
        .iter()
        .enumerate()
        .map(|(i, (cmd, desc))| {
            let style = if i == app.command_suggestion_index {
                Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::FG)
            };
            let prefix = if i == app.command_suggestion_index { "► " } else { "  " };
            Line::from(vec![
                Span::styled(format!("{}{}", prefix, cmd), style),
                Span::styled(format!("  - {}", desc), Style::default().fg(theme::COMMENT)),
            ])
        })
        .collect();
    
    let suggestions_block = Block::default()
        .title(" Suggestions (↑/↓ to select, Enter to execute) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::FG_DARK));
    
    let suggestions_widget = Paragraph::new(items)
        .block(suggestions_block)
        .wrap(Wrap { trim: false });
    
    frame.render_widget(suggestions_widget, chunks[1]);
}

fn render_footer(app: &App, area: Rect, frame: &mut Frame) {
    // Display status message if it exists
    let helper_text = if let Some(ref status) = app.status_message {
        status.as_str()
    } else {
        match (&app.mode, &app.modal) {
            (Mode::InitPassphrase, _) => "Type passphrase and Enter. Esc to quit.",
            (_, Modal::AddSecret) => "Tab: field | Enter: next/confirm | Esc: cancel",
            (_, Modal::DeleteConfirm) => "Y: confirm | N/Esc: cancel",
            (_, Modal::Help) => "Esc/h: close help",
            (_, Modal::Command) => "↑/↓: select | Enter: execute | Esc: cancel",
            (Mode::Normal, Modal::None) => "a: add | e: reveal | y: copy | d: delete | :: cmd | h: help | q: quit",
        }
    };

    let style = if app.status_message.is_some() {
        Style::default().fg(theme::GREEN)
    } else {
        Style::default().fg(theme::COMMENT)
    };

    let helper = Paragraph::new(helper_text)
        .style(style)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::FG_DARK))
                .style(Style::default().bg(theme::BG_DARK))
                .title(" Shortcuts ")
        );

    frame.render_widget(helper, area);
}