# local-secrets

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![CI](https://github.com/DK26/local-secrets/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/DK26/local-secrets/actions/workflows/ci.yml)
[![Security Audit](https://github.com/DK26/local-secrets/actions/workflows/audit.yml/badge.svg?branch=main)](https://github.com/DK26/local-secrets/actions/workflows/audit.yml)
[![Keyring Protected](https://img.shields.io/badge/protected%20by-OS%20Keyring-green.svg)](https://github.com/DK26/local-secrets)

**Minimalist CLI for secure secret management using OS keyring.**

A simple tool to **store secrets in your OS keyring** and inject them as environment variables into child processes.  
No plaintext files, no persistent environment variables, just secure storage and explicit injection.

## 🚨 **Why Your Current Secret Management is Probably Broken**

```bash
# ❌ This exposes secrets in shell history, process lists, and environment dumps
export API_KEY="super_secret_key"
my-app deploy

# ❌ This stores secrets in plaintext files that get committed, copied, leaked
echo "API_KEY=super_secret_key" > .env
docker run --env-file .env my-app

# ✅ This uses OS keyring and only injects to specific processes
local-secrets --env API_KEY -- my-app deploy
```

**The Problem**: Most developers store secrets in shell configs, `.env` files, or environment variables that persist across sessions.

**The Solution**: Store secrets in OS keyring, inject only to specific processes.

## ⚡ **Get Secure in 30 Seconds**

```bash
# 1. Store a secret (encrypted in OS keyring)
local-secrets store API_KEY
Enter secret for API_KEY: ********

# 2. Use it securely (injected only into your process)  
local-secrets --env API_KEY -- curl -H "Authorization: Bearer $API_KEY" api.example.com

# 3. That's it - no plaintext files, no persistent environment variables
```

## 🛡️ **Security Features**

- **🔐 OS Keyring Encryption** — Windows Credential Manager, macOS Keychain, Linux Secret Service
- **🧠 Memory Safety** — `SecretString` with automatic memory zeroization, no plaintext in memory dumps  
- **🎯 Explicit Injection** — You choose exactly which secrets to expose, when, and to what process
- **🚫 Zero Plaintext Storage** — No config files, no environment persistence, no accidental leaks
- **🛡️ Input Validation** — Protection against command injection, path traversal, and other attack vectors
- **🔍 Input Validation Tests** — Test suite validates against common attack patterns

## 🎯 **When to Use local-secrets**

| Your Scenario                         | Use local-secrets | Why                                               |
| ------------------------------------- | ----------------- | ------------------------------------------------- |
| **Local development with API keys**   | ✅ Yes             | Secure storage, no accidental commits             |
| **CI/CD secret injection**            | ✅ Yes             | Explicit injection, audit trail                   |
| **Docker containers needing secrets** | ✅ Yes             | No plaintext files in images                      |
| **Multi-environment deployments**     | ✅ Yes             | Environment-specific keyring isolation            |
| **Team secret sharing**               | ❌ No              | Use dedicated secret management platforms         |
| **Production server secrets**         | ❌ Maybe           | Consider HashiCorp Vault or cloud secret managers |

---

## ✨ Quick Start

### Install (from source)

```bash
git clone https://github.com/DK26/local-secrets.git
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
