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

**Critical principle: Integration tests must cover all PROJECT.md scenarios completely.** If integration tests pass, the CLI works correctly for users. Manual testing indicates missing test coverage, not working functionality.

## Commit & Pull Request Guidelines
Write commits in the imperative mood (`Add ephemeral injection flag`). Group logical changes and keep diffs focused. PRs should link any tracking issue, describe behavior changes, call out security considerations, and list the commands run (`cargo fmt`, `cargo test`). Include sample CLI output when altering prompts or flags so reviewers can see UX impact.

## Security & Configuration Tips
Never print secrets in logs or tests; sanitize with `SecretString::expose_secret()` only at the injection boundary. When creating examples, use placeholder names such as `GITHUB_PAT` instead of real tokens. Confirm platform-specific keyring availability before shipping features and document any new environment variables in `README.md`.

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
