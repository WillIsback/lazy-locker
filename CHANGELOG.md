# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.2] - 2025-12-06

### Fixed
- **TUI Agent Detection Bug** - Fixed issue where TUI would ask for passphrase even when agent was already active, then freeze on input ([#1](https://github.com/WillIsback/lazy-locker/issues/1))
- TUI now checks if agent is running before prompting for passphrase
- When agent is active, secrets are loaded directly from the agent

### Added
- **Tokyo Night Theme** - Beautiful new color scheme for the TUI interface
- **Comprehensive Test Suite** - 68 unit and integration tests covering:
  - Crypto module (encrypt/decrypt roundtrip, nonce uniqueness, error handling)
  - Store module (CRUD operations, expiration, persistence)
  - Executor module (token usage scanning, env generation)
  - App module (state management, modal navigation, key handling)
  - CLI integration tests

### Changed
- Improved visual consistency across all TUI components
- Better error handling in agent mode

## [0.0.1] - 2025-12-05

### Added
- Initial release
- Secure local secrets storage with AES-256-GCM encryption
- TUI interface with Ratatui
- Agent daemon for unlocked secrets access
- Python SDK (`lazy-locker-py`)
- Node.js SDK (`lazy-locker-js`)
- CLI commands: `init`, `agent start/stop/status`, `run`
- Token usage scanning in project files
- Secret expiration support
