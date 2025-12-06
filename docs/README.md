# Lazy-Locker Documentation

Welcome to the Lazy-Locker documentation! This guide will help you get started with the secure local secrets manager.

## Table of Contents

1. [Getting Started](./getting-started.md)
2. [Architecture](./architecture.md)
3. [CLI Reference](./cli-reference.md)
4. [SDK Guide](./sdk-guide.md)
5. [Security](./security.md)

## Quick Links

- [Installation](#installation)
- [Python SDK](./sdk-guide.md#python)
- [JavaScript/TypeScript SDK](./sdk-guide.md#javascript-typescript)
- [Troubleshooting](./troubleshooting.md)

## What is Lazy-Locker?

Lazy-Locker is a secure local secrets manager designed to replace plain-text `.env` files. It provides:

- **AES-256-GCM encryption** for all secrets at rest
- **Argon2id key derivation** for passphrase-based security
- **Agent-based architecture** for seamless SDK integration
- **Terminal UI (TUI)** for easy secret management
- **Python and JavaScript SDKs** for direct integration

## Installation

### From Source

```bash
git clone https://github.com/WillIsback/lazy-locker.git
cd lazy-locker
cargo build --release
sudo cp target/release/lazy-locker /usr/local/bin/
```

### SDKs

**Python:**

```bash
pip install lazy-locker
# or
uv add lazy-locker
```

**JavaScript/TypeScript:**

```bash
npm install lazy-locker
# or
pnpm add lazy-locker
# or
bun add lazy-locker
```

## Quick Start

1. **Initialize and start the agent:**

   ```bash
   lazy-locker
   ```

   Enter your passphrase when prompted. The agent will start automatically (8h TTL).

2. **Add secrets** using the TUI (press `a`)

3. **Use in your code:**

   **Python:**

   ```python
   from lazy_locker import inject_secrets
   inject_secrets()
   
   import os
   api_key = os.environ["MY_API_KEY"]
   ```

   **JavaScript:**

   ```javascript
   import { injectSecrets } from 'lazy-locker';
   await injectSecrets();
   
   const apiKey = process.env.MY_API_KEY;
   ```

4. **Run your scripts normally** - no wrapper needed!

   ```bash
   python my_script.py
   uv run my_script.py
   bun run my_script.ts
   ```

## License

MIT License - see [LICENSE](../LICENSE) for details.
