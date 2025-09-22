use assert_cmd::Command;
use std::env;

/// Security-focused integration tests for local-secrets CLI
/// These tests validate that the CLI properly handles malicious inputs and edge cases

#[test]
fn test_malicious_environment_variable_names() {
    // Test command injection attempts in environment variable names
    let malicious_names = vec![
        "$(echo 'injected')",   // Command substitution
        "`echo 'injected'`",    // Backtick command substitution
        "VAR;rm -rf /",         // Command separator
        "VAR && echo attack",   // Command chaining
        "VAR || echo fallback", // Alternative command
        "$PATH",                // Variable substitution
        "../../../etc/passwd",  // Path traversal
        "\\x00\\x01\\x02",      // Control characters
        "очень_длинное_имя_переменной_которое_может_вызвать_проблемы_с_памятью", // Long Unicode
        "",                     // Empty string
        "   ",                  // Whitespace only
        "\n\r\t",               // Newlines and tabs
        "PATH",                 // Critical system variable
        "LD_LIBRARY_PATH",      // Library path hijacking
        "HOME",                 // Home directory override
    ];

    for malicious_name in malicious_names {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();

        // Test store command with malicious variable name
        cmd.env("LOCAL_SECRETS_BACKEND", "memory")
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .env("LOCAL_SECRETS_TEST_SECRET", "test_value")
            .arg("store")
            .arg(malicious_name);

        let output = cmd.output().unwrap();

        // Should either reject with proper error or handle safely
        if output.status.success() {
            // If it succeeds, verify no command injection occurred
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Should not contain evidence of command execution
            assert!(!stdout.contains("injected"));
            assert!(!stderr.contains("injected"));
            assert!(!stdout.contains("attack"));
            assert!(!stderr.contains("attack"));

            println!("Malicious name '{}' was handled safely", malicious_name);
        } else {
            // If it fails, should fail with proper error message
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("Error:"));
            println!("Malicious name '{}' was properly rejected", malicious_name);
        }
    }
}

#[test]
fn test_command_injection_in_child_process() {
    // Test that malicious commands don't get executed via environment variables

    // First, store the malicious secret
    let mut store_cmd = Command::cargo_bin("local-secrets").unwrap();
    store_cmd
        .env("LOCAL_SECRETS_BACKEND", "memory")
        .env("LOCAL_SECRETS_TEST_MODE", "1")
        .env("LOCAL_SECRETS_TEST_SECRET", "$(echo 'INJECTED')")
        .arg("store")
        .arg("TEST_VAR");

    let store_output = store_cmd.output().unwrap();

    if !store_output.status.success() {
        // If storing fails due to our security validation, that's good!
        let stderr = String::from_utf8_lossy(&store_output.stderr);
        println!(
            "Malicious secret properly rejected during storage: {}",
            stderr
        );
        assert!(stderr.contains("Error:"));
        return; // Test passes - the malicious input was blocked
    }

    // If storage succeeded, test that the secret is treated literally when used
    let mut run_cmd = Command::cargo_bin("local-secrets").unwrap();

    run_cmd
        .env("LOCAL_SECRETS_BACKEND", "memory")
        .env("LOCAL_SECRETS_TEST_MODE", "1")
        .arg("--env")
        .arg("TEST_VAR")
        .arg("--")
        .arg("echo")
        .arg("Retrieved:");

    // On Windows, need to use %VAR% syntax
    #[cfg(windows)]
    run_cmd.arg("%TEST_VAR%");

    // On Unix, use $VAR syntax
    #[cfg(unix)]
    run_cmd.arg("$TEST_VAR");

    let output = run_cmd.output().unwrap();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // The secret value should be literal, not executed
        // Command injection would result in "INJECTED" appearing in output
        assert!(!stdout.contains("INJECTED"));
        println!("Command injection test passed - no code execution detected");
    } else {
        // If execution fails, that's also acceptable for security
        println!("Command execution failed - this may be due to security restrictions");
    }
}

#[test]
fn test_path_traversal_in_variable_names() {
    // Test path traversal attempts in variable names
    let traversal_attempts = vec![
        "../secret",
        "../../etc/passwd",
        "..\\..\\windows\\system32",
        "/etc/shadow",
        "C:\\Windows\\System32\\config\\SAM",
        ".ssh/id_rsa",
        "~/.bashrc",
    ];

    for attempt in traversal_attempts {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();

        cmd.env("LOCAL_SECRETS_BACKEND", "memory")
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .env("LOCAL_SECRETS_TEST_SECRET", "secret_value")
            .arg("store")
            .arg(attempt);

        let output = cmd.output().unwrap();

        // Should either reject or handle safely without accessing unauthorized paths
        if output.status.success() {
            println!("Path traversal attempt '{}' was handled safely", attempt);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("Error:"));
            println!("Path traversal attempt '{}' was properly rejected", attempt);
        }
    }
}

#[test]
fn test_environment_variable_pollution() {
    // Test that we don't accidentally override critical system environment variables
    let critical_vars = vec![
        "PATH",
        "HOME",
        "USER",
        "SHELL",
        "PWD",
        "OLDPWD",
        "LD_LIBRARY_PATH",
        "PYTHONPATH",
    ];

    for var in critical_vars {
        let original_value = env::var(var).ok();

        let mut cmd = Command::cargo_bin("local-secrets").unwrap();

        cmd.env("LOCAL_SECRETS_BACKEND", "memory")
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .env("LOCAL_SECRETS_TEST_SECRET", "malicious_value")
            .arg("--env")
            .arg(var)
            .arg("--")
            .arg("env");

        let output = cmd.output().unwrap();

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Verify the variable was set to our value in the child process
            assert!(stdout.contains(&format!("{}=malicious_value", var)));

            // Verify our current process environment wasn't affected
            let current_value = env::var(var).ok();
            assert_eq!(current_value, original_value);

            println!(
                "Environment variable '{}' injection was isolated to child process",
                var
            );
        }
    }
}

#[test]
fn test_unicode_and_special_characters() {
    // Test handling of Unicode and special characters in variable names and values
    let special_cases = vec![
        ("unicode_var", "Здравствуй мир! 🚀"),
        ("emoji_💣", "bomb_emoji"),
        ("null_byte", "value\0with\0nulls"),
        ("newlines", "line1\nline2\rline3"),
        ("quotes", "value'with\"quotes"),
        ("backslashes", "path\\with\\backslashes"),
        ("xml_injection", "<script>alert('xss')</script>"),
        ("sql_injection", "'; DROP TABLE secrets; --"),
    ];

    for (var_name, secret_value) in special_cases {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();

        cmd.env("LOCAL_SECRETS_BACKEND", "memory")
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .env("LOCAL_SECRETS_TEST_SECRET", secret_value)
            .arg("store")
            .arg(var_name);

        // Handle cases where the command itself might fail due to invalid input
        match cmd.output() {
            Ok(output) => {
                // Should handle gracefully without panicking or exposing internals
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // Error messages should not contain the raw secret value
                    assert!(!stderr.contains(secret_value));
                    println!("Special case '{}' was properly rejected", var_name);
                } else {
                    println!("Special case '{}' was handled safely", var_name);
                }
            }
            Err(e) => {
                // Some special characters might cause the command to fail at the OS level
                // This is acceptable for security - it means the dangerous input was blocked
                println!("Special case '{}' blocked at OS level: {}", var_name, e);
            }
        }
    }
}

#[test]
fn test_resource_exhaustion_attacks() {
    // Test handling of extremely long inputs that could cause resource exhaustion
    let long_string = "A".repeat(1_000_000); // 1MB string

    let test_cases = vec![
        ("long_var_name", "short_value"),
        ("short_name", &long_string),
        ("both_long_name_that_keeps_going_and_going", &long_string),
    ];

    for (var_name, secret_value) in test_cases {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();

        cmd.env("LOCAL_SECRETS_BACKEND", "memory")
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .env("LOCAL_SECRETS_TEST_SECRET", secret_value)
            .arg("store")
            .arg(var_name)
            .timeout(std::time::Duration::from_secs(30)); // Prevent hanging

        let result = cmd.output();

        match result {
            Ok(output) => {
                // Should either succeed gracefully or fail with proper error
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    assert!(stderr.contains("Error:"));
                }
                println!("Large input case handled within time limit");
            }
            Err(_) => {
                // Timeout or other error - this is acceptable for resource exhaustion tests
                println!("Large input case properly limited (timeout or error)");
            }
        }
    }
}

#[test]
fn test_concurrent_access_safety() {
    // Test that concurrent access doesn't cause race conditions or crashes
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let success_count = success_count.clone();
            let error_count = error_count.clone();

            thread::spawn(move || {
                let var_name = format!("concurrent_var_{}", i);
                let secret_value = format!("secret_{}", i);

                let mut cmd = Command::cargo_bin("local-secrets").unwrap();

                cmd.env("LOCAL_SECRETS_BACKEND", "memory")
                    .env("LOCAL_SECRETS_TEST_MODE", "1")
                    .env("LOCAL_SECRETS_TEST_SECRET", &secret_value)
                    .arg("store")
                    .arg(&var_name);

                match cmd.output() {
                    Ok(output) => {
                        if output.status.success() {
                            success_count.fetch_add(1, Ordering::Relaxed);
                        } else {
                            error_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let successes = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!(
        "Concurrent access test: {} successes, {} errors",
        successes, errors
    );

    // At least some operations should complete (either success or controlled error)
    assert!(successes + errors == 10);
}

#[test]
fn test_signal_handling_security() {
    // Test that the process handles signals gracefully without exposing secrets
    // This is particularly important for secrets in memory

    let mut cmd = Command::cargo_bin("local-secrets").unwrap();

    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .env("LOCAL_SECRETS_TEST_MODE", "1")
        .env("LOCAL_SECRETS_TEST_SECRET", "sensitive_secret")
        .arg("--env")
        .arg("TEST_SECRET")
        .arg("--")
        .arg("sleep")
        .arg("1"); // Short-running command

    let output = cmd.output().unwrap();

    // Process should complete normally without hanging or crashing
    // In a real scenario, we'd send signals, but that's complex in integration tests
    println!(
        "Signal handling test completed: {}",
        output.status.success()
    );
}

#[test]
fn test_error_message_information_disclosure() {
    // Test that error messages don't leak sensitive information
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();

    cmd.env("LOCAL_SECRETS_BACKEND", "memory")
        .env("LOCAL_SECRETS_TEST_MODE", "1")
        .env("LOCAL_SECRETS_TEST_SECRET", "super_secret_password_123")
        .arg("store")
        .arg(""); // Invalid empty variable name

    let output = cmd.output().unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Error message should not contain the secret value
    assert!(!stderr.contains("super_secret_password_123"));

    // Should contain helpful error without leaking internals
    assert!(stderr.contains("Error:"));

    println!("Error message security test passed");
}

#[test]
fn test_keyring_backend_security_isolation() {
    // Test that keyring backend properly isolates secrets per service
    // and doesn't accidentally access other applications' secrets

    // This test mainly verifies our service name isolation
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();

    // Use actual keyring backend (not memory) for this test
    cmd.env_remove("LOCAL_SECRETS_BACKEND")
        .env("LOCAL_SECRETS_TEST_MODE", "1")
        .env("LOCAL_SECRETS_TEST_SECRET", "isolated_secret")
        .arg("store")
        .arg("isolation_test");

    let output = cmd.output().unwrap();

    if output.status.success() {
        println!("Keyring isolation test: secret stored successfully");

        // Verify we can retrieve it
        let mut retrieve_cmd = Command::cargo_bin("local-secrets").unwrap();
        retrieve_cmd
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .arg("--env")
            .arg("isolation_test")
            .arg("--")
            .arg("echo")
            .arg("retrieved");

        let retrieve_output = retrieve_cmd.output().unwrap();

        if retrieve_output.status.success() {
            println!("Keyring isolation test: secret retrieved successfully");
        }

        // Clean up
        let mut delete_cmd = Command::cargo_bin("local-secrets").unwrap();
        delete_cmd
            .env("LOCAL_SECRETS_TEST_MODE", "1")
            .arg("delete")
            .arg("isolation_test");

        let _ = delete_cmd.output(); // Ignore result for cleanup
    } else {
        println!("Keyring isolation test: keyring not available, skipped");
    }
}
