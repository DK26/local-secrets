# Copilot Instructions for local-secrets

## Project Overview

This is a minimalist Rust CLI for secure secret management using OS keyrings. The architecture prioritizes **security-by-design** with explicit injection patterns and zero plaintext storage.

## Core Architecture

### Backend Abstraction Pattern
- **Trait**: `SecretBackend` in `src/backend.rs` defines store/retrieve/delete operations
- **Production**: `KeyringBackend` uses OS keyring (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- **Testing**: `MemoryBackend` stores plaintext in temp files (requires `LOCAL_SECRETS_TEST_MODE=1`)

### Security-First Design
- All secrets wrapped in `SecretString` with automatic memory zeroization
- Input validation in `src/security.rs` prevents injection attacks
- Memory allocator uses `mimalloc` with secure features enabled
- No configuration files or plaintext storage

## Development Patterns

### Testing Strategy
```rust
// Always use memory backend for tests
cmd.env("LOCAL_SECRETS_BACKEND", "memory")
   .env("LOCAL_SECRETS_TEST_MODE", "1")
   .env("LOCAL_SECRETS_TEST_SECRET", "test_value")
```

### Security Validation
- Use `validate_env_var_name()` for all environment variable inputs
- Use `validate_secret_value()` for all secret content
- Check for command injection patterns: `$()`, backticks, `&`, `|`, path traversal

### Feature Gates
- `test-secret-param` feature enables `--test-secret` parameter for CI/CD
- Only available in test builds to prevent production usage

## Key Files

- `src/main.rs`: CLI parsing with `clap`, security validation entry point
- `src/backend.rs`: Backend trait and implementations (keyring vs memory)
- `src/commands.rs`: Core operations (store, delete, run with injection)
- `src/security.rs`: Input validation and attack prevention
- `tests/security_tests.rs`: Comprehensive security validation tests

## Development Workflows

### Building
```bash
cargo build --release  # Optimized for size with LTO
```

### Testing with Memory Backend
```bash
LOCAL_SECRETS_TEST_MODE=1 LOCAL_SECRETS_BACKEND=memory cargo test
```

### Integration Tests
Use `assert_cmd` for CLI testing with helper binaries in `target/test-bin/`

## Security Considerations

- **NEVER** use `MemoryBackend` in production (plaintext temp files)
- Always validate inputs through `src/security.rs` functions
- Memory zeroization is automatic via `SecretString` and explicit `zeroize()`
- Test malicious inputs thoroughly (see `tests/security_tests.rs` patterns)

## Code Conventions

- Error handling via `anyhow::Result` with contextual messages
- Security validation before any secret operations
- Explicit memory cleanup with `zeroize()` for sensitive data
- Defensive programming with input validation at boundaries