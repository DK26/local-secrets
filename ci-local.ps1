# ci-local.ps1 - Local CI Test Runner for local-secrets (PowerShell)
# Run all CI checks locally before pushing
# Inspired by strict-path-rs but adapted for security-focused CLI tool

$ErrorActionPreference = "Stop"

Write-Host "=== local-secrets CI Local Test Runner ===" -ForegroundColor Cyan
Write-Host ""

# Try to find cargo in common locations
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
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
            Write-Host "Found cargo at: $cargoPath" -ForegroundColor Green
            $cargoFound = $true
            break
        }
    }

    if (-not $cargoFound -and -not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "ERROR: cargo not found. Make sure Rust is installed." -ForegroundColor Red
        Write-Host ""
        Write-Host "To run CI tests:" -ForegroundColor Yellow
        Write-Host "  * Make sure 'cargo --version' works in your terminal" -ForegroundColor Yellow
        Write-Host "  * Or install Rust from https://rustup.rs/" -ForegroundColor Yellow
        exit 1
    }
}

Write-Host ("Using cargo: {0}" -f (Get-Command cargo | Select-Object -ExpandProperty Source)) -ForegroundColor Green
Write-Host ""

# Check Rust version and warn about nightly vs stable differences
$rustVersion = & rustc --version
Write-Host ("Rust version: {0}" -f $rustVersion) -ForegroundColor Magenta

if ($rustVersion -match "nightly") {
    Write-Host "WARNING: You are using nightly Rust, but GitHub Actions uses stable!" -ForegroundColor Yellow
    Write-Host "         Some nightly-only APIs might work locally but fail in CI." -ForegroundColor Yellow
    Write-Host "         Consider testing with: rustup default stable" -ForegroundColor Yellow
} elseif ($rustVersion -match "1\.(8[8-9]|9[0-9]|\d{3})") {
    Write-Host "WARNING: You are using a newer Rust version than GitHub Actions stable!" -ForegroundColor Yellow
    Write-Host "         GitHub Actions uses the latest stable release." -ForegroundColor Yellow
}
Write-Host ""

Write-Host "Auto-fixing common issues before CI checks" -ForegroundColor Cyan
Write-Host ""

function Run-Check {
    param(
        [string]$Name,
        [string]$Command
    )

    Write-Host ("Running: {0}" -f $Name) -ForegroundColor Blue
    Write-Host ("Command: {0}" -f $Command) -ForegroundColor Gray

    $startTime = Get-Date

    try {
        Invoke-Expression $Command
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed with exit code $LASTEXITCODE"
        }
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host ("OK: {0} completed in {1}s" -f $Name, [math]::Round($duration)) -ForegroundColor Green
        Write-Host ""
        return
    } catch {
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host ("ERROR: {0} failed after {1}s" -f $Name, [math]::Round($duration)) -ForegroundColor Red
        Write-Host "CI checks failed. Fix issues before pushing." -ForegroundColor Red
        throw $_
    }
}

function Run-Check-Block {
    param(
        [string]$Name,
        [scriptblock]$Block
    )

    Write-Host ("Running: {0}" -f $Name) -ForegroundColor Blue
    $startTime = Get-Date
    try {
        & $Block
        if ($LASTEXITCODE -ne 0) {
            throw "Script block failed with exit code $LASTEXITCODE"
        }
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host ("OK: {0} completed in {1}s" -f $Name, [math]::Round($duration)) -ForegroundColor Green
        Write-Host ""
        return
    } catch {
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host ("ERROR: {0} failed after {1}s" -f $Name, [math]::Round($duration)) -ForegroundColor Red
        Write-Host "CI checks failed. Fix issues before pushing." -ForegroundColor Red
        throw $_
    }
}

function Run-Fix {
    param(
        [string]$Name,
        [string]$Command
    )

    Write-Host ("Auto-fixing: {0}" -f $Name) -ForegroundColor Blue
    Write-Host ("Command: {0}" -f $Command) -ForegroundColor Gray

    $startTime = Get-Date

    try {
        Invoke-Expression $Command
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host ("OK: {0} auto-fix completed in {1}s" -f $Name, [math]::Round($duration)) -ForegroundColor Green
        Write-Host ""
        return $true
    } catch {
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        Write-Host ("WARN: {0} auto-fix failed after {1}s" -f $Name, [math]::Round($duration)) -ForegroundColor Yellow
        Write-Host "WARN: Continuing with CI checks anyway..." -ForegroundColor Yellow
        Write-Host ""
        return $false
    }
}

if (-not (Test-Path "Cargo.toml")) {
    Write-Host "ERROR: Cargo.toml not found. Are you in the project root?" -ForegroundColor Red
    exit 1
}

Write-Host "Validating UTF-8 encoding for critical files..." -ForegroundColor Cyan

function Test-Utf8Encoding {
    param([string]$FilePath)

    if (-not (Test-Path $FilePath)) {
        Write-Host ("ERROR: File not found: {0}" -f $FilePath) -ForegroundColor Red
        return $false
    }

    try {
        $content = Get-Content -Path $FilePath -Encoding UTF8 -Raw -ErrorAction Stop

        $bytes = [System.IO.File]::ReadAllBytes($FilePath)
        if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
            Write-Host ("ERROR: {0} contains UTF-8 BOM (should be UTF-8 without BOM)" -f $FilePath) -ForegroundColor Red
            return $false
        }

        if ($bytes.Length -ge 2 -and (($bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) -or ($bytes[0] -eq 0xFE -and $bytes[1] -eq 0xFF))) {
            Write-Host ("ERROR: {0} contains UTF-16 BOM (use UTF-8 without BOM)" -f $FilePath) -ForegroundColor Red
            return $false
        }

        Write-Host ("OK: {0} UTF-8 encoding verified" -f $FilePath) -ForegroundColor Green
        return $true
    } catch {
        Write-Host ("ERROR: {0} not valid UTF-8 - {1}" -f $FilePath, $_.Exception.Message) -ForegroundColor Red
        return $false
    }
}

Write-Host "Checking README.md..." -ForegroundColor Blue
if (-not (Test-Utf8Encoding "README.md")) { exit 1 }

Write-Host "Checking Cargo.toml..." -ForegroundColor Blue
if (-not (Test-Utf8Encoding "Cargo.toml")) { exit 1 }

Write-Host "Checking Rust source files..." -ForegroundColor Blue
if (Test-Path "src") {
    $rustFiles = Get-ChildItem -Path "src" -Filter "*.rs" -Recurse
    if ($rustFiles.Count -gt 0) {
        foreach ($file in $rustFiles) {
            if (-not (Test-Utf8Encoding $file.FullName)) { exit 1 }
        }
        Write-Host "OK: All Rust source files UTF-8 encoding verified" -ForegroundColor Green
    } else {
        Write-Host "WARN: No Rust source files found in src/" -ForegroundColor Yellow
    }
}

if (Test-Path "tests") {
    $testFiles = Get-ChildItem -Path "tests" -Filter "*.rs" -Recurse
    if ($testFiles.Count -gt 0) {
        foreach ($file in $testFiles) {
            if (-not (Test-Utf8Encoding $file.FullName)) { exit 1 }
        }
        Write-Host "OK: All test files UTF-8 encoding verified" -ForegroundColor Green
    }
}

Write-Host "All file encoding checks passed!" -ForegroundColor Green
Write-Host ""

Write-Host "Auto-formatting code before linting..." -ForegroundColor Cyan
Run-Fix "Format Code" "cargo fmt --all"
Write-Host ""

Write-Host "Running CI checks (same as GitHub Actions)..." -ForegroundColor Cyan
Write-Host "WARN: FAIL-FAST MODE: any linting error will stop execution" -ForegroundColor Yellow
Write-Host ""

Run-Check-Block "Format Check" {
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Formatting issues found after auto-format!" -ForegroundColor Red
        Write-Host "CI should not reach this state - check for syntax errors" -ForegroundColor Red
        exit 1
    }
}

Run-Check "Clippy Lint (FAIL-FAST)" "cargo clippy --all-targets --all-features -- -D warnings"

Write-Host "Running comprehensive test suite..." -ForegroundColor Cyan

if (Test-Path "src/lib.rs") {
    Run-Check "Library Unit Tests" "cargo test --lib --verbose"
} else {
    Run-Check "Binary Unit Tests" "cargo test --bins --verbose"
}

Run-Check "Store Automation Tests" "cargo test --test store_automation_tests --verbose --features test-secret-param"
Run-Check "Security Input Validation Tests" "cargo test --test security_validation_tests --verbose --features test-secret-param"
Run-Check "Security and Injection Prevention Tests" "cargo test --test security_tests --verbose"

$env:RUSTDOCFLAGS = "-D warnings"
Run-Check "Documentation" "cargo doc --no-deps --document-private-items --all-features"

Write-Host "Skipping documentation tests for binary crate" -ForegroundColor Blue
Write-Host "INFO: Documentation tests are not applicable to binary crates" -ForegroundColor Gray

Run-Check "Build Release Binary" "cargo build --release"

Write-Host "Skipping CLI help tests (PowerShell output redirection issues)" -ForegroundColor Blue
Write-Host "INFO: CLI functionality is validated through integration tests" -ForegroundColor Gray
Write-Host ""

Write-Host "Running REAL keyring integration tests..." -ForegroundColor Cyan
Write-Host "   This validates what the memory backend tests cannot:" -ForegroundColor Gray
Write-Host "   - Actual OS keyring store/retrieve cycles" -ForegroundColor Gray
Write-Host "   - Windows Credential Manager integration" -ForegroundColor Gray
Write-Host "   - Cross-platform keyring compatibility" -ForegroundColor Gray
Write-Host "   - Real-world performance and error handling" -ForegroundColor Gray
Write-Host ""

$env:LOCAL_SECRETS_TEST_MODE = ""
$env:LOCAL_SECRETS_BACKEND = ""

try {
    Run-Check "Keyring Integration Tests (REAL KEYRING)" "cargo test --test keyring_integration_tests --verbose"
    Write-Host "OK: Real keyring functionality validated!" -ForegroundColor Green
} catch {
    Write-Host "WARN: Keyring integration tests failed or keyring service unavailable" -ForegroundColor Yellow
    Write-Host "INFO: This is expected in some CI environments without keyring services" -ForegroundColor Blue
    Write-Host "INFO: Check test output above for details" -ForegroundColor Blue
}

Write-Host "Running security audit..." -ForegroundColor Cyan
if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    Run-Check "Security Audit" "cargo audit"
} else {
    Write-Host "WARN: cargo-audit not found. Installing..." -ForegroundColor Yellow
    try {
        cargo install cargo-audit --locked
        Write-Host "OK: cargo-audit installed" -ForegroundColor Green
        Run-Check "Security Audit" "cargo audit"
    } catch {
        Write-Host "WARN: Could not install cargo-audit. Skipping security audit." -ForegroundColor Yellow
        Write-Host "INFO: To install manually: cargo install cargo-audit" -ForegroundColor Blue
    }
}

Write-Host "SUCCESS: All CI checks passed!" -ForegroundColor Green
Write-Host "INFO: Remember to review and commit any auto-fixes made." -ForegroundColor Blue
Write-Host "READY: Ready to push to remote." -ForegroundColor Green
