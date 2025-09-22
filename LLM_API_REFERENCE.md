# local-secrets — LLM API Reference 

**Security-first secret management**: Store secrets in OS keyring and inject them safely into processes.

## Essential Flow (Start Here)
1. **Store secret**: `local-secrets store VARIABLE` → encrypted in OS keyring
2. **Inject securely**: `local-secrets --env VARIABLE -- command args` → explicit injection only
3. **Clean up**: `local-secrets delete VARIABLE` → remove when no longer needed

**Core Security Promise**: If you use local-secrets for injection, secrets are never stored in plaintext and only exposed to the exact process that needs them.

## Command Reference

### Storage Commands
```bash
# Interactive secret storage (secure prompt)
local-secrets store VARIABLE_NAME

# Automated storage for testing (requires test-secret-param feature)
local-secrets store VARIABLE_NAME --test-secret "secret_value"

# Delete stored secret
local-secrets delete VARIABLE_NAME
```

### Injection Commands  
```bash
# Single secret injection
local-secrets --env VARIABLE_NAME -- command args

# Multiple secret injection
local-secrets --env VAR1 --env VAR2 --env VAR3 -- command args

# Injection with missing secret handling
local-secrets --env VARIABLE_NAME --no-save-missing -- command args
```

## Security Architecture

**Backend Selection**:
- **Production (Default)**: `KeyringBackend` → OS keyring (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- **Testing Only**: `MemoryBackend` → Plaintext temp files (requires `LOCAL_SECRETS_TEST_MODE=1`)

**Memory Safety**:
- All secrets wrapped in `SecretString` with automatic zeroization
- No plaintext storage in memory dumps or swap files
- Explicit memory clearing after use

**Input Validation**:
- Environment variable names validated against injection patterns
- Secret values checked for null bytes and size limits
- Command arguments sanitized for shell metacharacters

## LLM Integration Patterns

### Pattern 1: AI Agent File Operations
```bash
# Store API key for LLM service
local-secrets store OPENAI_API_KEY

# Use in AI workflows
local-secrets --env OPENAI_API_KEY -- python ai_agent.py --task "analyze code"
```

### Pattern 2: Dynamic Secret Management
```bash
# Store secrets based on AI-generated variable names (after validation)
local-secrets store "$(echo "$AI_GENERATED_NAME" | tr -d '[:cntrl:]')"

# Inject into AI-recommended tools
local-secrets --env VALIDATED_SECRET -- "$AI_RECOMMENDED_COMMAND"
```

### Pattern 3: CI/CD Integration
```bash
# Store deployment secrets
local-secrets store DEPLOY_KEY

# Use in automated workflows
local-secrets --env DEPLOY_KEY -- deploy-script.sh production
```

## Security Validation

**Input Validation Functions** (implemented in `src/security.rs`):
- `validate_env_var_name()` - Blocks dangerous patterns like `$(...)`, `;`, `&&`, `../`
- `validate_secret_value()` - Enforces size limits and null byte detection  
- `validate_command_args()` - Prevents shell metacharacter injection

**Protected Patterns**:
- Command injection: `"$(rm -rf /)"`, `"; cat /etc/passwd"`
- Path traversal: `"../../../etc/passwd"`, `"..\\..\\system32"`
- Environment pollution: Warns on system variables like `PATH`, `HOME`
- Resource exhaustion: 1MB limit on secret values

## Backend Configuration

### Production Backend (Default)
```bash
# Uses OS keyring automatically
local-secrets store API_KEY
local-secrets --env API_KEY -- my-app
```

### Test Backend (Development Only)  
```bash
# Enable test mode (required for memory backend)
export LOCAL_SECRETS_TEST_MODE=1
export LOCAL_SECRETS_BACKEND=memory

# Now uses temporary files (PLAINTEXT - never use in production)
local-secrets store TEST_SECRET
```

**⚠️ CRITICAL WARNING**: Memory backend stores secrets in **PLAINTEXT** temporary files and is **NEVER** safe for production use. It requires explicit activation to prevent accidental production usage.

## Error Handling

**Common Error Types**:
- `Failed to store secret` - Keyring service unavailable or permission denied
- `Environment variable name contains dangerous pattern` - Input validation blocked injection attempt  
- `Secret value too long` - Exceeds 1MB limit for resource protection
- `Command not found` - Target command doesn't exist or isn't executable

**Error Response Pattern**:
```rust
// When validation fails
if dangerous_input_detected {
    return Err("Environment variable name contains dangerous pattern: $(");
}

// When keyring operations fail  
if keyring_error {
    return Err("Failed to store secret in keyring: access denied");
}
```

## Testing Patterns

### Security Testing
```bash
# Test malicious variable names (should fail)
local-secrets store '$(echo injection)' # Error: dangerous pattern
local-secrets store '../../../etc/passwd' # Error: dangerous pattern

# Test resource limits (should fail)
local-secrets store HUGE_SECRET --test-secret "$(head -c 10M /dev/zero)"
```

### Integration Testing  
```bash
# Test keyring backend (production)
local-secrets store TEST_VAR --test-secret "test_value"
local-secrets --env TEST_VAR -- echo "Success: $TEST_VAR"
local-secrets delete TEST_VAR

# Test memory backend (development)  
LOCAL_SECRETS_TEST_MODE=1 LOCAL_SECRETS_BACKEND=memory \
local-secrets store TEST_VAR --test-secret "test_value"
```

### CI/CD Testing
```bash
# Automated store with test feature
cargo build --features test-secret-param
local-secrets store CI_SECRET --test-secret "$SECRET_VALUE"

# Verify injection works  
local-secrets --env CI_SECRET -- test-script.sh
```

## Common Anti-Patterns

### ❌ Don't Do This
```bash
# Storing secrets in environment variables (persistent exposure)
export API_KEY="secret"
my-app

# Using memory backend in production (plaintext files)
LOCAL_SECRETS_BACKEND=memory local-secrets store PROD_SECRET

# Bypassing input validation (security vulnerability)
local-secrets store "$(malicious_command)" 

# Using secrets in shell history (exposure risk)
local-secrets store API_KEY --test-secret "visible_in_history"
```

### ✅ Do This Instead
```bash
# Use interactive storage (hidden prompt)
local-secrets store API_KEY

# Use explicit injection (no persistence)  
local-secrets --env API_KEY -- my-app

# Use keyring backend (encrypted storage)
local-secrets store API_KEY # Uses keyring automatically

# Use secure automation (test feature only)
local-secrets store API_KEY --test-secret "$SECURE_VARIABLE"
```

## Advanced Usage

### Multiple Environment Management
```bash
# Development secrets
local-secrets store DEV_API_KEY

# Production secrets (separate keyring entry)
local-secrets store PROD_API_KEY  

# Use appropriate secret per environment
local-secrets --env "${ENV}_API_KEY" -- deploy.sh
```

### Docker Integration
```bash
# Store secrets on host
local-secrets store DOCKER_SECRET

# Inject into container (no plaintext in image)
local-secrets --env DOCKER_SECRET -- docker run --rm -e DOCKER_SECRET my-app
```

### Batch Operations
```bash
# Store multiple secrets
for secret in API_KEY DB_PASSWORD JWT_SECRET; do
    local-secrets store "$secret"
done

# Inject multiple secrets
local-secrets --env API_KEY --env DB_PASSWORD --env JWT_SECRET -- my-app start
```

## Platform-Specific Behavior

### Windows
- Uses **Windows Credential Manager**
- Secrets stored per user account
- Requires user authentication for access

### macOS  
- Uses **Keychain Services**
- Integration with system keychain
- Supports Touch ID/Face ID authentication

### Linux
- Uses **Secret Service** (GNOME Keyring, KWallet)
- Desktop session integration
- May require keyring unlock

## Development Guidelines

### For LLM Agents
- **Always validate** AI-generated variable names and command arguments
- **Never bypass** input validation - it prevents real attacks
- **Use test features** only in development environments  
- **Prefer explicit injection** over environment variable persistence

### For CI/CD Systems
- Use `--test-secret` parameter for automated testing (requires feature flag)
- Set `LOCAL_SECRETS_TEST_MODE=1` for memory backend testing
- Implement proper error handling for keyring service unavailability
- Use separate keyring entries for different environments/stages

### For Security Reviews
- All secret storage uses OS keyring encryption by default
- Input validation prevents command injection and path traversal
- Memory safety prevents secret leakage in dumps/swap
- Test mode is explicitly isolated from production usage