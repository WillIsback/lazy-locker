# Getting Started with Lazy-Locker

This guide will walk you through setting up Lazy-Locker for the first time.

## Prerequisites

- **Operating System:** Linux (macOS support coming soon)
- **Rust toolchain** (for building from source)
- **Python 3.8+** (for Python SDK)
- **Node.js 16+** (for JavaScript SDK)

## Installation

### Building from Source

```bash
# Clone the repository
git clone https://github.com/WillIsback/lazy-locker.git
cd lazy-locker

# Build release version
cargo build --release

# Install globally (optional)
sudo cp target/release/lazy-locker /usr/local/bin/
```

### Installing SDKs

**Python SDK:**

```bash
# Using pip
pip install lazy-locker

# Using uv
uv add lazy-locker

# Using poetry
poetry add lazy-locker
```

**JavaScript/TypeScript SDK:**

```bash
# Using npm
npm install lazy-locker

# Using pnpm
pnpm add lazy-locker

# Using bun
bun add lazy-locker
```

## First Run

1. **Launch Lazy-Locker:**

   ```bash
   lazy-locker
   ```

2. **Create your passphrase:**

   On first run, you'll be prompted to create a passphrase. This passphrase:
   - Encrypts all your secrets locally
   - Is never stored in plain text
   - Uses Argon2id for secure key derivation

   **⚠️ Remember your passphrase! It cannot be recovered.**

3. **The agent starts automatically:**

   After entering your passphrase, an agent daemon starts in the background. This agent:
   - Stores the derived key in memory
   - Responds to SDK requests via Unix socket
   - Has an 8-hour TTL (time-to-live)

## Adding Your First Secret

1. In the TUI, press `a` to open the "Add Secret" modal
2. Enter the secret name (e.g., `MY_API_KEY`)
3. Press `Enter` or `Tab` to move to the value field
4. Enter the secret value in plain text
5. Optionally set an expiration (in days)
6. Press `Enter` to save

## Using Secrets in Your Code

### Python

```python
from lazy_locker import inject_secrets
import os

# Inject all secrets into os.environ
inject_secrets()

# Use your secrets
api_key = os.environ["MY_API_KEY"]
```

### JavaScript/TypeScript

```javascript
import { injectSecrets } from 'lazy-locker';

// Inject all secrets into process.env
await injectSecrets();

// Use your secrets
const apiKey = process.env.MY_API_KEY;
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate between secrets |
| `a` | Add new secret |
| `e` | Reveal/hide selected secret |
| `y` | Copy secret to clipboard |
| `d` | Delete selected secret |
| `:` | Open command modal |
| `h` | Show help |
| `q` | Quit |

### Export Commands

Press `:` to open the command modal, then type or select:

| Command | Description |
|---------|-------------|
| `:env` | Generate `.env` file (plain text) |
| `:bash` | Export to `~/.bashrc` |
| `:zsh` | Export to `~/.zshrc` |
| `:fish` | Export to fish config |
| `:json` | Export as JSON file |
| `:clear` | Remove exports from shell profiles |

## Next Steps

- Read the [Architecture](./architecture.md) guide
- Explore the [CLI Reference](./cli-reference.md)
- Check the [SDK Guide](./sdk-guide.md) for advanced usage
- Review [Security](./security.md) best practices
