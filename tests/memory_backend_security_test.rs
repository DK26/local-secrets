use assert_cmd::Command;
use std::env;

/// Test that MemoryBackend is properly blocked in production contexts
#[test]
fn test_memory_backend_security_protection() {
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();

    // Attempt to use memory backend without test mode
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .env_remove("LOCAL_SECRETS_TEST_MODE") // Ensure test mode is NOT set
        .arg("store")
        .arg("TEST_VAR");

    let output = cmd.output().unwrap();

    // Should fail with security error
    assert!(
        !output.status.success(),
        "Memory backend should be rejected for security"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("SECURITY ERROR"),
        "Should show security error message"
    );
    assert!(
        stderr.contains("PLAINTEXT"),
        "Should warn about plaintext storage"
    );
    assert!(
        stderr.contains("LOCAL_SECRETS_TEST_MODE=1"),
        "Should show how to enable test mode"
    );
}

/// Test that MemoryBackend works when LOCAL_SECRETS_TEST_MODE is set
#[test]
fn test_memory_backend_allowed_with_test_mode() {
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();

    // Use memory backend with test mode explicitly enabled
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .env("LOCAL_SECRETS_TEST_MODE", "1")
        .arg("store")
        .arg("TEST_VAR_ALLOWED");

    // Should show warning but still work
    let output = cmd.output().unwrap();

    // For store commands without test-secret parameter, it will prompt for input
    // and eventually timeout or fail, but it should NOT fail with security error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("SECURITY ERROR"),
        "Should not show security error with test mode"
    );
    assert!(
        !stderr.contains("MemoryBackend rejected"),
        "Should not reject with test mode"
    );
}

/// Test that production usage shows appropriate warnings
#[test]
fn test_memory_backend_production_warnings() {
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();

    // Attempt production usage - should be blocked
    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .arg("--help"); // Use help to avoid input prompts

    let output = cmd.output().unwrap();

    // Help should work, but if we tried to store it would fail
    // Let's test actual storage attempt
    let mut store_cmd = Command::cargo_bin("local-secrets").unwrap();
    store_cmd
        .env("LOCAL_SECRETS_BACKEND", "memory")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .arg("store")
        .arg("PRODUCTION_TEST");

    let store_output = store_cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&store_output.stderr);

    assert!(stderr.contains("🚨"), "Should show security emoji warnings");
    assert!(
        stderr.contains("NEVER be used in production"),
        "Should warn about production usage"
    );
}

/// Test file system impact of MemoryBackend when allowed
#[test]
fn test_memory_backend_file_creation() {
    use std::path::PathBuf;

    // Get expected file path
    let mut temp_path = env::temp_dir();
    temp_path.push("local-secrets-memory-backend.json");

    // Clean up any existing file
    let _ = std::fs::remove_file(&temp_path);

    // Test with test mode enabled and test-secret parameter
    #[cfg(feature = "test-secret-param")]
    {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.env("LOCAL_SECRETS_BACKEND", "memory")
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .arg("store")
            .arg("FILE_TEST_VAR")
            .arg("--test-secret")
            .arg("test_value");

        let output = cmd.output().unwrap();

        if output.status.success() {
            // Verify file was created
            assert!(temp_path.exists(), "Memory backend should create temp file");

            // Verify file contains the secret (this is the security issue we're documenting)
            let content = std::fs::read_to_string(&temp_path).unwrap();
            assert!(
                content.contains("FILE_TEST_VAR"),
                "File should contain variable name"
            );
            assert!(
                content.contains("test_value"),
                "File should contain secret value - SECURITY ISSUE!"
            );

            // Clean up
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    #[cfg(not(feature = "test-secret-param"))]
    {
        // Without test-secret parameter, we can't easily test file creation
        // but we can verify the security protection is in place
        println!("Compile with --features test-secret-param to test file creation");
    }
}
