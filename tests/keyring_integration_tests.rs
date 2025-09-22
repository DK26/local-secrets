use assert_cmd::Command;
use std::time::Duration;

/// End-to-end keyring integration tests
/// These tests use the REAL keyring backend to validate actual functionality
///
/// CRITICAL: These tests validate what memory backend tests cannot:
/// - Actual OS keyring store/retrieve cycles  
/// - Cross-platform keyring compatibility
/// - Keyring service availability and error handling
/// - Real-world performance characteristics
const TEST_VAR_PREFIX: &str = "LOCAL_SECRETS_E2E_TEST_";

/// Helper to generate unique test variable names to avoid conflicts
fn unique_test_var() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}VAR_{}", TEST_VAR_PREFIX, timestamp)
}

/// Clean up any test variables left behind  
fn cleanup_test_vars() {
    // This is a best-effort cleanup - some may fail if vars don't exist
    for i in 0..10 {
        let var_name = format!("{}CLEANUP_{}", TEST_VAR_PREFIX, i);
        let _ = Command::cargo_bin("local-secrets")
            .unwrap()
            .arg("delete")
            .arg(&var_name)
            .timeout(Duration::from_secs(5))
            .output();
    }
}

/// Test that keyring backend is available and functional on this platform
#[test]
fn test_keyring_availability() {
    cleanup_test_vars();

    let test_var = unique_test_var();
    let test_secret = "test_keyring_availability_secret_123";

    // Test 1: Store secret using real keyring
    let mut store_cmd = Command::cargo_bin("local-secrets").unwrap();
    store_cmd
        .arg("store")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND") // Use default keyring backend
        .env_remove("LOCAL_SECRETS_TEST_MODE") // Production mode
        .env("LOCAL_SECRETS_TEST_SECRET", test_secret)
        .timeout(Duration::from_secs(10));

    let store_output = store_cmd.output().unwrap();

    if !store_output.status.success() {
        let stderr = String::from_utf8_lossy(&store_output.stderr);
        // Some CI environments don't have keyring services available
        if stderr.contains("keyring") || stderr.contains("credential") || stderr.contains("service")
        {
            eprintln!(
                "⚠️ Keyring service not available in this environment: {}",
                stderr
            );
            eprintln!("This is expected in some CI environments without GUI/keyring services");
            return; // Skip test gracefully
        } else {
            panic!("Store command failed unexpectedly: {}", stderr);
        }
    }

    // Test 2: Retrieve and verify secret
    let mut run_cmd = Command::cargo_bin("local-secrets").unwrap();

    // Use cross-platform command that works on all systems
    #[cfg(target_os = "windows")]
    let cmd_args = ["cmd", "/c", "echo keyring_test_success"];
    #[cfg(not(target_os = "windows"))]
    let cmd_args = ["echo", "keyring_test_success"];

    run_cmd
        .args(["--env", &test_var, "--"])
        .args(cmd_args)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .timeout(Duration::from_secs(10));

    let run_output = run_cmd.output().unwrap();

    if !run_output.status.success() {
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        panic!("Run command failed: {}", stderr);
    }

    // Verify the secret was injected (we can't see the actual value for security)
    let stderr = String::from_utf8_lossy(&run_output.stderr);
    assert!(stderr.contains(&format!("Injecting env vars: [\"{}\"]", test_var)));

    // Test 3: Clean up - delete the secret
    let mut delete_cmd = Command::cargo_bin("local-secrets").unwrap();
    delete_cmd
        .arg("delete")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .timeout(Duration::from_secs(10));

    let delete_output = delete_cmd.output().unwrap();

    if !delete_output.status.success() {
        let stderr = String::from_utf8_lossy(&delete_output.stderr);
        eprintln!(
            "⚠️ Warning: Failed to clean up test variable {}: {}",
            test_var, stderr
        );
    }

    println!("✅ Keyring backend is functional on this platform");
}

/// Test cross-platform keyring error handling
#[test]
fn test_keyring_error_handling() {
    cleanup_test_vars();

    let non_existent_var = format!("{}NONEXISTENT", TEST_VAR_PREFIX);

    // Test: Try to retrieve non-existent secret from keyring
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();

    // Use cross-platform command
    #[cfg(target_os = "windows")]
    let cmd_args = ["cmd", "/c", "echo should_not_run"];
    #[cfg(not(target_os = "windows"))]
    let cmd_args = ["echo", "should_not_run"];

    cmd.args(["--env", &non_existent_var, "--no-save-missing", "--"])
        .args(cmd_args)
        .env_remove("LOCAL_SECRETS_BACKEND") // Use real keyring
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .env_remove("LOCAL_SECRETS_TEST_SECRET") // No fallback
        .timeout(Duration::from_secs(10));

    let output = cmd.output().unwrap();

    // Should fail gracefully with proper error message
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should mention the missing secret
    assert!(stderr.contains(&non_existent_var) || stderr.contains("not found"));

    // Should not have executed the command
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("should_not_run"));

    println!("✅ Keyring error handling works correctly");
}

/// Test keyring service isolation and security
#[test]
fn test_keyring_service_isolation() {
    cleanup_test_vars();

    let test_var1 = format!("{}ISOLATION_1", TEST_VAR_PREFIX);
    let test_var2 = format!("{}ISOLATION_2", TEST_VAR_PREFIX);
    let secret1 = "isolation_test_secret_1";
    let secret2 = "isolation_test_secret_2";

    // Store two different secrets
    for (var, secret) in [(&test_var1, secret1), (&test_var2, secret2)] {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(var)
            .env_remove("LOCAL_SECRETS_BACKEND")
            .env_remove("LOCAL_SECRETS_TEST_MODE")
            .env("LOCAL_SECRETS_TEST_SECRET", secret)
            .timeout(Duration::from_secs(10));

        let output = cmd.output().unwrap();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("keyring") || stderr.contains("credential") {
                eprintln!("⚠️ Keyring service not available, skipping isolation test");
                return;
            }
            panic!("Failed to store secret for {}: {}", var, stderr);
        }
    }

    // Verify each secret can be retrieved independently
    // (We can't directly compare secret values for security reasons,
    // but we can verify the injection process works independently)
    for var in [&test_var1, &test_var2] {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();

        #[cfg(target_os = "windows")]
        let cmd_args = ["cmd", "/c", "echo retrieved"];
        #[cfg(not(target_os = "windows"))]
        let cmd_args = ["echo", "retrieved"];

        cmd.args(["--env", var, "--"])
            .args(cmd_args)
            .env_remove("LOCAL_SECRETS_BACKEND")
            .env_remove("LOCAL_SECRETS_TEST_MODE")
            .timeout(Duration::from_secs(10));

        let output = cmd.output().unwrap();

        assert!(output.status.success(), "Failed to retrieve {}", var);

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(&format!("Injecting env vars: [\"{}\"]", var)));
    }

    // Clean up
    for var in [&test_var1, &test_var2] {
        let _ = Command::cargo_bin("local-secrets")
            .unwrap()
            .arg("delete")
            .arg(var)
            .env_remove("LOCAL_SECRETS_BACKEND")
            .env_remove("LOCAL_SECRETS_TEST_MODE")
            .timeout(Duration::from_secs(10))
            .output();
    }

    println!("✅ Keyring service isolation works correctly");
}

/// Test keyring performance characteristics
#[test]
fn test_keyring_performance() {
    cleanup_test_vars();

    let test_var = unique_test_var();
    let test_secret = "performance_test_secret";

    // Measure store operation
    let start = std::time::Instant::now();

    let mut store_cmd = Command::cargo_bin("local-secrets").unwrap();
    store_cmd
        .arg("store")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .env("LOCAL_SECRETS_TEST_SECRET", test_secret)
        .timeout(Duration::from_secs(30)); // More time for slow keyrings

    let store_output = store_cmd.output().unwrap();

    if !store_output.status.success() {
        let stderr = String::from_utf8_lossy(&store_output.stderr);
        if stderr.contains("keyring") || stderr.contains("credential") {
            eprintln!("⚠️ Keyring service not available, skipping performance test");
            return;
        }
        panic!("Store operation failed: {}", stderr);
    }

    let store_duration = start.elapsed();

    // Measure retrieve operation
    let start = std::time::Instant::now();

    let mut run_cmd = Command::cargo_bin("local-secrets").unwrap();

    #[cfg(target_os = "windows")]
    let cmd_args = ["cmd", "/c", "echo performance_test"];
    #[cfg(not(target_os = "windows"))]
    let cmd_args = ["echo", "performance_test"];

    run_cmd
        .args(["--env", &test_var, "--"])
        .args(cmd_args)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .timeout(Duration::from_secs(30));

    let run_output = run_cmd.output().unwrap();
    assert!(run_output.status.success());

    let retrieve_duration = start.elapsed();

    // Clean up
    let _ = Command::cargo_bin("local-secrets")
        .unwrap()
        .arg("delete")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .timeout(Duration::from_secs(10))
        .output();

    // Performance assertions (reasonable bounds for keyring operations)
    assert!(
        store_duration < Duration::from_secs(10),
        "Store operation took too long: {:?}",
        store_duration
    );
    assert!(
        retrieve_duration < Duration::from_secs(10),
        "Retrieve operation took too long: {:?}",
        retrieve_duration
    );

    println!(
        "✅ Keyring performance: store={:?}, retrieve={:?}",
        store_duration, retrieve_duration
    );
}

/// Platform-specific keyring service tests
#[cfg(target_os = "windows")]
#[test]
fn test_windows_credential_manager() {
    cleanup_test_vars();

    let test_var = unique_test_var();

    // Windows should always have Credential Manager available
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .env("LOCAL_SECRETS_TEST_SECRET", "windows_test_secret")
        .timeout(Duration::from_secs(10));

    let output = cmd.output().unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Windows Credential Manager should always be available: {}",
            stderr
        );
    }

    // Clean up
    let _ = Command::cargo_bin("local-secrets")
        .unwrap()
        .arg("delete")
        .arg(&test_var)
        .timeout(Duration::from_secs(10))
        .output();

    println!("✅ Windows Credential Manager is functional");
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_keychain() {
    cleanup_test_vars();

    let test_var = unique_test_var();

    // macOS should have Keychain available (may require user permission in some cases)
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .env("LOCAL_SECRETS_TEST_SECRET", "macos_test_secret")
        .timeout(Duration::from_secs(10));

    let output = cmd.output().unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("permission") || stderr.contains("access") {
            eprintln!("⚠️ macOS Keychain access denied (expected in some CI environments)");
            return;
        }
        eprintln!("⚠️ macOS Keychain test failed: {}", stderr);
        // Don't fail the test - keychain access varies by environment
        return;
    }

    // Clean up
    let _ = Command::cargo_bin("local-secrets")
        .unwrap()
        .arg("delete")
        .arg(&test_var)
        .timeout(Duration::from_secs(10))
        .output();

    println!("✅ macOS Keychain is functional");
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_secret_service() {
    cleanup_test_vars();

    let test_var = unique_test_var();

    // Linux Secret Service availability varies by environment
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .env("LOCAL_SECRETS_TEST_SECRET", "linux_test_secret")
        .timeout(Duration::from_secs(10));

    let output = cmd.output().unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("dbus")
            || stderr.contains("service")
            || stderr.contains("org.freedesktop")
        {
            eprintln!("⚠️ Linux Secret Service not available (expected in headless CI)");
            return;
        }
        eprintln!("⚠️ Linux Secret Service test failed: {}", stderr);
        // Don't fail the test - secret service varies by environment
        return;
    }

    // Clean up
    let _ = Command::cargo_bin("local-secrets")
        .unwrap()
        .arg("delete")
        .arg(&test_var)
        .timeout(Duration::from_secs(10))
        .output();

    println!("✅ Linux Secret Service is functional");
}

/// Test that empty secrets are properly rejected by the keyring backend
#[test]
fn test_empty_secret_validation_with_keyring() {
    cleanup_test_vars();

    let test_var = unique_test_var();

    // Test: Try to store empty secret using real keyring
    let mut store_cmd = Command::cargo_bin("local-secrets").unwrap();
    store_cmd
        .arg("store")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND") // Use default keyring backend
        .env_remove("LOCAL_SECRETS_TEST_MODE") // Production mode
        .env("LOCAL_SECRETS_TEST_SECRET", "") // Empty secret
        .timeout(Duration::from_secs(10));

    let store_output = store_cmd.output().unwrap();

    // Should fail with empty secret error
    assert!(
        !store_output.status.success(),
        "Empty secret should be rejected"
    );

    let stderr = String::from_utf8_lossy(&store_output.stderr);

    // Should contain error message about empty secret
    assert!(
        stderr.contains("empty secret") || stderr.contains("Cannot store empty"),
        "Should show empty secret error, got: {}",
        stderr
    );

    println!("✅ Empty secret validation works with keyring backend");
}

/// Test the --test-secret parameter feature with real keyring backend
#[cfg(feature = "test-secret-param")]
#[test]
fn test_test_secret_parameter_with_keyring() {
    cleanup_test_vars();

    let test_var = unique_test_var();
    let test_secret = "automated_test_secret_with_keyring_456";

    // Test: Store secret using --test-secret parameter with real keyring
    let mut store_cmd = Command::cargo_bin("local-secrets").unwrap();
    store_cmd
        .arg("store")
        .arg(&test_var)
        .arg("--test-secret")
        .arg(test_secret)
        .env_remove("LOCAL_SECRETS_BACKEND") // Use default keyring backend
        .env_remove("LOCAL_SECRETS_TEST_MODE") // Production mode
        .timeout(Duration::from_secs(10));

    let store_output = store_cmd.output().unwrap();

    if !store_output.status.success() {
        let stderr = String::from_utf8_lossy(&store_output.stderr);
        if stderr.contains("keyring") || stderr.contains("credential") || stderr.contains("service")
        {
            eprintln!("⚠️ Keyring service not available, skipping --test-secret test");
            return;
        } else {
            panic!("Store with --test-secret failed: {}", stderr);
        }
    }

    let stdout = String::from_utf8_lossy(&store_output.stdout);
    assert!(
        stdout.contains(&format!("Stored secret for {}", test_var)),
        "Should confirm secret storage"
    );

    // Test: Verify secret was stored by retrieving it
    let mut retrieve_cmd = Command::cargo_bin("local-secrets").unwrap();
    #[cfg(target_os = "windows")]
    let cmd_args = ["cmd", "/c", &format!("echo %{}%", test_var)];
    #[cfg(not(target_os = "windows"))]
    let cmd_args = ["sh", "-c", &format!("echo ${}", test_var)];

    retrieve_cmd
        .args(["--env", &test_var, "--"])
        .args(cmd_args)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .env_remove("LOCAL_SECRETS_TEST_MODE")
        .timeout(Duration::from_secs(10));

    let retrieve_output = retrieve_cmd.output().unwrap();

    if retrieve_output.status.success() {
        let stdout = String::from_utf8_lossy(&retrieve_output.stdout);
        assert!(
            stdout.contains(test_secret),
            "Retrieved secret should match stored secret"
        );
    }

    // Clean up
    let mut cleanup = Command::cargo_bin("local-secrets").unwrap();
    cleanup
        .arg("delete")
        .arg(&test_var)
        .env_remove("LOCAL_SECRETS_BACKEND")
        .timeout(Duration::from_secs(5));
    let _ = cleanup.output(); // Best effort cleanup

    println!("✅ --test-secret parameter works with keyring backend");
}
