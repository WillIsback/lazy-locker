mod app;
mod core;
mod event;
mod tui;
mod ui;

use anyhow::Result;
use app::{App, Field, Mode, Modal};
use core::agent::{self, AgentClient};
use core::executor;
use core::init::Locker;
use core::store::SecretsStore;
use crossterm::event::{Event, KeyCode};
use zeroize::Zeroize;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // Mode CLI
    if args.len() >= 2 {
        match args[1].as_str() {
            "run" if args.len() >= 3 => return run_with_secrets(&args[2..]),
            "agent" => return run_agent_mode(&args[2..]),
            "status" => return show_status(),
            "stop" => return stop_agent(),
            "help" | "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {}
        }
    }
    
    // Mode TUI standard
    run_tui()
}

fn print_help() {
    println!("lazy-locker - Secure secrets manager");
    println!();
    println!("USAGE:");
    println!("  lazy-locker              Opens the TUI interface");
    println!("  lazy-locker run <cmd>    Executes a command with injected secrets");
    println!("  lazy-locker status       Shows agent status");
    println!("  lazy-locker stop         Stops the agent");
    println!();
    println!("EXAMPLES:");
    println!("  lazy-locker run python script.py");
    println!("  lazy-locker run uv run app.py");
    println!("  lazy-locker run bun run index.ts");
}

/// Agent mode (called by the daemon)
fn run_agent_mode(args: &[String]) -> Result<()> {
    let mut key_hex = String::new();
    let mut store_path = String::new();
    
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--key" if i + 1 < args.len() => {
                key_hex = args[i + 1].clone();
                i += 2;
            }
            "--store" if i + 1 < args.len() => {
                store_path = args[i + 1].clone();
                i += 2;
            }
            _ => i += 1,
        }
    }
    
    if key_hex.is_empty() || store_path.is_empty() {
        return Err(anyhow::anyhow!("Usage: lazy-locker agent --key <key_hex> --store <path>"));
    }
    
    agent::run_agent(&key_hex, &store_path)
}

/// Shows agent status
fn show_status() -> Result<()> {
    match AgentClient::status() {
        Ok(data) => {
            println!("✅ Agent active");
            if let Some(uptime) = data.get("uptime_secs").and_then(|v| v.as_u64()) {
                let hours = uptime / 3600;
                let mins = (uptime % 3600) / 60;
                println!("   Uptime: {}h {:02}m", hours, mins);
            }
            if let Some(remaining) = data.get("ttl_remaining_secs").and_then(|v| v.as_u64()) {
                let hours = remaining / 3600;
                let mins = (remaining % 3600) / 60;
                println!("   TTL remaining: {}h {:02}m", hours, mins);
            }
        }
        Err(_) => {
            println!("❌ Agent not started");
            println!("   Run lazy-locker to start the agent");
        }
    }
    Ok(())
}

/// Stops the agent
fn stop_agent() -> Result<()> {
    let socket_path = agent::get_socket_path()?;
    if socket_path.exists() {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixStream;
        
        if let Ok(mut stream) = UnixStream::connect(&socket_path) {
            writeln!(stream, r#"{{"action":"shutdown"}}"#)?;
            stream.flush()?;
            
            let mut reader = BufReader::new(&stream);
            let mut response = String::new();
            reader.read_line(&mut response)?;
            
            println!("✅ Agent stopped");
        }
    } else {
        println!("ℹ️  Agent not started");
    }
    Ok(())
}

/// Executes a command with secrets injected as environment variables
fn run_with_secrets(command_args: &[String]) -> Result<()> {
    // First, try via the agent (no passphrase needed)
    if agent::is_agent_running() {
        let secrets = AgentClient::get_secrets()?;
        
        // Exécuter la commande avec les secrets
        use std::process::{Command, Stdio};
        let command = command_args.join(" ");
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .envs(&secrets)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;
        
        if !output.success() {
            std::process::exit(output.code().unwrap_or(1));
        }
        
        return Ok(());
    }
    
    // Fallback: ask for passphrase
    use std::io::Write;
    
    print!("Passphrase: ");
    std::io::stdout().flush()?;
    
    let passphrase = rpassword::read_password()?;
    
    let locker = Locker::init_or_load_with_passphrase(&passphrase)?;
    let key = locker.get_key()
        .ok_or_else(|| anyhow::anyhow!("Error loading key"))?;
    
    let store = SecretsStore::load(locker.base_dir(), key)?;
    
    let command = command_args.join(" ");
    let output = executor::execute_with_secrets(&command, &store, key)?;
    
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    
    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }
    
    Ok(())
}

fn run_tui() -> Result<()> {
    let mut terminal = tui::init()?;
    let mut app = App::new();
    let mut locker: Option<Locker> = None;
    let work_dir = std::env::current_dir()?;

    // Check locker init
    match Locker::try_new() {
        Ok(l) => {
            locker = Some(l);
            app.initialized = true;
            if let Some(ref l) = locker {
                if let Some(key) = l.get_key() {
                    app.secrets_store = Some(SecretsStore::load(l.base_dir(), key)?);
                }
            }
        }
        Err(_) => app.enter_init_mode(),
    }

    // Update usages at startup
    app.update_token_usages(&work_dir);

    loop {
        terminal.draw(|frame| ui::render(&app, frame))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                // Clear status message on any key press
                app.clear_status();

                let prev_selected = app.selected_index;
                
                // Handle special actions before general key handling
                let handled = match (&app.mode, &app.modal, key.code) {
                    // Passphrase validation
                    (Mode::InitPassphrase, _, KeyCode::Enter) => {
                        let passphrase_str = String::from_utf8_lossy(&app.passphrase);
                        match Locker::init_or_load_with_passphrase(&passphrase_str) {
                            Ok(l) => {
                                locker = Some(l);
                                app.initialized = true;
                                app.mode = Mode::Normal;
                                if let Some(ref l) = locker {
                                    if let Some(key) = l.get_key() {
                                        let store = SecretsStore::load(l.base_dir(), key)?;
                                        
                                        // Start agent in background if not already active
                                        if !agent::is_agent_running() {
                                            if let Err(e) = agent::start_daemon(key.to_vec(), store.clone()) {
                                                app.set_status(format!("⚠️ Agent: {}", e));
                                            } else {
                                                app.set_status("✅ Agent started (8h)".to_string());
                                            }
                                        } else {
                                            app.set_status("✅ Agent already active".to_string());
                                        }
                                        
                                        app.secrets_store = Some(store);
                                    }
                                }
                                app.passphrase.zeroize();
                                app.update_token_usages(&work_dir);
                            }
                            Err(e) => app.set_error(e.to_string()),
                        }
                        true
                    }
                    // Add secret - only validate when on Expiration field and Enter pressed
                    (Mode::Normal, Modal::AddSecret, KeyCode::Enter) if app.current_field == Field::Expiration => {
                        if !app.new_secret_name.is_empty() && !app.new_secret_value.is_empty() {
                            let expiration_days = app.get_expiration_days();
                            let name = app.new_secret_name.clone();
                            let value = app.new_secret_value.clone();
                            
                            if let Some(ref mut store) = app.secrets_store {
                                if let Some(ref l) = locker {
                                    if let Some(key) = l.get_key() {
                                        match store.add_secret(
                                            name,
                                            value,
                                            expiration_days,
                                            l.base_dir(),
                                            key,
                                        ) {
                                            Ok(_) => {
                                                app.new_secret_name.clear();
                                                app.new_secret_value.zeroize();
                                                app.new_secret_expiration.clear();
                                                app.close_modal();
                                                app.set_status("✓ Secret added successfully".to_string());
                                                app.update_token_usages(&work_dir);
                                            }
                                            Err(e) => app.set_error(e.to_string()),
                                        }
                                    }
                                }
                            }
                        }
                        true
                    }
                    // Delete confirmation
                    (Mode::Normal, Modal::DeleteConfirm, KeyCode::Char('y'))
                    | (Mode::Normal, Modal::DeleteConfirm, KeyCode::Enter) => {
                        if let Some(secret_name) = app.get_selected_secret_name() {
                            if let Some(ref mut store) = app.secrets_store {
                                if let Some(ref l) = locker {
                                    if let Some(key) = l.get_key() {
                                        match store.delete_secret(&secret_name, l.base_dir(), key) {
                                            Ok(_) => {
                                                let count = app.secrets_count();
                                                if count > 0 && app.selected_index >= count {
                                                    app.selected_index = count - 1;
                                                }
                                                app.close_modal();
                                                app.set_status("✓ Secret deleted".to_string());
                                                app.update_token_usages(&work_dir);
                                            }
                                            Err(e) => app.set_error(e.to_string()),
                                        }
                                    }
                                }
                            }
                        }
                        true
                    }
                    // Reveal secret with 'e'
                    (Mode::Normal, Modal::None, KeyCode::Char('e')) => {
                        if let Some(secret_name) = app.get_selected_secret_name() {
                            if app.revealed_secret.is_some() {
                                if let Some(ref mut revealed) = app.revealed_secret {
                                    revealed.zeroize();
                                }
                                app.revealed_secret = None;
                            } else {
                                if let Some(ref store) = app.secrets_store {
                                    if let Some(ref l) = locker {
                                        if let Some(key) = l.get_key() {
                                            match store.decrypt_secret(&secret_name, key) {
                                                Ok(decrypted) => {
                                                    app.revealed_secret = Some(decrypted);
                                                }
                                                Err(e) => app.set_error(e.to_string()),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        true
                    }
                    // Copy to clipboard with 'y'
                    (Mode::Normal, Modal::None, KeyCode::Char('y')) => {
                        if let Some(secret_name) = app.get_selected_secret_name() {
                            if let Some(ref store) = app.secrets_store {
                                if let Some(ref l) = locker {
                                    if let Some(key) = l.get_key() {
                                        match store.decrypt_secret(&secret_name, key) {
                                            Ok(mut decrypted) => {
                                                match executor::copy_to_clipboard(&decrypted) {
                                                    Ok(_) => {
                                                        app.set_status(format!("✓ '{}' copied to clipboard", secret_name));
                                                    }
                                                    Err(e) => app.set_error(format!("Clipboard error: {}", e)),
                                                }
                                                decrypted.zeroize();
                                            }
                                            Err(e) => app.set_error(e.to_string()),
                                        }
                                    }
                                }
                            }
                        }
                        true
                    }
                    // Generate .env.ll reference file with 'r'
                    (Mode::Normal, Modal::None, KeyCode::Char('r')) => {
                        if let Some(ref store) = app.secrets_store {
                            let env_path = work_dir.join(".env.ll");
                            match executor::generate_env_reference(store, &env_path) {
                                Ok(_) => {
                                    app.set_status(format!("✓ .env.ll file generated: {}", env_path.display()));
                                }
                                Err(e) => app.set_error(format!("Error generating .env.ll: {}", e)),
                            }
                        }
                        true
                    }
                    _ => false,
                };

                if !handled {
                    app.handle_key(key.code);
                }

                // Update usages if selection has changed
                if app.selected_index != prev_selected {
                    app.update_token_usages(&work_dir);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    tui::restore()?;
    println!("Closing Lazy Locker.");
    Ok(())
}