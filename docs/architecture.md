# Architecture

This document describes the internal architecture of Lazy-Locker.

## Overview

```
┌─────────────────────────────────────────────────────────┐
│                     TUI Application                      │
│                    (lazy-locker binary)                  │
└─────────────────────────────────────────────────────────┘
                          │
                          │ Starts on unlock
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    Agent Daemon                          │
│              (background process)                        │
│  - Stores derived key in memory                         │
│  - Responds via Unix socket                             │
│  - 8-hour TTL                                           │
│  Socket: ~/.config/.lazy-locker/agent.sock              │
└─────────────────────────────────────────────────────────┘
        ▲                    ▲                    ▲
        │ Socket             │ Socket             │ Socket
        │                    │                    │
   ┌────┴────┐          ┌────┴────┐          ┌────┴────┐
   │   CLI   │          │ Python  │          │   JS    │
   │   run   │          │   SDK   │          │   SDK   │
   └─────────┘          └─────────┘          └─────────┘
```

## Components

### 1. TUI Application

The main terminal user interface built with [Ratatui](https://ratatui.rs/).

**Responsibilities:**

- Display secrets list with expiration status
- Handle secret CRUD operations
- Manage passphrase input
- Start the agent daemon

**Key files:**

- `src/main.rs` - Entry point and CLI handling
- `src/app.rs` - Application state and logic
- `src/ui.rs` - UI rendering
- `src/tui.rs` - Terminal setup/teardown
- `src/event.rs` - Event handling

### 2. Agent Daemon

A background process that holds the derived encryption key in memory.

**Responsibilities:**

- Keep the key in memory (zeroized on shutdown)
- Respond to SDK requests via Unix socket
- Enforce TTL (8 hours by default)
- Decrypt secrets on demand

**Key files:**

- `src/core/agent.rs` - Agent implementation

**Protocol:**
The agent uses a simple JSON-over-newline protocol:

```json
// Request
{"action": "get_secrets"}

// Response
{"status": "ok", "data": {"MY_KEY": "value"}}
```

### 3. Secrets Store

Manages encrypted secret storage.

**Responsibilities:**

- Encrypt/decrypt secrets with AES-256-GCM
- Persist secrets to disk
- Track expiration dates

**Key files:**

- `src/core/store.rs` - Secret storage
- `src/core/crypto.rs` - Encryption primitives

**Storage location:** `~/.config/.lazy-locker/secrets.json`

### 4. Locker (Key Management)

Handles passphrase verification and key derivation.

**Responsibilities:**

- Derive encryption key from passphrase using Argon2id
- Verify passphrase on unlock
- Store salt and hash for verification

**Key files:**

- `src/core/init.rs` - Locker initialization

**Storage:**

- `~/.config/.lazy-locker/salt` - Salt for key derivation
- `~/.config/.lazy-locker/hash` - Passphrase hash for verification

### 5. SDKs

Client libraries for Python and JavaScript.

**Responsibilities:**

- Connect to agent via Unix socket
- Request secrets
- Inject into environment variables

**Key files:**

- `sdk/python/lazy_locker/__init__.py`
- `sdk/javascript/src/index.ts`

## Security Model

### Threat Model

**Protected against:**

- Plain-text secrets on disk
- Secrets in version control
- Unauthorized access to secrets file
- Memory leaks (zeroize on drop)

**Not protected against:**

- Root access on the machine
- Memory forensics while agent is running
- Keyloggers capturing passphrase

### Cryptographic Primitives

| Purpose | Algorithm |
|---------|-----------|
| Key Derivation | Argon2id |
| Encryption | AES-256-GCM |
| Random Generation | OS RNG (via `rand_core::OsRng`) |

### Data Flow

```
Passphrase
    │
    ▼ Argon2id
Derived Key (32 bytes)
    │
    ├──▶ Stored in Agent memory (zeroized on shutdown)
    │
    ▼ AES-256-GCM
Encrypted Secrets (on disk)
```

## File Structure

```
~/.config/.lazy-locker/
├── salt            # Salt for Argon2id key derivation
├── hash            # Passphrase hash for verification
├── secrets.json    # Encrypted secrets
├── agent.sock      # Unix socket for agent communication
└── agent.pid       # Agent process ID
```

## SDK Communication

SDKs communicate with the agent using Unix sockets:

1. SDK connects to `~/.config/.lazy-locker/agent.sock`
2. SDK sends JSON request with newline
3. Agent processes request
4. Agent sends JSON response with newline
5. SDK parses response and injects secrets

This architecture ensures secrets are never written to disk in plain text and are only decrypted in memory when needed.
