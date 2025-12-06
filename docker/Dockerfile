# =============================================================================
# Dockerfile for lazy-locker release testing
# 
# This image installs the LATEST RELEASED versions from:
#   - crates.io (Rust CLI)
#   - PyPI (Python SDK)
#   - npm (JavaScript/TypeScript SDK)
#
# Usage:
#   docker build -t lazy-locker-test ./docker
#   docker run -it lazy-locker-test
#
# Then inside the container:
#   1. lazy-locker              # Initialize with passphrase, create secrets
#   2. Create test secrets: test, test2, MY_API_KEY, DB_PASSWORD
#   3. python test_env.py       # Test Python SDK
#   4. bun run test_env.ts      # Test TypeScript SDK
# =============================================================================

FROM debian:bookworm-slim

# Avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive
ENV TERM=xterm-256color

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    build-essential \
    pkg-config \
    python3 \
    python3-pip \
    python3-venv \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# =============================================================================
# Install Rust (nightly for edition 2024 support)
# =============================================================================
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"

# Install lazy-locker from crates.io (latest release)
RUN cargo install lazy-locker

# =============================================================================
# Install Bun (JavaScript/TypeScript runtime)
# =============================================================================
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:${PATH}"

# =============================================================================
# Setup workspace
# =============================================================================
WORKDIR /workspace

# Create Python virtual environment and install lazy-locker SDK
RUN python3 -m venv /workspace/.venv
ENV PATH="/workspace/.venv/bin:${PATH}"
RUN pip install --upgrade pip && pip install lazy-locker

# Install lazy-locker JavaScript SDK
RUN bun init -y && bun add lazy-locker

# =============================================================================
# Create example test scripts (using installed packages from registries)
# =============================================================================

# Python test script
RUN cat > /workspace/test_env.py << 'PYEOF'
#!/usr/bin/env python3
"""
Test script for lazy-locker Python SDK (installed from PyPI).

Usage:
  1. Run lazy-locker and enter your passphrase (agent starts automatically)
  2. Create secrets: test, test2, MY_API_KEY, DB_PASSWORD
  3. python test_env.py
"""

import os
import sys
from lazy_locker import inject_secrets, is_agent_running, status

print("=" * 50)
print("Test du SDK lazy-locker pour Python")
print("=" * 50)

# Check if agent is active
if not is_agent_running():
    print("\n❌ Agent non démarré!")
    print("   Lancez 'lazy-locker' et entrez votre passphrase.")
    sys.exit(1)

# Show status
try:
    info = status()
    remaining_hours = info.get('ttl_remaining_secs', 0) // 3600
    remaining_mins = (info.get('ttl_remaining_secs', 0) % 3600) // 60
    print(f"\n✅ Agent actif (TTL: {remaining_hours}h {remaining_mins}m)")
except Exception as e:
    print(f"\n⚠️  Erreur statut: {e}")

# Inject secrets
try:
    count = inject_secrets()
    print(f"✅ {count} secrets injectés dans os.environ")
except Exception as e:
    print(f"\n❌ Erreur injection: {e}")
    sys.exit(1)

# Display secrets (masked)
print("\nSecrets disponibles:")
for key in ['test', 'test2', 'MY_API_KEY', 'DB_PASSWORD']:
    value = os.getenv(key)
    if value:
        masked = value[:3] + '*' * (len(value) - 3) if len(value) > 3 else '***'
        print(f"  {key} = {masked}")
    else:
        print(f"  {key} = (non défini)")

print("\n" + "=" * 50)
print("✅ Test Python SDK terminé avec succès!")
print("=" * 50)
PYEOF

# TypeScript test script
RUN cat > /workspace/test_env.ts << 'TSEOF'
/**
 * Test script for lazy-locker JavaScript/TypeScript SDK (installed from npm).
 *
 * Usage:
 *   1. Run lazy-locker and enter your passphrase (agent starts automatically)
 *   2. Create secrets: test, test2, MY_API_KEY, DB_PASSWORD
 *   3. bun run test_env.ts
 */

import { injectSecrets, isAgentRunning, status, getSecrets } from 'lazy-locker';

console.log('='.repeat(50));
console.log('Test du SDK lazy-locker pour JavaScript/TypeScript');
console.log('='.repeat(50));

async function main() {
  // Check if agent is active
  if (!(await isAgentRunning())) {
    console.log('\n❌ Agent non démarré!');
    console.log("   Lancez 'lazy-locker' et entrez votre passphrase.");
    process.exit(1);
  }

  // Show status
  try {
    const info = await status();
    const remainingHours = Math.floor(info.ttl_remaining_secs / 3600);
    const remainingMins = Math.floor((info.ttl_remaining_secs % 3600) / 60);
    console.log(`\n✅ Agent actif (TTL: ${remainingHours}h ${remainingMins}m)`);
  } catch (e) {
    console.log(`\n⚠️  Erreur statut: ${e}`);
  }

  // Inject secrets
  try {
    const count = await injectSecrets();
    console.log(`✅ ${count} secrets injectés dans process.env`);
  } catch (e) {
    console.log(`\n❌ Erreur injection: ${e}`);
    process.exit(1);
  }

  // Display secrets (masked)
  console.log('\nSecrets disponibles:');
  const secrets = await getSecrets();
  for (const [key, value] of Object.entries(secrets)) {
    const masked = value.length > 3 ? value.slice(0, 3) + '*'.repeat(value.length - 3) : '***';
    console.log(`  ${key} = ${masked}`);
  }

  console.log('\n' + '='.repeat(50));
  console.log('✅ Test TypeScript SDK terminé avec succès!');
  console.log('='.repeat(50));
}

main().catch(console.error);
TSEOF

# Make Python script executable
RUN chmod +x /workspace/test_env.py

# Default command - interactive bash
CMD ["bash"]
