# CI/CD Setup for local-secrets

This document describes the comprehensive CI/CD pipeline for local-secrets, inspired by strict-path-rs patterns but adapted for our security-focused CLI tool.

## 🏗️ CI/CD Architecture

### GitHub Actions Workflows

#### 1. **Main CI Pipeline** (`.github/workflows/ci.yml`)
- **Triggers**: Push to main, Pull Requests
- **Multi-platform testing**: Ubuntu, Windows, macOS
- **Security-first testing strategy**: 
  - UTF-8 encoding validation
  - **REAL keyring end-to-end integration testing** (5 tests validating actual OS keyring services)
  - Windows Credential Manager, macOS Keychain, Linux Secret Service validation
  - Memory backend security protection (4 tests ensuring memory backend is blocked in production)
  - Comprehensive injection and attack prevention (11 security tests)
  - Input validation and security function testing (5 unit tests)
- **Code quality**: Format checking, Clippy linting, documentation tests
- **Features tested**: Both regular and `test-secret-param` feature

#### 2. **Security Audit** (`.github/workflows/audit.yml`)
- **Triggers**: Push/PR to main, Daily at 2 AM UTC, Manual dispatch
- **Security scanning**: `cargo audit` for vulnerability detection
- **Security validation**: Tests memory backend restrictions
- **Artifact upload**: JSON audit results for analysis

#### 3. **Release Pipeline** (`.github/workflows/release.yml`)
- **Triggers**: Version tags (`v*`)
- **Multi-platform builds**: Linux, macOS, Windows
- **Size optimization**: Uses our custom release profile settings
- **Binary verification**: Size checks and smoke tests
- **Release artifacts**: Compressed binaries with checksums
- **Documentation**: Auto-generated release notes with security highlights

### Local Development Scripts

#### **PowerShell**: `ci-local.ps1`
```powershell
# Run all CI checks locally
.\ci-local.ps1
```

#### **Bash**: `ci-local.sh` 
```bash
# Run all CI checks locally  
./ci-local.sh
```

Both scripts provide:
- **Auto-fixing**: Format and clippy issues
- **UTF-8 validation**: Critical for cross-platform compatibility
- **Security testing**: Full test suite with memory backend
- **End-to-end keyring testing**: Validates actual OS keyring functionality
- **Documentation validation**: Doc tests and warnings
- **Binary building**: Release-optimized builds
- **Audit integration**: Security vulnerability scanning

## 🚀 **Continuous Deployment (CD) - Runs Only on Tags**

Release builds are triggered **only** when you push a tag starting with `v`:

```bash
# Create and push a release tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

### Release Pipeline:

1. **Build Release Binaries** (`build-release`):
   - **Platforms**: 
     - `x86_64-unknown-linux-gnu` (Linux x64)
     - `aarch64-apple-darwin` (macOS ARM64/Apple Silicon)
     - `x86_64-pc-windows-msvc` (Windows x64)
   - **Artifacts**: Creates optimized binaries for each platform
   - **Packaging**: `.tar.gz` for Unix, `.zip` for Windows

2. **Publish Release** (`publish-release`):
   - **GitHub Release**: Automatically creates GitHub release
   - **Checksums**: Generates `SHA256SUMS` for all binaries
   - **Release Notes**: Auto-generates from commits since last tag

## 🛡️ **Security & Quality Gates**

### **Mandatory Checks (All Must Pass):**
- ✅ Code formatting (`cargo fmt`)
- ✅ Zero clippy warnings (`cargo clippy -D warnings`)
- ✅ All tests pass (`cargo test`)
- ✅ Multi-platform compatibility
- ✅ Security audit clean (`cargo audit`)
- ✅ Basic CLI functionality

### **Release Requirements:**
- ✅ All CI checks must pass
- ✅ Security audit must be clean
- ✅ Tag must follow `v*` pattern
- ✅ Builds must succeed on all platforms

## 📋 **Workflow Commands**

### **For Development:**
```bash
# Run all checks locally (same as CI)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings  
cargo test --all-targets

# CRITICAL: Test with REAL keyring (not memory backend)
unset LOCAL_SECRETS_TEST_MODE  # or $env:LOCAL_SECRETS_TEST_MODE=""
cargo test --test keyring_integration_tests

# Full test suite (25 tests: unit + keyring + security)
cargo test --all-targets

# Security audit
cargo audit

# Build release locally
cargo build --release
```

### **For Releases:**
```bash
# Create a release (triggers CD pipeline)
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# View release status
# Check GitHub Actions tab in repository
```

## 🎯 **Best Practices**

1. **Always test locally** before pushing
2. **Use semantic versioning** for tags (v1.0.0, v1.0.1, etc.)
3. **Write meaningful tag messages** (they become release notes)
4. **Monitor the Actions tab** for build status
5. **Fix security issues immediately** (blocks all releases)

## 🎯 **CRITICAL: Keyring Integration Testing**

**Major Testing Breakthrough**: We now test REAL keyring functionality, not just memory backend substitutes.

### Why This Matters
- **Previous Issue**: 99% of tests used `MemoryBackend` (plaintext temp files), providing false confidence
- **Solution**: `tests/keyring_integration_tests.rs` validates actual OS keyring services
- **Impact**: We now know our tool works end-to-end with real Windows Credential Manager, macOS Keychain, Linux Secret Service

### Keyring Test Suite (`keyring_integration_tests.rs`)
- ✅ **`test_keyring_availability`**: Basic store/retrieve/delete cycle
- ✅ **`test_keyring_error_handling`**: Invalid operations and edge cases  
- ✅ **`test_keyring_service_isolation`**: Namespace isolation between services
- ✅ **`test_keyring_performance`**: Benchmarks with actual keyring latency
- ✅ **`test_windows_credential_manager`**: Windows-specific validation
- ✅ **`test_macos_keychain`**: macOS Keychain validation (when available)
- ✅ **`test_linux_secret_service`**: Linux Secret Service validation (when available)

### Running Keyring Tests
```bash
# Windows PowerShell
$env:LOCAL_SECRETS_TEST_MODE=""
cargo test --test keyring_integration_tests

# Linux/macOS
unset LOCAL_SECRETS_TEST_MODE
cargo test --test keyring_integration_tests
```

**Expected Results**: 5 tests pass in ~10-15 seconds with real keyring interactions.

### Test Suite Summary (25 Total Tests)
- **5 Unit Tests**: Security function validation (`src/security.rs`)
- **5 Keyring Integration Tests**: Real OS keyring functionality
- **4 Memory Backend Security Tests**: Production security validation  
- **11 Security Tests**: Injection prevention and attack resistance

**Philosophy**: Test real functionality, not fake implementations. Every test serves a clear security or functional purpose.

## 🔧 **Troubleshooting**

- **CI failing?** Check the Actions tab for detailed logs
- **Security audit failing?** Run `cargo audit` locally and update dependencies
- **Cross-platform issues?** Test on different OS if possible
- **Release not created?** Ensure tag starts with 'v' and all checks pass