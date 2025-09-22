# ci-local.ps1 - Local CI Test Runner for local-secrets (PowerShell)
# Run all CI checks locally before pushing
# Inspired by strict-path-rs but adapted for security-focused CLI tool

$ErrorActionPreference = "Stop"

Write-Host "🔐 === local-secrets CI Local Test Runner ===" -ForegroundColor Cyan

# Try to find cargo in common locations  
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    # Try common cargo locations across platforms
    $cargoPaths = @(
        "$env:USERPROFILE\.cargo\bin\cargo.exe",
        "$env:HOME\.cargo\bin\cargo.exe",
        "$env:HOME\.cargo\bin\cargo",
        "C:\Users\$env:USERNAME\.cargo\bin\cargo.exe"
    )
    
    $cargoFound = $false
    foreach ($cargoPath in $cargoPaths) {
        if (Test-Path $cargoPath) {
            $env:PATH = "$(Split-Path $cargoPath);$env:PATH"
            Write-Host "✓ Found cargo at: $cargoPath" -ForegroundColor Green
            $cargoFound = $true
            break
        }
    }
    
    # Final check
    if (-not $cargoFound -and -not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "❌ cargo not found. Make sure Rust is installed." -ForegroundColor Red
        Write-Host "" 
        Write-Host "To run CI tests:" -ForegroundColor Yellow
        Write-Host "  * Make sure 'cargo --version' works in your terminal" -ForegroundColor Yellow
        Write-Host "  * Or install Rust from https://rustup.rs/" -ForegroundColor Yellow
        exit 1
    }
}

Write-Host "✓ Using cargo: $(Get-Command cargo | Select-Object -ExpandProperty Source)" -ForegroundColor Green

# Check Rust version and warn about nightly vs stable differences
$rustVersion = & rustc --version
Write-Host "🦀 Rust version: $rustVersion" -ForegroundColor Magenta

if ($rustVersion -match "nightly") {
    Write-Host "⚠️  WARNING: You're using nightly Rust, but GitHub Actions uses stable!" -ForegroundColor Yellow
    Write-Host "   Some nightly-only APIs might work locally but fail in CI." -ForegroundColor Yellow
    Write-Host "   Consider testing with: rustup default stable" -ForegroundColor Yellow
} elseif ($rustVersion -match "1\.(8[8-9]|9[0-9]|\d{3})") {
    Write-Host "⚠️  WARNING: You're using a newer Rust version than GitHub Actions stable!" -ForegroundColor Yellow
    Write-Host "   GitHub Actions uses the latest stable release." -ForegroundColor Yellow
}
Write-Host ""

Write-Host "🔧 Auto-fixing common issues before CI checks" -ForegroundColor Cyan
Write-Host ""

function Run-Check {
    param(
        [string]$Name,
        [string]$Command
    )
    
    Write-Host "Running: $Name" -ForegroundColor Blue
    Write-Host "Command: $Command" -ForegroundColor Gray
    
    $startTime = Get-Date
    
    try {
        Invoke-Expression $Command
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed with exit code $LASTEXITCODE"
        }
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host "✅ $Name completed in $([math]::Round($duration))s" -ForegroundColor Green
        Write-Host ""
        return
    } catch {
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host "❌ $Name failed after $([math]::Round($duration))s" -ForegroundColor Red
        Write-Host "💥 CI checks failed. Fix issues before pushing." -ForegroundColor Red
        throw $_
    }
}

# Execute a scriptblock as a CI check (avoids quoting/interpolation pitfalls)
function Run-Check-Block {
    param(
        [string]$Name,
        [scriptblock]$Block
    )

    Write-Host "Running: $Name" -ForegroundColor Blue
    $startTime = Get-Date
    try {
        & $Block
        if ($LASTEXITCODE -ne 0) {
            throw "Script block failed with exit code $LASTEXITCODE"
        }
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host "✅ $Name completed in $([math]::Round($duration))s" -ForegroundColor Green
        Write-Host ""
        return
    } catch {
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host "❌ $Name failed after $([math]::Round($duration))s" -ForegroundColor Red
        Write-Host "💥 CI checks failed. Fix issues before pushing." -ForegroundColor Red
        throw $_
    }
}

function Run-Fix {
    param(
        [string]$Name,
        [string]$Command
    )
    
    Write-Host "Auto-fixing: $Name" -ForegroundColor Blue
    Write-Host "Command: $Command" -ForegroundColor Gray
    
    $startTime = Get-Date
    
    try {
        Invoke-Expression $Command
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host "✅ $name auto-fix completed in $([math]::Round($duration))s" -ForegroundColor Green
        Write-Host ""
        return $true
    } catch {
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host "⚠️  $Name auto-fix failed after $([math]::Round($duration))s" -ForegroundColor Yellow
        Write-Host "⚠️  Continuing with CI checks anyway..." -ForegroundColor Yellow
        Write-Host ""
        return $false
    }
}

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "❌ Cargo.toml not found. Are you in the project root?" -ForegroundColor Red
    exit 1
}

# Validate file encodings first (critical for Cargo publish)
Write-Host "🔍 Validating UTF-8 encoding for critical files..." -ForegroundColor Cyan

function Test-Utf8Encoding {
    param([string]$FilePath)
    
    if (-not (Test-Path $FilePath)) {
        Write-Host "❌ File not found: $FilePath" -ForegroundColor Red
        return $false
    }
    
    try {
        # Try to read as UTF-8
        $content = Get-Content -Path $FilePath -Encoding UTF8 -Raw -ErrorAction Stop
        
        # Check for UTF-8 BOM (should not be present)
        $bytes = [System.IO.File]::ReadAllBytes($FilePath)
        if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
            Write-Host "❌ $FilePath: Contains UTF-8 BOM (should be UTF-8 without BOM)" -ForegroundColor Red
            return $false
        }
        
        # Check for UTF-16 BOM
        if ($bytes.Length -ge 2 -and (($bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) -or ($bytes[0] -eq 0xFE -and $bytes[1] -eq 0xFF))) {
            Write-Host "❌ $FilePath: Contains UTF-16 BOM (use UTF-8 without BOM)" -ForegroundColor Red
            return $false
        }
        
        Write-Host "✅ $FilePath: UTF-8 encoding verified" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "❌ $FilePath: Not valid UTF-8 - $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Check critical files for encoding issues
Write-Host "📄 Checking README.md..." -ForegroundColor Blue
if (-not (Test-Utf8Encoding "README.md")) { exit 1 }

Write-Host "📄 Checking Cargo.toml..." -ForegroundColor Blue
if (-not (Test-Utf8Encoding "Cargo.toml")) { exit 1 }

Write-Host "📄 Checking Rust source files..." -ForegroundColor Blue
if (Test-Path "src") {
    $rustFiles = Get-ChildItem -Path "src" -Filter "*.rs" -Recurse
    if ($rustFiles.Count -gt 0) {
        foreach ($file in $rustFiles) {
            if (-not (Test-Utf8Encoding $file.FullName)) { exit 1 }
        }
        Write-Host "✅ All Rust source files: UTF-8 encoding verified" -ForegroundColor Green
    } else {
        Write-Host "⚠️  No Rust source files found in src/" -ForegroundColor Yellow
    }
}

if (Test-Path "tests") {
    $testFiles = Get-ChildItem -Path "tests" -Filter "*.rs" -Recurse
    if ($testFiles.Count -gt 0) {
        foreach ($file in $testFiles) {
            if (-not (Test-Utf8Encoding $file.FullName)) { exit 1 }
        }
        Write-Host "✅ All test files: UTF-8 encoding verified" -ForegroundColor Green
    }
}

Write-Host "🎉 All file encoding checks passed!" -ForegroundColor Green
Write-Host ""

# Auto-fix common issues first
Write-Host "🔧 Auto-fixing common issues..." -ForegroundColor Cyan
Run-Fix "Format" "cargo fmt --all"
Run-Fix "Clippy Fixable Issues" "cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features"
Run-Fix "Format (after clippy fix)" "cargo fmt --all"

Write-Host "🦀 Now running CI checks (same as GitHub Actions)..." -ForegroundColor Cyan
Write-Host ""

# Run all CI checks in order
Run-Check-Block "Format Check" {
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Formatting issues found. Run: cargo fmt --all" -ForegroundColor Red
        Write-Host "Then commit the formatting changes." -ForegroundColor Red
        exit 1
    }
}

# Lint and tests with security focus
Run-Check "Clippy Lint" "cargo clippy --all-targets --all-features -- -D warnings"

# Security-focused testing with memory backend
$env:LOCAL_SECRETS_TEST_MODE = "1"
$env:LOCAL_SECRETS_BACKEND = "memory"

Run-Check "Unit Tests" "cargo test --lib --verbose"
Run-Check "Integration Tests" "cargo test --test cli --test security_tests --verbose"
Run-Check "Security Tests" "cargo test --test security_tests --verbose"

# Test with feature flags
Run-Check "Test with test-secret-param feature" "cargo test --test automated_store_tests --features test-secret-param --verbose"

# Documentation
$env:RUSTDOCFLAGS = "-D warnings"
Run-Check "Documentation" "cargo doc --no-deps --document-private-items --all-features"
Run-Check "Documentation Tests" "cargo test --doc --verbose"

# Build optimized release
Run-Check "Build Release Binary" "cargo build --release"

# CLI functionality tests
Run-Check-Block "CLI Help Tests" {
    cargo run --release -- --help | Out-Null
    cargo run --release -- store --help | Out-Null 
    cargo run --release -- delete --help | Out-Null
}

# Security audit (if available)
Write-Host "🔍 Running security audit..." -ForegroundColor Cyan
if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    Run-Check "Security Audit" "cargo audit"
} else {
    Write-Host "⚠️  cargo-audit not found. Installing..." -ForegroundColor Yellow
    try {
        cargo install cargo-audit --locked
        Write-Host "✅ cargo-audit installed" -ForegroundColor Green
        Run-Check "Security Audit" "cargo audit"
    } catch {
        Write-Host "⚠️  Could not install cargo-audit. Skipping security audit." -ForegroundColor Yellow
        Write-Host "💡 To install manually: cargo install cargo-audit" -ForegroundColor Blue
    }
}

Write-Host "🎉 All CI checks passed!" -ForegroundColor Green
Write-Host "💡 Remember to review and commit any auto-fixes made." -ForegroundColor Blue
Write-Host "🚀 Ready to push to remote." -ForegroundColor Green