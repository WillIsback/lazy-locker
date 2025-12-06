# CLI Reference

Complete reference for Lazy-Locker command-line interface.

## Synopsis

```bash
lazy-locker [COMMAND] [OPTIONS]
```

## Commands

### Default (TUI Mode)

```bash
lazy-locker
```

Opens the interactive Terminal User Interface for managing secrets.

### run

```bash
lazy-locker run <command>
```

Execute a command with secrets injected as environment variables.

**Examples:**

```bash
lazy-locker run python script.py
lazy-locker run uv run my_app.py
lazy-locker run node server.js
lazy-locker run bun run index.ts
```

**Behavior:**

1. If the agent is running, secrets are retrieved from it (no passphrase needed)
2. If the agent is not running, prompts for passphrase

### status

```bash
lazy-locker status
```

Display the current agent status.

**Output:**

```
✅ Agent active
   Uptime: 2h 15m
   TTL remaining: 5h 45m
```

Or if not running:

```
❌ Agent not started
   Run lazy-locker to start the agent
```

### stop

```bash
lazy-locker stop
```

Stop the agent daemon.

**Output:**

```
✅ Agent stopped
```

### help

```bash
lazy-locker help
lazy-locker --help
lazy-locker -h
```

Display help information.

## TUI Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |

### Secret Operations

| Key | Action |
|-----|--------|
| `a` | Add new secret |
| `e` | Reveal/hide selected secret value |
| `y` | Copy decrypted value to clipboard |
| `d` | Delete selected secret |

### General

| Key | Action |
|-----|--------|
| `h` | Show help modal |
| `q` | Quit application |
| `Esc` | Close modal / Cancel |

### Add Secret Modal

| Key | Action |
|-----|--------|
| `Tab` | Switch between fields |
| `Enter` | Next field / Confirm |
| `Esc` | Cancel |

### Delete Confirmation Modal

| Key | Action |
|-----|--------|
| `y` / `Y` | Confirm deletion |
| `n` / `N` / `Esc` | Cancel |

## Environment Variables

Lazy-Locker respects the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `HOME` | User home directory | System default |
| `XDG_CONFIG_HOME` | Config directory base | `~/.config` |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 127 | Command not found (in `run` mode) |

## Files

| Path | Description |
|------|-------------|
| `~/.config/.lazy-locker/salt` | Salt for key derivation |
| `~/.config/.lazy-locker/hash` | Passphrase hash |
| `~/.config/.lazy-locker/secrets.json` | Encrypted secrets |
| `~/.config/.lazy-locker/agent.sock` | Agent Unix socket |
| `~/.config/.lazy-locker/agent.pid` | Agent process ID |
