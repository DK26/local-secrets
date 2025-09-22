# Repository Guidelines

## Project Structure & Module Organization
local-secrets is a single binary crate. `Cargo.toml` defines dependencies and CLI metadata. Place CLI wiring and command parsing in `src/main.rs`; factor reusable logic into modules under `src/` (e.g., `src/keyring.rs`, `src/inject.rs`). Reusable integration helpers or fixtures go under `tests/`. Keep documentation assets next to `README.md`.

## Build, Test, and Development Commands
Use `cargo fmt` to apply rustfmt defaults; CI treats formatting drift as failure. `cargo clippy -- -D warnings` ensures lint cleanliness before PRs. Run `cargo test` for unit and integration coverage. Validate the CLI end-to-end with `cargo run -- --help` or targeted scenarios like `cargo run -- --env GITHUB_PAT -- echo hi` to confirm flag parsing.

## Coding Style & Naming Conventions
Follow standard Rust style: four-space indentation, `snake_case` for functions and variables, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for env keys. Clap arguments should use `kebab-case` long flags mirroring the README (`--no-save-missing`). Prefer `anyhow::Context` for error chains and wrap secrets in `SecretString`; allow `zeroize` to scrub values on drop.

## Testing Guidelines
Leverage Rust's built-in test framework. Co-locate fast unit tests near the implementation with `#[cfg(test)]` modules; place end-to-end tests in `tests/` using temporary binaries or mock keyrings. Name tests after the behavior under check, e.g., `store_prompts_when_missing`. When touching keyring flows, gate tests behind feature flags or environment variables to avoid mutating real credentials.

## Commit & Pull Request Guidelines
Write commits in the imperative mood (`Add ephemeral injection flag`). Group logical changes and keep diffs focused. PRs should link any tracking issue, describe behavior changes, call out security considerations, and list the commands run (`cargo fmt`, `cargo test`). Include sample CLI output when altering prompts or flags so reviewers can see UX impact.

## Security & Configuration Tips
Never print secrets in logs or tests; sanitize with `SecretString::expose_secret()` only at the injection boundary. When creating examples, use placeholder names such as `GITHUB_PAT` instead of real tokens. Confirm platform-specific keyring availability before shipping features and document any new environment variables in `README.md`.

## Release Workflow
Push annotated tags that follow `v*` (for example `git tag -a v0.1.0 -m "Release v0.1.0"`); the CI workflow builds release binaries for Linux (`x86_64-unknown-linux-gnu`), macOS (`aarch64-apple-darwin`), and Windows (`x86_64-pc-windows-msvc`) after linting and tests pass. Artifacts are attached to the GitHub release along with a `SHA256SUMS` manifest. Locally, mirror the release build with `cargo build --release --target <triple>` before tagging, and smoke-test the CLI by running `target/<triple>/release/local-secrets --help`.
