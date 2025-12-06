# lazy-locker Release Test Container

This Docker image provides a clean environment to manually test the **released versions** of lazy-locker from public registries:

- **Rust CLI**: `lazy-locker` from [crates.io](https://crates.io/crates/lazy-locker)
- **Python SDK**: `lazy-locker` from [PyPI](https://pypi.org/project/lazy-locker/)
- **JavaScript SDK**: `lazy-locker` from [npm](https://www.npmjs.com/package/lazy-locker)

## Build the Image

```bash
cd /path/to/lazy-locker
docker build -t lazy-locker-test -f docker/Dockerfile docker/
```

## Run Interactive Container

```bash
docker run -it lazy-locker-test
```

## Manual Testing Steps

Once inside the container:

### 1. Initialize lazy-locker

```bash
lazy-locker
```

- Enter a passphrase when prompted
- The TUI opens and the agent starts automatically

### 2. Create Test Secrets

In the TUI, press `a` to add secrets with these names (the values can be anything):

| Secret Name   | Example Value     |
|---------------|-------------------|
| `test`        | `my_test_value`   |
| `test2`       | `another_value`   |
| `MY_API_KEY`  | `sk-abc123`       |
| `DB_PASSWORD` | `super_secret`    |

Press `q` to quit the TUI (agent stays running in background).

### 3. Test Python SDK

```bash
python test_env.py
```

### 4. Test TypeScript SDK

```bash
bun run test_env.ts
```

## Verify Installed Versions

```bash
lazy-locker --help
pip show lazy-locker
bun pm ls | grep lazy-locker
```

## Notes

- The container uses Rust **nightly** toolchain (required for `edition = "2024"`)
- Python SDK runs in a virtual environment at `/workspace/.venv`
- All packages are installed from **public registries**, not from source
