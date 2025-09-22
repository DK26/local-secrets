# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Initial Implementation**: Minimalist CLI for secure secret management using OS keyrings
- **Core Backend Architecture**: `SecretBackend` trait with `KeyringBackend` for production use
- **Security-First Design**: All secrets wrapped in `SecretString` with automatic memory zeroization
- **Cross-Platform Support**: Windows Credential Manager, macOS Keychain, Linux Secret Service
- **CLI Operations**:
  - `store VARIABLE` - Store secrets securely in OS keyring  
  - `delete VARIABLE` - Remove secrets from keyring
  - `--env VARIABLE -- command args` - Inject secrets into child processes
- **Input Validation**: Comprehensive security validation in `src/security.rs`
- **Memory Safety**: Uses `mimalloc` with secure features and explicit `zeroize()` calls
- **Size Optimization**: 74% size reduction (1.87MB → 486KB) with `opt-level="z"` and LTO
- Comprehensive CI/CD pipeline with GitHub Actions workflows
- Local CI scripts (`ci-local.ps1`, `ci-local.sh`) for development parity
- End-to-end keyring integration tests for actual OS keyring validation
- Fail-fast CI behavior with auto-formatting to save development time
- Professional documentation structure following industry standards

### Security
- **Zero Plaintext Storage**: No configuration files or plaintext storage anywhere
- **Attack Prevention**: Protection against command injection, path traversal, and environment pollution  
- **Memory Protection**: Automatic zeroization of sensitive data with `SecretString`
- **Platform Security**: Windows 8.3 short-name handling and comprehensive input sanitization
- **Defensive Programming**: All inputs validated at function boundaries with fail-fast error handling
- Added comprehensive security testing framework with attack pattern validations
- Implemented input validation protection against command injection and path traversal
- Enhanced memory safety with automatic `SecretString` zeroization
- Added protection against resource exhaustion and unicode-based attacks

### Changed
- Optimized CI scripts to format code automatically before linting
- Enhanced test suite organization with focus on real functionality validation
- Improved error messages and security warnings

### Removed
- **MemoryBackend eliminated** - Removed useless memory backend that served no real purpose
- Pointless memory backend functionality tests that didn't validate real behavior
- Redundant CLI tests using memory backend instead of keyring integration
- Unnecessary duplicate CI scripts in favor of optimized existing ones

### Technical Details
- **Dependencies**: Minimal dependency footprint with `default-features = false` optimization
- **Error Handling**: `anyhow::Result` with contextual error messages throughout
- **Testing**: Comprehensive test suite with security validation and keyring integration
- **Build System**: Cargo workspace with release optimization profiles
- **Documentation**: Extensive security guidelines and development workflow documentation

### Added
- **Initial Release**: Minimalist CLI for secure secret management using OS keyrings
- **Core Backend Architecture**: `SecretBackend` trait with `KeyringBackend` (production) and `MemoryBackend` (testing)
- **Security-First Design**: All secrets wrapped in `SecretString` with automatic memory zeroization
- **Cross-Platform Support**: Windows Credential Manager, macOS Keychain, Linux Secret Service
- **CLI Operations**:
  - `store VARIABLE` - Store secrets securely in OS keyring  
  - `delete VARIABLE` - Remove secrets from keyring
  - `--env VARIABLE -- command args` - Inject secrets into child processes
- **Input Validation**: Comprehensive security validation in `src/security.rs`
- **Memory Safety**: Uses `mimalloc` with secure features and explicit `zeroize()` calls
- **Test Features**: `test-secret-param` feature enables `--test-secret` for CI/CD automation
- **Size Optimization**: 74% size reduction (1.87MB → 486KB) with `opt-level="z"` and LTO

### Security
- **Zero Plaintext Storage**: No configuration files or plaintext storage anywhere
- **Attack Prevention**: Protection against command injection, path traversal, and environment pollution  
- **Memory Protection**: Automatic zeroization of sensitive data with `SecretString`
- **Platform Security**: Windows 8.3 short-name handling and comprehensive input sanitization
- **Defensive Programming**: All inputs validated at function boundaries with fail-fast error handling

### Technical Details
- **Dependencies**: Minimal dependency footprint with `default-features = false` optimization
- **Error Handling**: `anyhow::Result` with contextual error messages throughout
- **Testing**: Comprehensive test suite with security validation and keyring integration
- **Build System**: Cargo workspace with release optimization profiles
- **Documentation**: Extensive security guidelines and development workflow documentation