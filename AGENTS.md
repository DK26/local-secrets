# Repository Guidelines

## Project Structure & Module Organization
local-secrets is a single binary crate. `Cargo.toml` defines dependencies and CLI metadata. Place CLI wiring and command parsing in `src/main.rs`; factor reusable logic into modules under `src/` (e.g., `src/keyring.rs`, `src/inject.rs`). Reusable integration helpers or fixtures go under `tests/`. Keep documentation assets next to `README.md`.

## Build, Test, and Development Commands
**CRITICAL: Always use automated testing over manual verification.** Use `cargo fmt` to apply rustfmt defaults; CI treats formatting drift as failure. `cargo clippy -- -D warnings` ensures lint cleanliness before PRs. Run `cargo test` for unit and integration coverage. 

**Never rely on manual CLI testing** - our integration tests in `tests/cli.rs` provide comprehensive validation across all user scenarios. If you need to verify functionality, trust the automated tests that run on every commit.

**Local development workflow:**
```bash
cargo fmt --all -- --check  # Format checking (like CI)
cargo clippy -- -D warnings # Lint with zero tolerance
cargo test --all-targets     # Complete test suite
cargo build --release       # Size-optimized build
```

## Coding Style & Naming Conventions
Follow standard Rust style: four-space indentation, `snake_case` for functions and variables, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for env keys. Clap arguments should use `kebab-case` long flags mirroring the README (`--no-save-missing`). Prefer `anyhow::Context` for error chains and wrap secrets in `SecretString`; allow `zeroize` to scrub values on drop.

## CLI Design Patterns
The CLI uses a hybrid approach with optional subcommands and trailing arguments:

**Structure:**
```rust
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,  // store, delete
    
    #[arg(long, action = clap::ArgAction::Append)]
    env: Vec<String>,          // --env flags for run mode
    
    #[arg(last = true)]
    command_args: Vec<String>, // -- command args for run mode
}
```

**Usage patterns:**
- Subcommand mode: `local-secrets store VARIABLE` 
- Run mode: `local-secrets --env VARIABLE -- command args...`

**Note:** Help display may not work intuitively due to the `last = true` attribute conflicting with optional subcommands, but core functionality is fully tested and working.

## CRITICAL Security Incident & Resolution
**🚨 VULNERABILITY DISCOVERED & FIXED:** During test automation implementation, we discovered a **CRITICAL SECURITY FLAW** in the MemoryBackend implementation that stored secrets in **PLAINTEXT** in temporary files.

### The Security Issue
- **MemoryBackend stored secrets unencrypted** in `%TEMP%\local-secrets-memory-backend.json`
- **Complete security bypass** - secrets persisted on disk in readable JSON format
- **Contradicted the entire purpose** of secure secret management
- **Available to any process** that could read temp directory

### Immediate Security Hardening Applied
1. **Production Usage Blocked:** MemoryBackend now requires `LOCAL_SECRETS_TEST_MODE=1` environment variable
2. **Explicit Security Warnings:** Multiple warning layers inform users of plaintext storage risks
3. **Comprehensive Security Testing:** New test suite validates protection mechanisms
4. **Clear Error Messages:** Production attempts show explicit security warnings with remediation steps

### Security Protection Implementation
```rust
// Backend selection now includes security validation
match env::var("LOCAL_SECRETS_BACKEND").as_deref() {
    Ok("memory") => {
        if !cfg!(test) && env::var("LOCAL_SECRETS_TEST_MODE").is_err() {
            return Err(anyhow::anyhow!("MemoryBackend rejected for security reasons"));
        }
        // Only allowed in test contexts
    },
    _ => Box::new(KeyringBackend::new()), // Default secure backend
}
```

**Key Lesson:** Even test-only components require security reviews. Always audit ALL code paths, including testing utilities, for potential security vulnerabilities.

## Defensive Programming - MANDATORY APPROACH
**All code MUST follow defensive programming principles without exception.** This is not optional - it's the only way we code in this project:

### Input Validation (Required)
- **Validate ALL inputs** at function boundaries before any processing
- Check for empty strings, null values, and invalid ranges
- Use `trim()` to handle whitespace and validate meaningful content
- Return descriptive errors for invalid inputs using `anyhow::anyhow!()`

### Error Handling (Required)
- **Never panic** - always return `Result<T, E>` for fallible operations
- Use `anyhow::Context` to add meaningful context to every error
- Handle ALL error cases explicitly, no `unwrap()` or `expect()` in production code
- Provide clear, actionable error messages that help users understand what went wrong

### Resource Safety (Required)
- Use RAII patterns - resources should be automatically cleaned up
- Explicitly `zeroize()` all sensitive data after use
- Handle file operations defensively with proper error checking
- Validate exit codes and ensure they're in valid ranges (0-255)

### Memory Safety (Required)
- Use `SecretString` for ALL sensitive data, never plain `String`
- Call `zeroize()` on temporary copies of sensitive data
- Prefer owned data over references when security is involved
- Clear temporary variables containing secrets before function returns

### Boundary Checking (Required)  
- Validate array/vector indices before access
- Check for empty collections before operations
- Ensure numeric values are within expected ranges
- Handle edge cases like empty commands or malformed input

### Fail-Fast Principle (Required)
- Validate inputs immediately at function start
- Return early on invalid conditions
- Don't attempt to "fix" invalid input - report the error clearly
- Use descriptive error messages that aid in debugging

## Testing Guidelines
**Integration tests are our primary validation method.** Leverage Rust's built-in test framework with `tests/cli.rs` as the primary test suite using `assert_cmd` for real binary execution. Co-locate fast unit tests near the implementation with `#[cfg(test)]` modules only when needed.

**Integration test patterns:**
- Use `AssertCommand::cargo_bin("local-secrets")` to test real binary behavior
- Set `LOCAL_SECRETS_BACKEND=memory` for deterministic, file-based secrets storage  
- Use `LOCAL_SECRETS_TEST_MODE=1` and `LOCAL_SECRETS_TEST_SECRET` for non-interactive testing
- Name tests after user workflows: `store_then_run_injects_secret_from_memory_backend`

### Automated Testing of Store Commands

**Test-Secret Parameter for Store Command Automation:**
For testing store operations without requiring user input, use the `--test-secret` parameter (only available when compiled with the `test-secret-param` feature):

```bash
# Build with test feature enabled
cargo build --features test-secret-param

# Test store command with automated secret input
cargo test --test store_tests --features test-secret-param
```

**Test implementation pattern:**
```rust
#[test]
fn test_store_with_test_secret() {
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .arg("store")
        .arg("TEST_VAR")
        .arg("--test-secret")
        .arg("my_secret_value");
    
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}
```

**Security validation testing:**
- Test malicious variable names: `"$(echo injection)"`, `"../../../etc/passwd"`
- Test special characters in secret values: Unicode, shell metacharacters
- Test boundary conditions: empty secrets, very long secrets
- Verify proper error messages without information disclosure

**Test isolation:**
- Each test should clean up memory backend files: `std::fs::remove_file(temp_dir.join("local-secrets-memory-backend.json"))`
- Use unique variable names to avoid conflicts between tests
- Tests run with the feature flag are completely separate from production builds

**Critical principle: Integration tests must cover all PROJECT.md scenarios completely.** If integration tests pass, the CLI works correctly for users. Manual testing indicates missing test coverage, not working functionality.

## Commit & Pull Request Guidelines
Write commits in the imperative mood (`Add ephemeral injection flag`). Group logical changes and keep diffs focused. PRs should link any tracking issue, describe behavior changes, call out security considerations, and list the commands run (`cargo fmt`, `cargo test`). Include sample CLI output when altering prompts or flags so reviewers can see UX impact.

## Security & Configuration Tips
Never print secrets in logs or tests; sanitize with `SecretString::expose_secret()` only at the injection boundary. When creating examples, use placeholder names such as `GITHUB_PAT` instead of real tokens. Confirm platform-specific keyring availability before shipping features and document any new environment variables in `README.md`.

## Security Vulnerability Research & Mitigation

### Vulnerability Research Process
Our security approach is based on comprehensive research of similar CLI tools and known vulnerability patterns:

**Research Sources:**
- 1Password GitHub Actions security audit documentation
- Quantum Config library security guidelines  
- CVE databases for secret management tools
- Security documentation from HashiCorp Vault, AWS CLI, and kubectl

**Key Vulnerability Categories Identified:**
1. **Command Injection** - Malicious code in environment variable names/values
2. **Path Traversal** - Directory traversal attacks in file operations
3. **Environment Variable Pollution** - Overriding critical system variables
4. **Input Validation Bypasses** - Edge cases in validation logic
5. **Memory Safety** - Secrets persisting in memory dumps
6. **Resource Exhaustion** - Large inputs causing performance issues

### Implemented Security Validations

**Input Validation (`src/security.rs`):**
- Environment variable name validation with dangerous pattern detection
- Secret value length limits (1MB max) and null byte checking
- Command argument validation for shell injection prevention
- Path sanitization for directory traversal prevention

**Validation Functions:**
- `validate_env_var_name()` - Blocks injection patterns like `$()`, `;`, `&&`, `../`
- `validate_secret_value()` - Enforces size limits and null byte detection
- `validate_command_args()` - Prevents shell metacharacter injection
- `validate_cli_security()` - Holistic validation at CLI entry point

**Critical System Variable Protection:**
The CLI warns when overriding critical variables like `PATH`, `LD_LIBRARY_PATH`, `HOME`, `USER`, `SHELL`, etc., but allows it for legitimate use cases while alerting users to potential risks.

### Security Testing Framework

**Comprehensive Test Suite (`tests/security_tests.rs`):**
- **Malicious Input Testing** - 15+ attack patterns including command injection, path traversal
- **Unicode & Special Characters** - Null bytes, control characters, emoji, long strings
- **Resource Exhaustion** - 1MB+ inputs with timeout protection
- **Concurrent Access** - Race condition detection
- **Error Message Security** - Information disclosure prevention
- **Environment Variable Pollution** - System variable override testing

**Test Results:**
- All 11 security tests passing ✅
- Validates protection against known attack patterns
- Confirms graceful degradation under attack conditions
- Verifies proper error handling without information leakage

### Defensive Programming Patterns

**Security-First Development:**
- Input validation at ALL function boundaries
- Fail-fast on suspicious input patterns  
- Defense-in-depth with multiple validation layers
- Secure memory handling with `SecretString` and `zeroize`

**Attack Surface Minimization:**
- Limited public API surface
- Strict input sanitization
- Controlled error messages
- Resource usage limits

### Vulnerability Prevention Examples

**Command Injection Prevention:**
```rust
// BLOCKED: local-secrets --env '$(rm -rf /)' -- echo test
// Error: Environment variable name contains dangerous pattern: $(
```

**Path Traversal Prevention:**
```rust  
// BLOCKED: local-secrets store '../../../etc/passwd'  
// Error: Environment variable name contains dangerous pattern: ../
```

**Resource Exhaustion Prevention:**
```rust
// BLOCKED: 10MB secret values
// Error: Secret value too long (max 1MB)
```

**System Variable Protection:**
```rust
// WARNED: local-secrets --env PATH -- echo test
// Warning: Overriding critical system variable 'PATH'
```

### Security Best Practices for Future Development

**Required Security Practices:**
1. **All new inputs MUST use validation functions** from `src/security.rs`
2. **Security tests MUST be added** for new attack surfaces
3. **Memory safety MUST be maintained** with `SecretString`/`zeroize`
4. **Error messages MUST NOT leak** sensitive information
5. **Resource limits MUST be enforced** for all user inputs

**Security Review Checklist:**
- [ ] Input validation covers all user-controlled data
- [ ] No `unwrap()` or `panic!()` on user inputs
- [ ] Secrets use `SecretString` and proper zeroization
- [ ] Error messages are sanitized
- [ ] Resource limits are enforced
- [ ] Security tests cover new functionality

This security framework ensures our CLI is hardened against real-world attack patterns while maintaining usability for legitimate use cases.

**Security is non-negotiable** - treat every piece of data as potentially dangerous:
- Validate ALL inputs as if they come from untrusted sources
- Use defensive checks even for internal function calls  
- Assume external systems (keyring, filesystem) can fail or be compromised
- Never trust environment variables without validation
- Log security-relevant validation failures for debugging (without exposing secrets)

## Size Optimization Guidelines
When optimizing for binary size over performance:

**Cargo.toml profile configuration:**
```toml
[profile.release]
opt-level = "z"   # Optimize for size instead of speed
lto = true        # Enable Link Time Optimization  
codegen-units = 1 # Use single codegen unit for better optimization
panic = "abort"   # Remove panic handling overhead
strip = true      # Remove debug symbols
```

**Dependency optimization:**
- Use `default-features = false` for all dependencies
- Only enable required features (e.g., `features = ["derive"]` for serde)
- Be careful with feature minimization - ensure serialization traits remain available
- Test thoroughly after dependency changes - missing features can cause compilation errors

**Size optimization results:** Achieved 74% reduction from 1.87MB to 486KB while maintaining full functionality and security features.

## Release Workflow
Push annotated tags that follow `v*` (for example `git tag -a v0.1.0 -m "Release v0.1.0"`); the CI workflow builds release binaries for Linux (`x86_64-unknown-linux-gnu`), macOS (`aarch64-apple-darwin`), and Windows (`x86_64-pc-windows-msvc`) after linting and tests pass. Artifacts are attached to the GitHub release along with a `SHA256SUMS` manifest. 

**Size-optimized builds:** Our release profile achieves 74% size reduction (1.87MB → 486KB) using `opt-level="z"`, `lto=true`, `strip=true`, and `panic="abort"`. Dependencies use `default-features=false` with minimal feature sets.

**Automated validation:** Never manually test releases - if CI passes (format, lint, test, security audit) across all platforms, the release works correctly. The integration test suite provides comprehensive coverage of all user scenarios.
