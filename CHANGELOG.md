# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.6] - 2024-12-07

### Changed
- **Refactor**: Extracted token security analyzer into standalone crate `token-analyzer`
  - Now available on [crates.io](https://crates.io/crates/token-analyzer)
  - Separate GitHub repository: [WillIsback/token-analyzer](https://github.com/WillIsback/token-analyzer)
- **Dependencies**: Replaced local analyzer code with `token-analyzer = "0.0.1"` dependency
- **Cleaner codebase**: Removed ~1500 lines of code (now maintained in separate crate)

### Removed
- `src/core/analyzer.rs` - Now in `token-analyzer` crate
- `src/bin/token_analyzer.rs` - Binary now provided by `token-analyzer` crate
- `benches/analyzer_benchmark.rs` - Benchmarks moved to `token-analyzer` crate
- Direct dependencies: `ignore`, `regex`, `rayon`, `parking_lot` (now transitive via `token-analyzer`)

### Added
- Documentation linking to `token-analyzer` in README
- Badge for `token-analyzer` crate in security section


## [0.0.5] - 2025-12-06

### Added
- **CLI Headless Mode** - Full CLI support for CI/CD automation
  - `lazy-locker init --passphrase <PASS>` - Initialize locker non-interactively
  - `lazy-locker token add/get/list/remove` - Manage tokens via CLI
  - `lazy-locker import [FILE]` - Import from .env or JSON files
  - `lazy-locker export` - Export secrets to stdout
  - Support for `LAZY_LOCKER_PASSPHRASE` environment variable
  - `--json` and `--env` output formats for scripting
  - `--stdin` flag for secure value input
  - `--expires <DAYS>` for token expiration

- **CI/CD Pipeline** - GitHub Actions workflows
  - Automated linting and formatting checks
  - Version consistency validation across Cargo.toml and SDKs
  - Automated publishing to crates.io, PyPI, and npm
  - GitHub Releases with binary artifacts

- **Development Scripts**
  - `scripts/setup-dev.sh` - Setup development environment with pre-commit hooks
  - `scripts/pre-commit.sh` - Quality checks (fmt, clippy, version consistency)
  - `scripts/bump-version.sh` - Bump version across all project files
  - `scripts/release.sh` - Interactive release preparation

### Changed
- Extended `--help` output with all headless commands documentation

## [0.0.4] - 2025-12-06

### Added
- **Command Modal** - New vim-style command interface (press `:` to open)
  - `:env` - Generate `.env` file with secrets in plain text
  - `:bash` - Export secrets to `~/.bashrc`
  - `:zsh` - Export secrets to `~/.zshrc`
  - `:fish` - Export secrets to fish config
  - `:json` - Export secrets as JSON file
  - `:clear` - Remove lazy-locker exports from all shell profiles
- **Auto-completion** in command modal with arrow key navigation
- **Shell export markers** for easy cleanup (`# >>> lazy-locker exports >>>`)

### Fixed
- **Agent Shutdown** - Agent now properly stops when receiving shutdown signal
  - Changed from blocking to non-blocking socket listener
  - Agent checks for shutdown flag every 50ms
- **TUI/Agent Conflict** - TUI now stops agent on launch to ensure write access
  - Agent is restarted on TUI exit with fresh secrets store
  - Fixes issue where added/deleted secrets weren't persisted

### Changed
- Replaced `r` hotkey with `:` for command modal
- Improved documentation with new command references

## [0.0.3] - 2025-12-06

### Fixed
- **Ghostty Terminal Compatibility** - Fixed TUI not displaying on Ghostty terminal
  - Increased poll timeout from 16ms to 100ms for better terminal compatibility
  - Fixed raw mode initialization order (enable before alternate screen)
  - Added keyboard enhancement flags for better escape code handling
  - Added mouse capture support

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
