# SDK Guide

This guide covers the Python and JavaScript/TypeScript SDKs for Lazy-Locker.

## Prerequisites

Before using the SDKs, make sure:

1. Lazy-Locker is installed
2. You've run `lazy-locker` and entered your passphrase (agent is running)
3. You've added secrets via the TUI

## Python

### Installation

```bash
pip install lazy-locker
```

Or with other package managers:

```bash
uv add lazy-locker
poetry add lazy-locker
pipenv install lazy-locker
```

### Basic Usage

```python
from lazy_locker import inject_secrets
import os

# Inject all secrets into os.environ
inject_secrets()

# Use your secrets
api_key = os.environ["MY_API_KEY"]
database_url = os.environ["DATABASE_URL"]
```

### API Reference

#### `inject_secrets(prefix="", override=True) -> int`

Injects all secrets into `os.environ`.

**Parameters:**

- `prefix` (str): Optional prefix for environment variable names
- `override` (bool): If True, overwrite existing environment variables

**Returns:** Number of secrets injected

**Example:**

```python
# Inject with prefix
inject_secrets(prefix="APP_")
# Access as os.environ["APP_MY_API_KEY"]

# Don't override existing variables
inject_secrets(override=False)
```

#### `get_secrets() -> Dict[str, str]`

Returns all secrets as a dictionary.

```python
from lazy_locker import get_secrets

secrets = get_secrets()
print(secrets)  # {"MY_API_KEY": "xxx", "DB_PASSWORD": "yyy"}
```

#### `get_secret(name: str) -> Optional[str]`

Returns a specific secret value.

```python
from lazy_locker import get_secret

api_key = get_secret("MY_API_KEY")
if api_key:
    print("Got API key")
```

#### `is_agent_running() -> bool`

Checks if the Lazy-Locker agent is running.

```python
from lazy_locker import is_agent_running

if not is_agent_running():
    print("Please run lazy-locker first")
```

#### `status() -> dict`

Returns agent status information.

```python
from lazy_locker import status

info = status()
print(f"Uptime: {info['uptime_secs']}s")
print(f"TTL remaining: {info['ttl_remaining_secs']}s")
```

### Migration from python-dotenv

```python
# Before (python-dotenv)
from dotenv import load_dotenv
load_dotenv()

# After (lazy-locker)
from lazy_locker import inject_secrets
inject_secrets()

# The rest of your code stays the same!
```

## JavaScript / TypeScript

### Installation

```bash
npm install lazy-locker
```

Or with other package managers:

```bash
pnpm add lazy-locker
bun add lazy-locker
yarn add lazy-locker
```

### Basic Usage

```typescript
import { injectSecrets } from 'lazy-locker';

// Inject all secrets into process.env
await injectSecrets();

// Use your secrets
const apiKey = process.env.MY_API_KEY;
const dbUrl = process.env.DATABASE_URL;
```

### One-Liner Config

```typescript
// At the top of your entry file
import 'lazy-locker/config';

// Secrets are now in process.env
```

### API Reference

#### `injectSecrets(options?) -> Promise<number>`

Injects all secrets into `process.env`.

**Options:**

- `prefix` (string): Optional prefix for environment variable names
- `override` (boolean): If true, overwrite existing variables (default: true)

**Returns:** Promise resolving to number of secrets injected

```typescript
// With options
await injectSecrets({ prefix: 'APP_', override: false });
```

#### `getSecrets() -> Promise<Record<string, string>>`

Returns all secrets as an object.

```typescript
import { getSecrets } from 'lazy-locker';

const secrets = await getSecrets();
console.log(secrets); // { MY_API_KEY: "xxx", DB_PASSWORD: "yyy" }
```

#### `getSecret(name: string) -> Promise<string | undefined>`

Returns a specific secret value.

```typescript
import { getSecret } from 'lazy-locker';

const apiKey = await getSecret('MY_API_KEY');
```

#### `isAgentRunning() -> Promise<boolean>`

Checks if the agent is running.

```typescript
import { isAgentRunning } from 'lazy-locker';

if (!(await isAgentRunning())) {
  console.log('Please run lazy-locker first');
  process.exit(1);
}
```

#### `status() -> Promise<{ uptime_secs: number; ttl_remaining_secs: number }>`

Returns agent status.

```typescript
import { status } from 'lazy-locker';

const info = await status();
console.log(`TTL remaining: ${info.ttl_remaining_secs}s`);
```

### Migration from dotenv

```typescript
// Before (dotenv)
import 'dotenv/config';

// After (lazy-locker)
import 'lazy-locker/config';

// The rest of your code stays the same!
```

## Error Handling

Both SDKs throw errors when the agent is not running:

**Python:**

```python
from lazy_locker import inject_secrets

try:
    inject_secrets()
except ConnectionError as e:
    print(f"Agent not running: {e}")
```

**JavaScript:**

```typescript
import { injectSecrets } from 'lazy-locker';

try {
  await injectSecrets();
} catch (error) {
  console.error('Agent not running:', error.message);
}
```

## Best Practices

1. **Check agent status** at application startup
2. **Handle errors gracefully** - provide helpful messages
3. **Don't log secrets** - avoid printing secret values
4. **Use prefixes** if you have naming conflicts
5. **Set override=false** to respect existing environment variables
