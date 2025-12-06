use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, Mode, Modal, Field};

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
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title(" Info "),
        );

    let passphrase_str = String::from_utf8_lossy(&app.passphrase);
    let masked_passphrase = "*".repeat(passphrase_str.len());
    let mut input_text = format!("Passphrase: {}", masked_passphrase);
    if let Some(ref error) = app.error_message {
        input_text.push_str(&format!("\n\nError: {}", error));
    }
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(if app.error_message.is_some() { Color::Red } else { Color::Gray }))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title(" Enter your passphrase (Enter to confirm) "));

    frame.render_widget(title, chunks[0]);
    frame.render_widget(input, chunks[1]);
}

fn render_main(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let title = Paragraph::new("🔒 LAZY LOCKER 🔒")
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Secrets Manager "));

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
    if let Some(ref store) = app.secrets_store {
        let secrets = store.list_secrets();
        if secrets.is_empty() {
            let empty_msg = Paragraph::new("No secrets. Press 'a' to add one.")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title(" Secrets "));
            frame.render_widget(empty_msg, area);
        } else {
            let items: Vec<ListItem> = secrets
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let is_selected = i == app.selected_index;
                    let prefix = if is_selected { "▶ " } else { "  " };
                    
                    // Display decrypted value if revealed
                    let value_display = if is_selected {
                        if let Some(ref revealed) = app.revealed_secret {
                            revealed.clone()
                        } else {
                            "********".to_string()
                        }
                    } else {
                        "********".to_string()
                    };
                    
                    // Display expiration
                    let expiration = s.expiration_display();
                    
                    let display = format!("{}{}: {} [{}]", prefix, s.name, value_display, expiration);
                    
                    let style = if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else if s.is_expired() {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    
                    ListItem::new(display).style(style)
                })
                .collect();
            
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Secrets (↑↓ navigate) "));
            frame.render_widget(list, area);
        }
    } else {
        let msg = Paragraph::new("Loading...")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Secrets "));
        frame.render_widget(msg, area);      
    }
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
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(title));
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
                ListItem::new(display).style(Style::default().fg(Color::White))
            })
            .collect();
        
        let count = app.token_usages.len();
        let title_with_count = format!("{} ({} files)", title, count);
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title_with_count));
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
        .style(Style::default().bg(Color::DarkGray));
    
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
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let value_style = if app.current_field == Field::Value {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let expiration_style = if app.current_field == Field::Expiration {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let name_input = Paragraph::new(app.new_secret_name.as_str())
        .style(name_style)
        .block(Block::default().borders(Borders::ALL).title(" Name (Enter: next) "));
    
    // Display token in plain text (not masked)
    let value_input = Paragraph::new(app.new_secret_value.as_str())
        .style(value_style)
        .block(Block::default().borders(Borders::ALL).title(" Plain text token (Enter: next) "));
    
    let expiration_display = if app.new_secret_expiration.is_empty() {
        "Permanent (empty = no expiration)".to_string()
    } else {
        format!("{} days", app.new_secret_expiration)
    };
    let expiration_input = Paragraph::new(expiration_display)
        .style(expiration_style)
        .block(Block::default().borders(Borders::ALL).title(" Expiration in days (Enter: confirm) "));
    
    let instructions = Paragraph::new("Tab: switch field | Enter: next/confirm | Esc: cancel")
        .style(Style::default().fg(Color::DarkGray))
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
        .title(" Confirm deletion ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray).fg(Color::Red));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let secret_name = app.get_selected_secret_name().unwrap_or_else(|| "?".to_string());
    let text = format!(
        "Do you really want to delete secret '{}' ?\n\n[Y] Yes  |  [N] No / Esc",
        secret_name
    );
    
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    frame.render_widget(paragraph, inner);
}

fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(" Help - Keyboard shortcuts ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    
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
        "Execution:",
        "  r       Generate a .env.ll file (secure reference)",
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
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    
    frame.render_widget(paragraph, inner);
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
            (Mode::Normal, Modal::None) => "a: add | e: reveal | y: copy | d: delete | r: .env | h: help | q: quit",
        }
    };

    let style = if app.status_message.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    let helper = Paragraph::new(helper_text)
        .style(style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Shortcuts "));

    frame.render_widget(helper, area);
}