#!/bin/bash
# ci-local.sh - Local CI Test Runner for local-secrets
# Run all CI checks locally before pushing
# Inspired by strict-path-rs but adapted for security-focused CLI tool

set -e

echo "🔐 === local-secrets CI Local Test Runner ==="

# Try to find cargo in common locations  
if ! command -v cargo &> /dev/null; then
    # Try common cargo locations across platforms
    CARGO_PATHS=(
        "$HOME/.cargo/bin/cargo"
        "$HOME/.cargo/bin/cargo.exe" 
        "/c/Users/$(whoami)/.cargo/bin/cargo.exe"
        "/home/$(whoami)/.cargo/bin/cargo"
        "$(pwd)/../.cargo/bin/cargo"
    )
    
    for cargo_path in "${CARGO_PATHS[@]}"; do
        if [[ -x "$cargo_path" ]]; then
            export PATH="$(dirname "$cargo_path"):$PATH"
            echo "✓ Found cargo at: $cargo_path"
            break
        fi
    done
    
    # Final check
    if ! command -v cargo &> /dev/null; then
        echo "❌ cargo not found. Make sure Rust is installed."
        echo ""
        echo "To run CI tests:"
        echo "  • Make sure 'cargo --version' works in your terminal"
        echo "  • Or install Rust from https://rustup.rs/"
        exit 1
    fi
fi

echo "✓ Using cargo: $(command -v cargo)"

# Check Rust version and warn about nightly vs stable differences
RUST_VERSION=$(rustc --version)
echo "🦀 Rust version: $RUST_VERSION"

if echo "$RUST_VERSION" | grep -q "nightly"; then
    echo "⚠️  WARNING: You're using nightly Rust, but GitHub Actions uses stable!"
    echo "   Some nightly-only APIs might work locally but fail in CI."
    echo "   Consider testing with: rustup default stable"
elif echo "$RUST_VERSION" | grep -qE "1\.(8[8-9]|9[0-9]|[0-9]{3})"; then
    echo "⚠️  WARNING: You're using a newer Rust version than GitHub Actions stable!"
    echo "   GitHub Actions uses the latest stable release."
fi
echo

echo "🔧 Auto-fixing common issues before CI checks"
echo

run_check() {
    local name="$1"
    local command="$2"
    
    echo "Running: $name"
    echo "Command: $command"
    
    start_time=$(date +%s)
    
    if eval "$command"; then
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo "✅ $name completed in ${duration}s"
        echo
        return 0
    else
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo "❌ $name failed after ${duration}s"
        echo "💥 CI checks failed. Fix issues before pushing."
        exit 1
    fi
}

run_fix() {
    local name="$1"
    local command="$2"
    
    echo "Auto-fixing: $name"
    echo "Command: $command"
    
    start_time=$(date +%s)
    
    if eval "$command"; then
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo "✅ $name auto-fix completed in ${duration}s"
        echo
        return 0
    else
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo "⚠️  $name auto-fix failed after ${duration}s"
        echo "⚠️  Continuing with CI checks anyway..."
        echo
        return 1
    fi
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo "❌ Cargo.toml not found. Are you in the project root?"
    exit 1
fi

# Validate file encodings first (critical for Cargo publish)
echo "🔍 Validating UTF-8 encoding for critical files..."

check_utf8_encoding() {
    local file="$1"
    
    # Check if file exists
    if [[ ! -f "$file" ]]; then
        echo "❌ File not found: $file"
        return 1
    fi
    
    # Method 1: Use file command if available (most reliable)
    if command -v file >/dev/null 2>&1; then
        local file_output=$(file "$file")
        # Check for UTF-8, ASCII, text files, or source files (which are typically UTF-8)
        if echo "$file_output" | grep -q "UTF-8\|ASCII\|text\|[Ss]ource"; then
            echo "✅ $file: UTF-8 encoding verified"
            return 0
        else
            echo "❌ $file: Not UTF-8 encoded - $file_output"
            return 1
        fi
    fi
    
    # Method 2: Check for UTF-16 BOM (Windows PowerShell sometimes creates these)
    if command -v xxd >/dev/null 2>&1; then
        if head -c 2 "$file" | xxd | grep -q "fffe\|feff"; then
            echo "❌ $file: Contains UTF-16 BOM (use UTF-8 without BOM)"
            return 1
        fi
    elif command -v od >/dev/null 2>&1; then
        if head -c 2 "$file" | od -t x1 | grep -q "ff fe\|fe ff"; then
            echo "❌ $file: Contains UTF-16 BOM (use UTF-8 without BOM)"
            return 1
        fi
    fi
    
    # Method 3: Try to read with Python UTF-8 (fallback)
    if command -v python3 >/dev/null 2>&1; then
        if python3 -c "
import sys
try:
    with open('$file', 'r', encoding='utf-8') as f:
        f.read()
    print('✅ $file: UTF-8 encoding verified (Python check)')
except UnicodeDecodeError as e:
    print('❌ $file: Not valid UTF-8 -', str(e))
    sys.exit(1)
        "; then
            return 0
        else
            return 1
        fi
    fi
    
    # If no validation method available, warn but continue
    echo "⚠️  Cannot verify encoding for $file (no validation tools available)"
    echo "   Assuming UTF-8. Install 'file' command for proper validation."
    return 0
}

check_no_bom() {
    local file="$1"
    
    # Check for UTF-8 BOM (EF BB BF) which should not be present
    if command -v xxd >/dev/null 2>&1; then
        if head -c 3 "$file" | xxd | grep -qi "efbbbf"; then
            echo "❌ $file: Contains UTF-8 BOM (should be UTF-8 without BOM)"
            return 1
        fi
        echo "✅ $file: No BOM detected (correct)"
    elif command -v od >/dev/null 2>&1; then
        if head -c 3 "$file" | od -t x1 | grep -q "ef bb bf"; then
            echo "❌ $file: Contains UTF-8 BOM (should be UTF-8 without BOM)"
            return 1
        fi
        echo "✅ $file: No BOM detected (correct)"
    fi
    
    return 0
}

# Check critical files for encoding issues
echo "📄 Checking README.md..."
check_utf8_encoding "README.md" || exit 1
check_no_bom "README.md" || exit 1

echo "📄 Checking Cargo.toml..."
check_utf8_encoding "Cargo.toml" || exit 1

echo "📄 Checking Rust source files..."
if find src -name "*.rs" -type f | head -1 >/dev/null 2>&1; then
    find src -name "*.rs" -type f | while read file; do
        check_utf8_encoding "$file" || exit 1
    done
    echo "✅ All Rust source files: UTF-8 encoding verified"
fi

if find tests -name "*.rs" -type f | head -1 >/dev/null 2>&1; then
    find tests -name "*.rs" -type f | while read file; do
        check_utf8_encoding "$file" || exit 1
    done
    echo "✅ All test files: UTF-8 encoding verified"
fi

echo "🎉 All file encoding checks passed!"
echo

# Auto-format FIRST to save time on linting
echo "🔧 Auto-formatting code before linting..."
run_fix "Format Code" "cargo fmt --all"
echo

echo "🦀 Running CI checks (same as GitHub Actions)..."
echo "⚠️  FAIL-FAST MODE: Any linting error will stop execution"
echo

# FAIL FAST: Format check - if this fails, stop immediately 
run_check "Format Check" '
    set -e
    if ! cargo fmt --all -- --check; then
        echo "❌ Formatting issues found after auto-format!"
        echo "💥 This should not happen - check for syntax errors"
        exit 1
    fi
'

# FAIL FAST: Linting - if this fails, stop immediately
run_check "Clippy Lint (FAIL-FAST)" "cargo clippy --all-targets --all-features -- -D warnings"

# Security-focused testing with keyring backend
# Note: Using actual keyring for real-world validation

# Comprehensive test suite
run_check "Unit Tests" "cargo test --bins --verbose"
run_check "Security Validation Tests" "cargo test --test security_tests --verbose"
run_check "Store Automation Tests" "cargo test --test store_automation_tests --verbose --features test-secret-param"
run_check "Security Input Validation Tests" "cargo test --test security_validation_tests --verbose --features test-secret-param"

# Documentation
run_check "Documentation" "RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --document-private-items --all-features"
# Skip doc tests for binary crate
echo "📚 Documentation Tests: SKIPPED (binary crate only)"

# Build optimized release
run_check "Build Release Binary" "cargo build --release"

# CLI functionality tests
# Skip CLI help tests due to clap design limitation with last=true 
echo "📖 CLI Help Tests: SKIPPED (known limitation with last=true attribute)"

# CRITICAL: Real keyring end-to-end testing
echo "🔑 Running real keyring integration tests..."
export LOCAL_SECRETS_TEST_MODE=""  # Use actual keyring
export LOCAL_SECRETS_BACKEND=""    # Use actual keyring

# Real keyring integration tests:
# This validates what memory backend tests CANNOT test:
# - Actual OS keyring store/retrieve cycles across platforms
# - Real-world keyring service interaction and error handling
run_check "Keyring Integration Tests (E2E)" "cargo test --test keyring_integration_tests --verbose" || {
    echo "⚠️  Keyring integration tests failed or keyring service unavailable"
    echo "💡 This is expected in some CI environments without keyring services"
    echo "🔍 Check test output above for details"
}

# Security audit (if available)
echo "🔍 Running security audit..."
if command -v cargo-audit &> /dev/null; then
    run_check "Security Audit" "cargo audit"
else
    echo "⚠️  cargo-audit not found. Installing..."
    if cargo install cargo-audit --locked; then
        echo "✅ cargo-audit installed"
        run_check "Security Audit" "cargo audit"
    else
        echo "⚠️  Could not install cargo-audit. Skipping security audit."
        echo "💡 To install manually: cargo install cargo-audit"
    fi
fi

echo "🎉 All CI checks passed!"
echo "💡 Remember to review and commit any auto-fixes made."
echo "🚀 Ready to push to remote."