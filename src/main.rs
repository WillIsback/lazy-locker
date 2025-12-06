mod app;
mod core;
mod event;
mod tui;
mod ui;

use anyhow::Result;
use app::{App, Field, Modal, Mode};
use core::agent::{self, AgentClient};
use core::cli;
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
            "init" => return run_init_command(&args[2..]),
            "token" => return run_token_command(&args[2..]),
            "import" => return run_import_command(&args[2..]),
            "export" => return run_export_command(&args[2..]),
            "help" | "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" | "-V" | "version" => {
                println!("lazy-locker {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }

    // Mode TUI standard
    run_tui()
}

fn print_help() {
    println!(
        "lazy-locker {} - Secure secrets manager",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("  lazy-locker                    Opens the TUI interface");
    println!("  lazy-locker run <cmd>          Executes a command with injected secrets");
    println!("  lazy-locker status             Shows agent status");
    println!("  lazy-locker stop               Stops the agent");
    println!("  lazy-locker --version          Shows version");
    println!();
    println!("HEADLESS COMMANDS (for CI/CD):");
    println!("  lazy-locker init [OPTIONS]");
    println!("      --passphrase <PASS>        Passphrase (or set LAZY_LOCKER_PASSPHRASE)");
    println!("      --force                    Overwrite existing locker");
    println!();
    println!("  lazy-locker token add <NAME> [VALUE] [OPTIONS]");
    println!("      --stdin                    Read value from stdin");
    println!("      --expires <DAYS>           Expiration in days");
    println!("      --passphrase <PASS>        Passphrase");
    println!();
    println!("  lazy-locker token get <NAME> [OPTIONS]");
    println!("      --json                     Output as JSON");
    println!("      --env                      Output as KEY=VALUE");
    println!("      --passphrase <PASS>        Passphrase");
    println!();
    println!("  lazy-locker token list [OPTIONS]");
    println!("      --json                     Output as JSON");
    println!("      --env                      Output all as KEY=VALUE");
    println!("      --passphrase <PASS>        Passphrase");
    println!();
    println!("  lazy-locker token remove <NAME> [OPTIONS]");
    println!("      --passphrase <PASS>        Passphrase");
    println!();
    println!("  lazy-locker import [FILE] [OPTIONS]");
    println!("      --stdin                    Read from stdin");
    println!("      --format <env|json>        Input format (default: env)");
    println!("      --expires <DAYS>           Expiration for all imported tokens");
    println!("      --passphrase <PASS>        Passphrase");
    println!();
    println!("  lazy-locker export [OPTIONS]");
    println!("      --json                     Output as JSON");
    println!("      --env                      Output as .env format (default)");
    println!("      --passphrase <PASS>        Passphrase");
    println!();
    println!("EXAMPLES:");
    println!("  lazy-locker run python script.py");
    println!("  lazy-locker init --passphrase \"mypass\"");
    println!("  lazy-locker token add API_KEY \"sk-123\" --expires 30");
    println!("  echo \"secret\" | lazy-locker token add DB_PASS --stdin");
    println!("  lazy-locker import .env --passphrase \"mypass\"");
    println!("  lazy-locker token list --json");
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
        return Err(anyhow::anyhow!(
            "Usage: lazy-locker agent --key <key_hex> --store <path>"
        ));
    }

    agent::run_agent(&key_hex, &store_path)
}

// ============================================================================
// HEADLESS CLI COMMANDS
// ============================================================================

/// Parse CLI arguments helper
fn parse_cli_args(
    args: &[String],
) -> (
    Vec<String>,
    std::collections::HashMap<String, Option<String>>,
) {
    let mut positional = Vec::new();
    let mut flags: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with("--") {
            let flag_name = arg.trim_start_matches("--").to_string();
            // Check if next arg is a value (not another flag)
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                flags.insert(flag_name, Some(args[i + 1].clone()));
                i += 2;
            } else {
                flags.insert(flag_name, None);
                i += 1;
            }
        } else {
            positional.push(arg.clone());
            i += 1;
        }
    }

    (positional, flags)
}

/// init command
fn run_init_command(args: &[String]) -> Result<()> {
    let (_, flags) = parse_cli_args(args);

    let passphrase = cli::get_passphrase(flags.get("passphrase").and_then(|v| v.as_deref()))?;
    let force = flags.contains_key("force");

    cli::cmd_init(&passphrase, force)
}

/// token subcommands
fn run_token_command(args: &[String]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("Usage: lazy-locker token <add|get|list|remove> [OPTIONS]");
    }

    let subcommand = &args[0];
    let sub_args = &args[1..];
    let (positional, flags) = parse_cli_args(sub_args);

    let passphrase = cli::get_passphrase(flags.get("passphrase").and_then(|v| v.as_deref()))?;
    let format =
        cli::OutputFormat::from_args(flags.contains_key("json"), flags.contains_key("env"));

    match subcommand.as_str() {
        "add" => {
            let name = positional
                .first()
                .ok_or_else(|| anyhow::anyhow!("Usage: lazy-locker token add <NAME> [VALUE]"))?;
            let value = positional.get(1).map(|s| s.as_str());
            let stdin = flags.contains_key("stdin");
            let expires = flags
                .get("expires")
                .and_then(|v| v.as_ref())
                .and_then(|v| v.parse::<u32>().ok());

            cli::cmd_token_add(name, value, stdin, expires, &passphrase)
        }
        "get" => {
            let name = positional
                .first()
                .ok_or_else(|| anyhow::anyhow!("Usage: lazy-locker token get <NAME>"))?;

            cli::cmd_token_get(name, format, &passphrase)
        }
        "list" => cli::cmd_token_list(format, &passphrase),
        "remove" | "rm" | "delete" => {
            let name = positional
                .first()
                .ok_or_else(|| anyhow::anyhow!("Usage: lazy-locker token remove <NAME>"))?;

            cli::cmd_token_remove(name, &passphrase)
        }
        _ => anyhow::bail!(
            "Unknown token subcommand: {}. Use add, get, list, or remove.",
            subcommand
        ),
    }
}

/// import command
fn run_import_command(args: &[String]) -> Result<()> {
    let (positional, flags) = parse_cli_args(args);

    let passphrase = cli::get_passphrase(flags.get("passphrase").and_then(|v| v.as_deref()))?;
    let file = positional.first().map(|s| s.as_str());
    let stdin = flags.contains_key("stdin");
    let format = flags
        .get("format")
        .and_then(|v| v.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("env");
    let expires = flags
        .get("expires")
        .and_then(|v| v.as_ref())
        .and_then(|v| v.parse::<u32>().ok());

    cli::cmd_import(file, stdin, format, expires, &passphrase)
}

/// export command
fn run_export_command(args: &[String]) -> Result<()> {
    let (_, flags) = parse_cli_args(args);

    let passphrase = cli::get_passphrase(flags.get("passphrase").and_then(|v| v.as_deref()))?;
    let format = if flags.contains_key("json") {
        cli::OutputFormat::Json
    } else {
        cli::OutputFormat::Env
    };

    cli::cmd_export(format, &passphrase)
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

            // Wait for agent to fully stop (socket removed)
            for _ in 0..50 {
                if !socket_path.exists() && !agent::is_agent_running() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // Force remove socket if still exists
            if socket_path.exists() {
                std::fs::remove_file(&socket_path).ok();
            }

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
    let key = locker
        .get_key()
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
    // Stop agent if running - TUI needs direct access to locker for write operations
    // Agent will be restarted when exiting TUI
    let agent_was_running = agent::is_agent_running();
    if agent_was_running {
        let _ = stop_agent(); // Ignore errors
    }

    let mut terminal = tui::init()?;
    let mut app = App::new();
    let mut locker: Option<Locker> = None;
    let work_dir = std::env::current_dir()?;

    // Always require passphrase to enable full functionality (add/delete secrets)
    app.enter_init_mode();

    // Update usages at startup
    app.update_token_usages(&work_dir);

    loop {
        terminal.draw(|frame| ui::render(&app, frame))?;

        // Use 100ms poll timeout for better compatibility with various terminals (e.g., Ghostty)
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
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
                            if let Some(ref l) = locker
                                && let Some(key) = l.get_key()
                            {
                                let store = SecretsStore::load(l.base_dir(), key)?;

                                // Don't start agent during TUI session - will be started on exit
                                // This ensures TUI has exclusive write access to the store
                                app.set_status("✅ Locker unlocked".to_string());

                                app.secrets_store = Some(store);
                            }
                            app.passphrase.zeroize();
                            app.update_token_usages(&work_dir);
                        }
                        Err(e) => app.set_error(e.to_string()),
                    }
                    true
                }
                // Add secret - validate with Enter on Expiration field
                (Mode::Normal, Modal::AddSecret, KeyCode::Enter)
                    if app.current_field == Field::Expiration =>
                {
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
                                            app.set_status(
                                                "✓ Secret added successfully".to_string(),
                                            );
                                            app.update_token_usages(&work_dir);
                                        }
                                        Err(e) => app.set_error(e.to_string()),
                                    }
                                } else {
                                    app.set_error("Encryption key not available".to_string());
                                }
                            } else {
                                app.set_error("Locker not initialized".to_string());
                            }
                        } else {
                            app.set_error("Secrets store not loaded".to_string());
                        }
                    } else if app.new_secret_name.is_empty() {
                        app.set_error("Name is required".to_string());
                    } else {
                        app.set_error("Value is required".to_string());
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
                                } else {
                                    app.set_error("Encryption key not available".to_string());
                                }
                            } else {
                                app.set_error("Locker not initialized".to_string());
                            }
                        } else {
                            app.set_error("Secrets store not loaded".to_string());
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
                        } else if let Some(ref store) = app.secrets_store
                            && let Some(ref l) = locker
                            && let Some(key) = l.get_key()
                        {
                            match store.decrypt_secret(&secret_name, key) {
                                Ok(decrypted) => {
                                    app.revealed_secret = Some(decrypted);
                                }
                                Err(e) => app.set_error(e.to_string()),
                            }
                        }
                    }
                    true
                }
                // Copy to clipboard with 'y'
                (Mode::Normal, Modal::None, KeyCode::Char('y')) => {
                    if let Some(secret_name) = app.get_selected_secret_name()
                        && let Some(ref store) = app.secrets_store
                        && let Some(ref l) = locker
                        && let Some(key) = l.get_key()
                    {
                        match store.decrypt_secret(&secret_name, key) {
                            Ok(mut decrypted) => {
                                match executor::copy_to_clipboard(&decrypted) {
                                    Ok(_) => {
                                        app.set_status(format!(
                                            "✓ '{}' copied to clipboard",
                                            secret_name
                                        ));
                                    }
                                    Err(e) => app.set_error(format!("Clipboard error: {}", e)),
                                }
                                decrypted.zeroize();
                            }
                            Err(e) => app.set_error(e.to_string()),
                        }
                    }
                    true
                }
                // Command modal - execute command with Enter
                (Mode::Normal, Modal::Command, KeyCode::Enter) => {
                    if let Some(cmd) = app.get_selected_command() {
                        match cmd {
                            "env" => {
                                if let (Some(store), Some(l)) = (&app.secrets_store, &locker) {
                                    if let Some(key) = l.get_key() {
                                        let env_path = work_dir.join(".env");
                                        match executor::generate_env_file(store, key, &env_path) {
                                            Ok(_) => {
                                                app.set_status(format!(
                                                    "✓ .env generated: {}",
                                                    env_path.display()
                                                ));
                                            }
                                            Err(e) => app.set_error(format!("Error: {}", e)),
                                        }
                                    } else {
                                        app.set_error("Encryption key not available".to_string());
                                    }
                                } else {
                                    app.set_error("Locker not initialized".to_string());
                                }
                            }
                            "bash" | "zsh" | "fish" => {
                                if let (Some(store), Some(l)) = (&app.secrets_store, &locker) {
                                    if let Some(key) = l.get_key() {
                                        match executor::export_to_shell_profile(store, key, cmd) {
                                            Ok(path) => {
                                                app.set_status(format!(
                                                    "✓ Exported to {}",
                                                    path.display()
                                                ));
                                            }
                                            Err(e) => app.set_error(format!("Error: {}", e)),
                                        }
                                    } else {
                                        app.set_error("Encryption key not available".to_string());
                                    }
                                } else {
                                    app.set_error("Locker not initialized".to_string());
                                }
                            }
                            "json" => {
                                if let (Some(store), Some(l)) = (&app.secrets_store, &locker) {
                                    if let Some(key) = l.get_key() {
                                        let json_path = work_dir.join("secrets.json");
                                        match executor::export_to_json(store, key, &json_path) {
                                            Ok(_) => {
                                                app.set_status(format!(
                                                    "✓ JSON exported: {}",
                                                    json_path.display()
                                                ));
                                            }
                                            Err(e) => app.set_error(format!("Error: {}", e)),
                                        }
                                    } else {
                                        app.set_error("Encryption key not available".to_string());
                                    }
                                } else {
                                    app.set_error("Locker not initialized".to_string());
                                }
                            }
                            "clear" => match executor::clear_shell_exports() {
                                Ok(cleared) if !cleared.is_empty() => {
                                    let paths: Vec<_> = cleared
                                        .iter()
                                        .map(|p| {
                                            p.file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string()
                                        })
                                        .collect();
                                    app.set_status(format!(
                                        "✓ Cleared exports from: {}",
                                        paths.join(", ")
                                    ));
                                }
                                Ok(_) => {
                                    app.set_status("ℹ No exports found to clear".to_string());
                                }
                                Err(e) => app.set_error(format!("Error: {}", e)),
                            },
                            _ => {
                                app.set_error(format!("Unknown command: {}", cmd));
                            }
                        }
                        app.close_modal();
                    } else if !app.command_input.is_empty() {
                        app.set_error(format!("Unknown command: {}", app.command_input));
                        app.close_modal();
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

        if app.should_quit {
            break;
        }
    }

    tui::restore()?;

    // Start agent on exit if locker was initialized (for SDKs to use)
    if let Some(ref l) = locker
        && let Some(key) = l.get_key()
        && let Some(ref store) = app.secrets_store
        && !agent::is_agent_running()
    {
        match agent::start_daemon(key.to_vec(), store.clone()) {
            Ok(_) => println!("✅ Agent started (8h TTL)"),
            Err(e) => println!("⚠️ Could not start agent: {}", e),
        }
    }

    println!("Closing Lazy Locker.");
    Ok(())
}
