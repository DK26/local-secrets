use assert_cmd::Command;

#[cfg(target_os = "windows")]
const LONG_SECRET_SIZE: usize = 30_000;
#[cfg(not(target_os = "windows"))]
const LONG_SECRET_SIZE: usize = 100_000;

/// Comprehensive integration tests for store command automation using --test-secret parameter
/// Tests real-world CI/CD scenarios using secure keyring backend
#[test]
fn test_store_command_automation_with_keyring() {
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("AUTOMATED_TEST_VAR")
        .arg("--test-secret")
        .arg("my_automated_secret_123");

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "Store command failed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Stored secret for AUTOMATED_TEST_VAR"),
        "Unexpected stdout: {stdout}"
    );
}

#[test]
fn test_store_security_validation_with_keyring() {
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("$(echo malicious)")
        .arg("--test-secret")
        .arg("valid_secret");

    let output = cmd.output().unwrap();
    assert!(
        !output.status.success(),
        "Should reject malicious variable name"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("dangerous pattern") || stderr.contains("invalid characters"),
        "Should indicate security issue, got: {}",
        stderr
    );
}

#[test]
fn test_path_traversal_rejection_in_store() {
    let malicious_names = vec!["../secret", "../../etc/passwd", "..\\..\\windows\\system32"];

    for name in malicious_names {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(name)
            .arg("--test-secret")
            .arg("secret_value");

        let output = cmd.output().unwrap();
        assert!(
            !output.status.success(),
            "Should reject path traversal: {name}"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("dangerous pattern") || stderr.contains("invalid"));
    }
}

#[test]
fn test_empty_secret_rejection() {
    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("VALID_VAR")
        .arg("--test-secret")
        .arg("");

    let output = cmd.output().unwrap();
    assert!(!output.status.success(), "Should reject empty secret");
}

#[test]
fn test_very_long_secret_handling() {
    // Windows cannot forward >32k characters through CreateProcess, so the test uses a
    // slightly smaller payload on that platform while still exercising large-secret handling.
    let long_secret = "A".repeat(LONG_SECRET_SIZE);

    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("LONG_SECRET_VAR")
        .arg("--test-secret")
        .arg(&long_secret);

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "Long secret store failed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Stored secret for LONG_SECRET_VAR"),
        "Unexpected stdout: {stdout}"
    );
}

#[test]
fn test_unicode_secret_handling() {
    // Use explicit Unicode escapes to avoid encoding issues on Windows terminals.
    let unicode_secret = "\u{1F512}\u{5BC6}\u{5B89}\u{5168}";

    let mut cmd = Command::cargo_bin("local-secrets").unwrap();
    cmd.arg("store")
        .arg("UNICODE_SECRET_VAR")
        .arg("--test-secret")
        .arg(unicode_secret);

    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "Unicode secret test failed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Stored secret for UNICODE_SECRET_VAR"),
        "Unexpected stdout: {stdout}"
    );
}

#[test]
fn test_ci_cd_automation_scenario() {
    let test_vars = vec![
        ("API_KEY", "sk-1234567890abcdef"),
        ("DATABASE_URL", "postgresql://user:pass@localhost/db"),
        ("JWT_SECRET", "super-secret-jwt-key-here"),
    ];

    for (var_name, secret_value) in test_vars {
        let mut cmd = Command::cargo_bin("local-secrets").unwrap();
        cmd.arg("store")
            .arg(var_name)
            .arg("--test-secret")
            .arg(secret_value);

        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "CI/CD scenario failed for {var_name}. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(&format!("Stored secret for {var_name}")),
            "Unexpected stdout for {var_name}: {stdout}"
        );
    }
}
