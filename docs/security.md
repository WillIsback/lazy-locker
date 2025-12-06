# Security

This document describes the security model and best practices for Lazy-Locker.

## Cryptographic Design

### Key Derivation

Lazy-Locker uses **Argon2id** for key derivation:

- **Algorithm:** Argon2id (memory-hard, resistant to GPU attacks)
- **Output:** 256-bit key
- **Salt:** Random 128-bit salt per locker

The derived key is used for all encryption operations and is never stored on disk.

### Encryption

All secrets are encrypted with **AES-256-GCM**:

- **Algorithm:** AES-256-GCM (authenticated encryption)
- **Key:** 256-bit derived from passphrase
- **Nonce:** Random 96-bit nonce per encryption

GCM provides both confidentiality and integrity protection.

### Random Number Generation

All random values are generated using the operating system's cryptographically secure random number generator via `OsRng`.

## Memory Security

### Zeroization

Sensitive data is zeroized (overwritten with zeros) when no longer needed:

- Passphrase after key derivation
- Derived key on locker drop
- Decrypted secret values after use
- Agent state on shutdown

This is implemented using the `zeroize` crate.

### Agent Isolation

The agent daemon:

- Runs as a separate process
- Communicates only via Unix socket
- Socket has restrictive permissions (0600)
- Only the user who started it can connect

## Threat Model

### Protected Against

| Threat | Protection |
|--------|------------|
| Plain-text secrets on disk | AES-256-GCM encryption |
| Secrets in version control | Encrypted storage |
| Brute-force passphrase attacks | Argon2id key derivation |
| Tampering with encrypted data | GCM authentication |
| Memory leaks | Zeroization |

### Not Protected Against

| Threat | Reason |
|--------|--------|
| Root/administrator access | Can read process memory |
| Memory forensics | Key is in memory while agent runs |
| Keyloggers | Can capture passphrase |
| Malware on the same machine | Can impersonate user |
| Physical access | Can extract keys from running system |

## Best Practices

### Passphrase

1. Use a strong, unique passphrase (16+ characters)
2. Never reuse your passphrase elsewhere
3. Consider using a passphrase manager
4. Don't share your passphrase

### Secrets Management

1. Set expiration dates for temporary credentials
2. Rotate secrets regularly
3. Remove unused secrets
4. Use descriptive names

### System Security

1. Keep your system updated
2. Use full-disk encryption
3. Lock your screen when away
4. Monitor for unauthorized access

### Development

1. Never log secret values
2. Don't commit secrets to version control
3. Use `.gitignore` for sensitive files
4. Review code for accidental secret exposure

## Comparison with Alternatives

| Feature | Lazy-Locker | .env files | HashiCorp Vault |
|---------|-------------|------------|-----------------|
| Encryption at rest | ✅ AES-256-GCM | ❌ Plain text | ✅ Yes |
| Local-first | ✅ Yes | ✅ Yes | ❌ Server-based |
| No infrastructure | ✅ Yes | ✅ Yes | ❌ Requires server |
| SDK support | ✅ Python, JS | ✅ dotenv | ✅ Multiple |
| Expiration | ✅ Yes | ❌ No | ✅ Yes |
| Access control | ✅ Passphrase | ❌ None | ✅ Policies |
| Audit logging | ❌ No | ❌ No | ✅ Yes |

## Reporting Security Issues

If you discover a security vulnerability, please:

1. **Do not** open a public issue
2. Email security concerns to: <william.derue@gmail.com>
3. Include detailed reproduction steps
4. Allow reasonable time for a fix before disclosure

## Audit Status

Lazy-Locker has not undergone a formal security audit. Use at your own risk for sensitive production workloads.

The cryptographic primitives used (Argon2, AES-GCM) are well-established and implemented by widely-used Rust crates:

- `argon2` - RustCrypto implementation
- `aes-gcm` - RustCrypto implementation
- `rand_core` - RustCrypto random number generation
