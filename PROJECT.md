# local-secrets

**GPL-3.0-only**  
A simple, cross-platform CLI to securely **store secrets in your OS keyring** and **inject them as environment variables** into child processes.  
Supports **Windows Credential Manager**, **Linux Secret Service**, and macOS Keychain (via the `keyring` crate).

---

## 🎯 Goals

- **Minimalist UX**: only three commands: `store`, `delete`, and the default run-mode.  
- **Secure storage**: secrets are encrypted at rest using the OS keyring service.  
- **Explicit injection**: you must specify `--env VAR` for each secret to inject.  
- **Controlled execution**: everything after `--` is passed as the binary + args to execute.  
- **Cross-platform**: works on Windows, Linux, and macOS.

---

## 📜 License

This project is licensed under **GPL-3.0-only**.  
Reason: GPL ensures derivative works remain open source and enforces reciprocity for security tooling.

---

## ⚙️ Dependencies (why chosen)

- [`clap`](https://crates.io/crates/clap) — ergonomic CLI parser; supports subcommands (`store`, `delete`) and run-mode flags.  
- [`anyhow`](https://crates.io/crates/anyhow) — clean error handling with context.  
- [`keyring`](https://crates.io/crates/keyring) — cross-platform OS keyring access (Credential Manager, Secret Service, Keychain).  
- [`rpassword`](https://crates.io/crates/rpassword) — hidden, no-echo password prompts.  
- [`secrecy`](https://crates.io/crates/secrecy) + [`zeroize`](https://crates.io/crates/zeroize) — secrets are stored in memory safely and scrubbed when dropped.

---

## 🛡️ Security principles

1. **No command-line secrets**: secrets are never passed via args; always prompted.  
2. **No plaintext files**: nothing written to disk; only keyring stores ciphertext.  
3. **Explicit injection**: you must specify `--env VAR` to inject secrets.  
4. **Scoped secrets**: only injected into the specified child process.  
5. **Prompt on missing**: if a secret isn’t in the keyring, you are prompted interactively. By default it is stored for next time, unless `--no-save-missing` is used.  
6. **Memory safety**: secrets use `SecretString` and `zeroize` to wipe memory after use.  

---

## 📂 Project Structure

```
local-secrets/
├── Cargo.toml
├── LICENSE (GPL-3.0-only)
├── README.md
└── src/
    └── main.rs
```

---

## 🖥️ CLI Overview

### Store a secret
```bash
local-secrets store GITHUB_PAT
Enter secret for GITHUB_PAT: ********
Stored secret for GITHUB_PAT.
```
- Uses `rpassword::read_password()` → no echo.  
- Stored under `(service="local-secrets", account="GITHUB_PAT")` in the OS keyring.

### Delete a secret
```bash
local-secrets delete GITHUB_PAT
Deleted GITHUB_PAT.
```
- Removes the entry from the OS keyring.

### Run a command with injection
```bash
local-secrets --env GITHUB_PAT -- codex --foo bar
Injecting env vars: ["GITHUB_PAT"]
```
- `--env VAR` → fetches secret from keyring.  
- If missing, prompts you and stores it (unless `--no-save-missing` is used).  
- Everything after `--` is treated as the binary and its arguments.  
- Example: above runs `codex --foo bar` with `GITHUB_PAT` injected.

### Run without storing missing secrets
```bash
local-secrets --env API_KEY --no-save-missing -- my-tool run
Enter secret for missing API_KEY: ********
```
- Prompts for input, injects into the child, but does not persist.

---

## 📦 Cargo.toml (annotated)

```toml
[package]
name = "local-secrets"
version = "0.1.0"
edition = "2021"
license = "GPL-3.0-only"

[dependencies]
anyhow = "1"                     # ergonomic error handling
clap = { version = "4", features = ["derive"] } # CLI parsing
keyring = "2"                    # cross-platform keyring access
rpassword = "7"                  # secure hidden password prompt
secrecy = "0.8"                  # secret wrapper for memory
zeroize = "1"                    # memory zeroing
```

---

## 🔑 Key Design Choices

- **No index.json**: we don’t track metadata. Only the OS keyring stores values.  
- **Explicit `--env` flags**: avoids “inject everything” accidents. You choose what to expose.  
- **Mandatory `--`**: everything after `--` is treated as the binary + args, preventing ambiguity.  
- **Prompt-on-missing**: makes first-time use seamless while ensuring secrets are captured securely.  
- **Optional `--no-save-missing`**: supports ephemeral secrets (CI/CD one-off runs).  

---

## 📝 Example Walkthrough

1. **First run (secret missing)**  
   ```bash
   $ local-secrets --env GITHUB_PAT -- codex
   Enter secret for missing GITHUB_PAT: ********
   Stored secret for GITHUB_PAT.
   Injecting env vars: ["GITHUB_PAT"]
   ```

2. **Subsequent run (secret already stored)**  
   ```bash
   $ local-secrets --env GITHUB_PAT -- codex
   Injecting env vars: ["GITHUB_PAT"]
   ```

3. **Ephemeral run (don’t store new secret)**  
   ```bash
   $ local-secrets --env API_KEY --no-save-missing -- my-tool
   Enter secret for missing API_KEY: ********
   Injecting env vars: ["API_KEY"]
   ```

4. **Rotate or delete**  
   ```bash
   $ local-secrets store GITHUB_PAT
   Enter secret for GITHUB_PAT: ********
   Stored secret for GITHUB_PAT.

   $ local-secrets delete GITHUB_PAT
   Deleted GITHUB_PAT.
   ```

---

## ⚠️ Security Caveats

- On Unix, processes with the same UID (or root) can still read envs from `/proc/<pid>/environ`.  
- On Windows, processes with the same user token can inspect environment blocks.  
- Secrets are **safer at rest**, but **not invisible at runtime** (same as any env-based injection).  
- Mitigation: use least-privilege, short-lived tokens. Prefer ephemeral mode in CI/CD.

---

## 📌 Conclusion

`local-secrets` provides a **minimalist, GPL-licensed tool** for managing secrets safely during local development:  
- Encryption-at-rest via OS keyrings.  
- Explicit, controlled injection.  
- No plaintext files.  
- No surprises — everything after `--` is your command.

