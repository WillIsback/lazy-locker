//! Integration tests for lazy-locker CLI.
//!
//! These tests verify end-to-end CLI functionality.
//! The TUI is interactive and would block tests, so we test CLI commands.
//!
//! NOTE: Most logic is tested via inline unit tests in src/.
//! These integration tests focus on CLI behavior.

use std::process::Command;

/// Helper to run lazy-locker CLI commands
fn run_lazy_locker(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lazy-locker"))
        .args(args)
        .output()
        .expect("Failed to execute lazy-locker")
}

// ============================================================================
// CLI Help tests
// ============================================================================

#[test]
fn test_help_command() {
    let output = run_lazy_locker(&["help"]);
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    assert!(stdout.contains("lazy-locker") || stdout.contains("Usage"));
}

#[test]
fn test_help_flag() {
    let output = run_lazy_locker(&["--help"]);
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    assert!(stdout.contains("lazy-locker") || stdout.contains("Usage"));
}

#[test]
fn test_short_help_flag() {
    let output = run_lazy_locker(&["-h"]);
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    assert!(stdout.contains("lazy-locker") || stdout.contains("Usage"));
}

// ============================================================================
// CLI Status tests
// ============================================================================

#[test]
fn test_status_command_runs() {
    let output = run_lazy_locker(&["status"]);
    
    // Should either show agent status or error (not crash)
    // Exit code 0 means agent running, non-zero means not running
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should contain some status info
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("Agent") || 
        combined.contains("agent") || 
        combined.contains("not running") ||
        combined.contains("active"),
        "Status output should mention agent state"
    );
}

// ============================================================================
// File structure tests
// ============================================================================

#[test]
fn test_binary_exists() {
    let binary_path = env!("CARGO_BIN_EXE_lazy-locker");
    assert!(
        std::path::Path::new(binary_path).exists(),
        "Binary should exist at {}",
        binary_path
    );
}
