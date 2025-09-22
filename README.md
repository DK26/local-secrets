# local-secrets

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

A **minimalist CLI tool** to securely **store secrets in your OS keyring** and inject them as environment variables into child processes.  
Designed to be **straight forward, easy to use, and secure** for local development and CI/CD.

- 🔐 **Encryption at rest** — secrets are stored in the OS keyring (Credential Manager, Secret Service, Keychain).
- 🎯 **Explicit injection** — you choose exactly which variables to expose via `--env`.
- 🚀 **No surprises** — everything after `--` is your binary + arguments.
- 🛡️ **Safe defaults** — hidden prompts, memory zeroization, no plaintext files.

---

## 🔒 Security

### Production Security
- **Default backend:** Uses OS keyring (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- **Memory protection:** Secrets wrapped in `SecretString` with automatic memory zeroization
- **No plaintext storage:** Secrets never stored in plain text files or logs
- **Input validation:** All inputs sanitized and validated against security threats

### Development Testing
For development and testing, a memory backend is available:
```bash
LOCAL_SECRETS_TEST_MODE=1 LOCAL_SECRETS_BACKEND=memory local-secrets store TEST_VAR
```

**⚠️ WARNING:** Memory backend stores secrets in **PLAINTEXT** temporary files and should **NEVER** be used in production. It's restricted to test contexts and requires explicit `LOCAL_SECRETS_TEST_MODE=1` activation.

---

## ✨ Quick Start

### Install (from source)

```bash
git clone https://github.com/yourname/local-secrets.git
cd local-secrets
cargo install --path .
```

> Requires Rust 1.70+ and a supported OS keyring backend.

---

## 💻 Usage

### 1. Store a secret
```bash
local-secrets store GITHUB_PAT
Enter secret for GITHUB_PAT: ********
Stored secret for GITHUB_PAT.
```

### 2. Run a program with injected secret
```bash
local-secrets --env GITHUB_PAT -- codex --foo bar
Injecting env vars: ["GITHUB_PAT"]
```

- `--env VAR` → tells `local-secrets` which secret to fetch from the keyring.
- If missing, you’ll be prompted and it will be stored for next time.
- Everything after `--` is passed as the binary + args.

### 3. Run without storing missing secrets
```bash
local-secrets --env API_KEY --no-save-missing -- my-tool run
Enter secret for missing API_KEY: ********
```

### 4. Delete a secret
```bash
local-secrets delete GITHUB_PAT
Deleted GITHUB_PAT.
```

---

## 🛡️ Why is this more secure?

- **Plain env vars** are often written into shell configs or registry in plaintext.  
- **local-secrets** stores secrets encrypted by your OS keyring (DPAPI, Keychain, Secret Service).  
- Secrets are only injected into the process you launch — not every shell session.  
- No config files, no `index.json`, no plaintext on disk.  

**Caveat:** once injected, secrets are still visible to the child process (and debuggers/root). This is inherent to any env-based injection.

---

## ⚙️ Configuration

No config files needed.  
Secrets are identified by the **variable name** you pass to `--env` or `store`.

---

## 🔑 Example Workflow

```bash
# Store once
local-secrets store GITHUB_PAT

# Use repeatedly
local-secrets --env GITHUB_PAT -- codex sync --verbose

# Rotate when needed
local-secrets store GITHUB_PAT

# Delete if no longer needed
local-secrets delete GITHUB_PAT
```

---

## 📦 Cargo.toml (core deps)

```toml
anyhow = "1"                     # error handling
clap = { version = "4", features = ["derive"] } # CLI parsing
keyring = "2"                    # OS keyring
rpassword = "7"                  # hidden input prompt
secrecy = "0.8"                  # secret wrapper
zeroize = "1"                    # memory zeroing
```

---

## 📜 License

This project is licensed under **GPL-3.0-only**.  
We want `local-secrets` (and any derivatives) to remain free and open.

---
