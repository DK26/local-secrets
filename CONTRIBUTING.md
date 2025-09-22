# Contributing to local-secrets

Thanks for your interest in contributing! 🔐

## Quick Start

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/local-secrets.git`
3. Test locally:
   - Linux/macOS/WSL: `bash ci-local.sh`
   - Windows PowerShell: `.\ci-local.ps1`
4. Submit a pull request

## How to Contribute

- 🐛 Bug reports: [Open an issue](https://github.com/DK26/local-secrets/issues) with reproduction steps
- 💡 Features: Discuss in an issue before implementing (see [Feature Suggestions](#feature-suggestions) below)
- 📝 Docs: Fix typos, add examples, improve clarity
- 🔧 Code: Bug fixes and security improvements welcome

## Feature Suggestions

Before suggesting features, **have your LLM agent read our [`AGENTS.md`](./AGENTS.md) and security documentation** and ask it:

1. Does my suggested feature align with the project's security-first design philosophy?
2. Why might this feature not already be implemented?
3. How does this fit within existing CLI patterns and security constraints?
4. Does this introduce any new attack surfaces?

**LLM Prompt:**
```
I want to suggest a feature for local-secrets CLI. Please read the AGENTS.md file from this repository and tell me if my feature idea aligns with the security-first design philosophy and why it might not already be implemented.
```

**Timeline expectations:**
- **Within security philosophy:** May be added in minor releases
- **Outside security philosophy:** Requires major security review (potentially far future unless critical)
- **New attack surfaces:** Requires comprehensive security analysis

We encourage **all** suggestions! The distinction just helps set implementation expectations. If you want to suggest security model changes, create an issue for discussion.

## Development

**Project Philosophy:**
- Security-by-design with explicit injection patterns
- Zero plaintext storage (all secrets via OS keyring)
- Defensive programming - validate ALL inputs
- Memory safety with automatic zeroization
- Cross-platform keyring compatibility

## Security Requirements

**CRITICAL: All code MUST follow defensive programming principles:**

### Input Validation (Required)
- **Validate ALL inputs** at function boundaries using `src/security.rs` functions
- Use `validate_env_var_name()` for environment variable names
- Use `validate_secret_value()` for secret content
- Check for injection patterns: `$()`, backticks, `&`, `|`, path traversal

### Memory Safety (Required)  
- Use `SecretString` for ALL sensitive data, never plain `String`
- Call `zeroize()` on temporary copies of sensitive data
- Clear temporary variables containing secrets before function returns

### Error Handling (Required)
- **Never panic** on user input - always return `Result<T, E>`
- Use `anyhow::Context` to add meaningful context to errors
- Provide clear, actionable error messages without information leakage

## Testing

Just run the CI script locally:

```bash
# Linux/macOS/WSL
bash ci-local.sh

# Windows PowerShell  
.\ci-local.ps1
```

**Our CI includes:**
- ✅ **Fail-fast formatting** - Auto-formats code before linting to save time
- ✅ **Zero-tolerance linting** - `clippy -- -D warnings` (no warnings allowed)
- ✅ **Comprehensive test suite** - Unit tests, security tests, integration tests
- ✅ **Real keyring testing** - End-to-end validation with actual OS keyring
- ✅ **Security audit** - `cargo audit` for known vulnerabilities

If it passes locally, your code is ready for review.

## Security Testing

**CRITICAL: All new functionality must include security tests:**

1. **Input Validation Tests** - Test malicious patterns in `tests/security_tests.rs`
2. **Memory Safety Tests** - Verify proper `SecretString` usage and zeroization
3. **Integration Tests** - Real keyring functionality in `tests/keyring_integration_tests.rs`

**Test malicious inputs thoroughly:**
- Command injection: `"$(rm -rf /)"`, `"; cat /etc/passwd"`  
- Path traversal: `"../../../etc/passwd"`, `"..\\..\\windows\\system32"`
- Unicode attacks: Null bytes, control characters, mixed scripts
- Resource exhaustion: Very long strings, repeated patterns

## Code Style

**Rust Standards:**
- Four-space indentation, `snake_case` for functions/variables
- `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants
- Use `anyhow::Result` for error handling with contextual messages
- Follow clippy suggestions with zero tolerance for warnings

**CLI Conventions:**
- Use `kebab-case` for command-line flags (`--no-save-missing`)
- Provide helpful error messages that guide users to solutions
- Support both subcommand mode and run mode for flexibility

## Local CI Scripts

Our CI scripts are optimized for developer productivity:

**Features:**
- 🚀 **Auto-format first** - Saves time by formatting before linting
- ⚡ **Fail-fast behavior** - Stops immediately on first failure
- 🔍 **Comprehensive testing** - All test types in correct order
- 🔑 **Real keyring validation** - Tests actual OS keyring functionality

The scripts match our GitHub Actions exactly, so if they pass locally, CI will pass.

## Security Vulnerability Handling

**If you discover a security vulnerability:**

1. **DO NOT** create a public issue
2. Email [dikaveman@gmail.com](mailto:dikaveman@gmail.com) with details
3. Include reproduction steps and potential impact assessment
4. We'll respond within 48 hours with remediation plan

**Common vulnerability areas to watch:**
- Input validation bypasses
- Command injection in environment variable names/values
- Memory leaks of sensitive data
- Race conditions in keyring operations
- Platform-specific attack vectors (Windows, macOS, Linux)

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project.

## Getting Help

- **Issues:** Bug reports and feature requests
- **Email:** [dikaveman@gmail.com](mailto:dikaveman@gmail.com)
- **Documentation:** Read `AGENTS.md` for comprehensive development guidance

Every contribution makes the ecosystem more secure! 🚀