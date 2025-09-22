use assert_cmd::Command;
use std::fs;

/// Comprehensive integration tests for store command automation using --test-secret parameter
/// Only available when compiled with --features test-secret-param

#[test]
fn test_store_command_automation() {
    // Clean up any existing memory backend file
    let temp_path = std::env::temp_dir().join("local-secrets-memory-backend.json");
    let _ = fs::remove_file(&temp_path);

    // Test basic store functionality with test-secret parameter
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .arg("store")
        .arg("AUTOMATED_TEST_VAR")
        .arg("--test-secret")
        .arg("my_automated_secret_123");

    let output = cmd.output().unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Store command failed. stderr: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stored secret for AUTOMATED_TEST_VAR"));
}

#[test]
fn test_store_security_validation_with_test_secret() {
    // Clean up any existing memory backend file
    let temp_path = std::env::temp_dir().join("local-secrets-memory-backend.json");
    let _ = fs::remove_file(&temp_path);

    // Test that malicious variable names are rejected even with test-secret
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .arg("store")
        .arg("$(echo malicious)") // Malicious variable name
        .arg("--test-secret")
        .arg("valid_secret");

    let output = cmd.output().unwrap();
    assert!(
        !output.status.success(),
        "Should reject malicious variable name"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("dangerous pattern"),
        "Should contain security error, got: {}",
        stderr
    );
}

#[test]
fn test_store_empty_secret_validation() {
    // Clean up any existing memory backend file
    let temp_path = std::env::temp_dir().join("local-secrets-memory-backend.json");
    let _ = fs::remove_file(&temp_path);

    // Test that empty secrets are rejected
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .arg("store")
        .arg("VALID_VAR")
        .arg("--test-secret")
        .arg(""); // Empty secret

    let output = cmd.output().unwrap();
    assert!(!output.status.success(), "Should reject empty secret");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("empty") || stderr.contains("Cannot store"),
        "Should contain empty secret error, got: {}",
        stderr
    );
}

#[test]
fn test_store_unicode_secrets() {
    // Clean up any existing memory backend file
    let temp_path = std::env::temp_dir().join("local-secrets-memory-backend.json");
    let _ = fs::remove_file(&temp_path);

    // Test that Unicode secrets work correctly
    let unicode_secret = "🔐 Secret with émojis and 中文 characters 🔑";
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .arg("store")
        .arg("UNICODE_VAR")
        .arg("--test-secret")
        .arg(unicode_secret);

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "Should accept Unicode secrets. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stored secret for UNICODE_VAR"));
}
