//! Agent daemon for lazy-locker.
//!
//! The agent stores the derived key in memory and responds to requests
//! from SDKs (Python, JS) via a Unix socket.
//!
//! Architecture:
//! - Socket: ~/.lazy-locker/agent.sock
//! - Protocol: Simple JSON over lines
//! - TTL: 8h by default, configurable

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::core::store::SecretsStore;

/// Default session duration (8 hours)
const DEFAULT_TTL_HOURS: u64 = 8;

/// Request sent to the agent
#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum AgentRequest {
    /// Ping to check if agent is alive
    #[serde(rename = "ping")]
    Ping,

    /// Request all decrypted secrets
    #[serde(rename = "get_secrets")]
    GetSecrets,

    /// Request a specific secret
    #[serde(rename = "get_secret")]
    GetSecret { name: String },

    /// List available secret names
    #[serde(rename = "list")]
    List,

    /// Stop the agent
    #[serde(rename = "shutdown")]
    Shutdown,
}

/// Agent response
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum AgentResponse {
    #[serde(rename = "ok")]
    Ok { data: serde_json::Value },

    #[serde(rename = "error")]
    Error { message: String },
}

/// Agent state in memory
struct AgentState {
    /// Decryption key (zeroized on shutdown)
    key: Vec<u8>,
    /// Secrets store
    store: SecretsStore,
    /// Startup timestamp
    started_at: Instant,
    /// TTL in hours
    ttl_hours: u64,
    /// Shutdown flag
    should_stop: bool,
}

impl Drop for AgentState {
    fn drop(&mut self) {
        // Clean up key in memory
        self.key.zeroize();
    }
}

/// Gets the agent socket path
pub fn get_socket_path() -> Result<PathBuf> {
    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Unable to determine user directories"))?;

    #[cfg(unix)]
    let sub_dir = ".lazy-locker";
    #[cfg(not(unix))]
    let sub_dir = "lazy-locker";

    let locker_dir = base_dirs.config_dir().join(sub_dir);
    Ok(locker_dir.join("agent.sock"))
}

/// Gets the agent PID file path
pub fn get_pid_path() -> Result<PathBuf> {
    let socket_path = get_socket_path()?;
    Ok(socket_path.with_extension("pid"))
}

/// Checks if the agent is running
pub fn is_agent_running() -> bool {
    let Ok(socket_path) = get_socket_path() else {
        return false;
    };
    if !socket_path.exists() {
        return false;
    }
    // Try connecting to verify
    let Ok(mut stream) = UnixStream::connect(&socket_path) else {
        return false;
    };
    let request = r#"{"action":"ping"}"#;
    if stream
        .write_all(format!("{}\n", request).as_bytes())
        .is_ok()
    {
        stream.flush().ok();
        let mut reader = BufReader::new(&stream);
        let mut response = String::new();
        if reader.read_line(&mut response).is_ok() {
            return response.contains("\"status\":\"ok\"");
        }
    }
    false
}

/// Starts the agent in daemon mode (fork)
pub fn start_daemon(key: Vec<u8>, store: SecretsStore) -> Result<()> {
    use std::process::Command;

    let socket_path = get_socket_path()?;
    let pid_path = get_pid_path()?;

    // Remove old socket if it exists
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    // Serialize key and store path for subprocess
    let key_hex = hex::encode(&key);
    let store_path = store.get_path().to_string_lossy().to_string();

    // Launch daemon in background
    let child = Command::new(std::env::current_exe()?)
        .arg("agent")
        .arg("--key")
        .arg(&key_hex)
        .arg("--store")
        .arg(&store_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    // Save PID
    std::fs::write(&pid_path, child.id().to_string())?;

    // Wait for socket to be ready
    for _ in 0..50 {
        if socket_path.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    Err(anyhow::anyhow!("Agent did not start in time"))
}

/// Agent mode entry point (called by the daemon)
pub fn run_agent(key_hex: &str, store_path: &str) -> Result<()> {
    let key = hex::decode(key_hex)?;
    let store = SecretsStore::load_from_path(&PathBuf::from(store_path), &key)?;

    let socket_path = get_socket_path()?;

    // Create Unix socket
    let listener = UnixListener::bind(&socket_path)?;

    // Set non-blocking to allow periodic shutdown checks
    listener.set_nonblocking(true)?;

    // Restrictive permissions on socket
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;
    }

    let state = Arc::new(Mutex::new(AgentState {
        key,
        store,
        started_at: Instant::now(),
        ttl_hours: DEFAULT_TTL_HOURS,
        should_stop: false,
    }));

    // TTL check thread
    let state_ttl = Arc::clone(&state);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let mut s = state_ttl.lock().unwrap();
            if s.started_at.elapsed() > Duration::from_secs(s.ttl_hours * 3600) {
                s.should_stop = true;
                break;
            }
            if s.should_stop {
                break;
            }
        }
    });

    // Main loop with non-blocking accept
    loop {
        // Check if we should stop first
        if state.lock().unwrap().should_stop {
            break;
        }

        match listener.accept() {
            Ok((stream, _)) => {
                let state_clone = Arc::clone(&state);
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream, state_clone) {
                        eprintln!("Client error: {}", e);
                    }
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No connection pending, sleep briefly then check again
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }

    // Cleanup
    std::fs::remove_file(&socket_path).ok();
    if let Ok(pid_path) = get_pid_path() {
        std::fs::remove_file(&pid_path).ok();
    }

    Ok(())
}

/// Handles a client connection
fn handle_client(stream: UnixStream, state: Arc<Mutex<AgentState>>) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;

    let mut line = String::new();
    reader.read_line(&mut line)?;

    let response = match serde_json::from_str::<AgentRequest>(&line) {
        Ok(request) => process_request(request, &state),
        Err(e) => AgentResponse::Error {
            message: format!("Invalid request: {}", e),
        },
    };

    let response_json = serde_json::to_string(&response)?;
    writeln!(writer, "{}", response_json)?;
    writer.flush()?;

    Ok(())
}

/// Processes a request
fn process_request(request: AgentRequest, state: &Arc<Mutex<AgentState>>) -> AgentResponse {
    let mut s = state.lock().unwrap();

    // Check TTL
    if s.started_at.elapsed() > Duration::from_secs(s.ttl_hours * 3600) {
        s.should_stop = true;
        return AgentResponse::Error {
            message: "Session expired".to_string(),
        };
    }

    match request {
        AgentRequest::Ping => AgentResponse::Ok {
            data: serde_json::json!({
                "uptime_secs": s.started_at.elapsed().as_secs(),
                "ttl_remaining_secs": (s.ttl_hours * 3600).saturating_sub(s.started_at.elapsed().as_secs()),
            }),
        },

        AgentRequest::GetSecrets => match s.store.decrypt_all(&s.key) {
            Ok(secrets) => AgentResponse::Ok {
                data: serde_json::to_value(secrets).unwrap_or_default(),
            },
            Err(e) => AgentResponse::Error {
                message: format!("Decryption error: {}", e),
            },
        },

        AgentRequest::GetSecret { name } => match s.store.decrypt_all(&s.key) {
            Ok(secrets) => {
                if let Some(value) = secrets.get(&name) {
                    AgentResponse::Ok {
                        data: serde_json::json!({ "value": value }),
                    }
                } else {
                    AgentResponse::Error {
                        message: format!("Secret '{}' not found", name),
                    }
                }
            }
            Err(e) => AgentResponse::Error {
                message: format!("Decryption error: {}", e),
            },
        },

        AgentRequest::List => {
            let names: Vec<String> = s
                .store
                .list_secrets()
                .iter()
                .map(|s| s.name.clone())
                .collect();
            AgentResponse::Ok {
                data: serde_json::to_value(names).unwrap_or_default(),
            }
        }

        AgentRequest::Shutdown => {
            s.should_stop = true;
            AgentResponse::Ok {
                data: serde_json::json!({ "message": "Agent stopped" }),
            }
        }
    }
}

/// Client for communicating with the agent
pub struct AgentClient;

impl AgentClient {
    /// Retrieves all secrets from the agent
    pub fn get_secrets() -> Result<HashMap<String, String>> {
        let socket_path = get_socket_path()?;
        let mut stream = UnixStream::connect(&socket_path)
            .map_err(|_| anyhow::anyhow!("Agent not started. Run lazy-locker first."))?;

        let request = r#"{"action":"get_secrets"}"#;
        writeln!(stream, "{}", request)?;
        stream.flush()?;

        let mut reader = BufReader::new(&stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        let resp: AgentResponse = serde_json::from_str(&response)?;
        match resp {
            AgentResponse::Ok { data } => Ok(serde_json::from_value(data)?),
            AgentResponse::Error { message } => Err(anyhow::anyhow!("{}", message)),
        }
    }

    /// Retrieves a specific secret
    #[allow(dead_code)]
    pub fn get_secret(name: &str) -> Result<String> {
        let socket_path = get_socket_path()?;
        let mut stream = UnixStream::connect(&socket_path)
            .map_err(|_| anyhow::anyhow!("Agent not started. Run lazy-locker first."))?;

        let request = serde_json::json!({"action": "get_secret", "name": name});
        writeln!(stream, "{}", request)?;
        stream.flush()?;

        let mut reader = BufReader::new(&stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        let resp: AgentResponse = serde_json::from_str(&response)?;
        match resp {
            AgentResponse::Ok { data } => Ok(data["value"].as_str().unwrap_or("").to_string()),
            AgentResponse::Error { message } => Err(anyhow::anyhow!("{}", message)),
        }
    }

    /// Checks agent status
    pub fn status() -> Result<serde_json::Value> {
        let socket_path = get_socket_path()?;
        let mut stream =
            UnixStream::connect(&socket_path).map_err(|_| anyhow::anyhow!("Agent not started"))?;

        let request = r#"{"action":"ping"}"#;
        writeln!(stream, "{}", request)?;
        stream.flush()?;

        let mut reader = BufReader::new(&stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        let resp: AgentResponse = serde_json::from_str(&response)?;
        match resp {
            AgentResponse::Ok { data } => Ok(data),
            AgentResponse::Error { message } => Err(anyhow::anyhow!("{}", message)),
        }
    }
}
