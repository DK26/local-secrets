use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};
use std::env;
use std::process::Command;
use zeroize::Zeroize;

use crate::backend::SecretBackend;
use crate::security::{validate_env_var_name, validate_secret_value};

#[cfg(not(feature = "test-secret-param"))]
pub fn store(backend: &mut dyn SecretBackend, variable: &str) -> Result<()> {
    store_with_options(backend, variable, None)
}

#[cfg(feature = "test-secret-param")]
pub fn store_with_test_value(
    backend: &mut dyn SecretBackend,
    variable: &str,
    test_secret: Option<&str>,
) -> Result<()> {
    store_with_options(backend, variable, test_secret)
}

fn store_with_options(
    backend: &mut dyn SecretBackend,
    variable: &str,
    test_secret_override: Option<&str>,
) -> Result<()> {
    // Security: Validate variable name for injection attacks
    validate_env_var_name(variable)?;

    // Get the secret value using priority order:
    // 1. test_secret_override parameter (test builds only)
    // 2. LOCAL_SECRETS_TEST_SECRET environment variable
    // 3. User input prompt
    let secret = if let Some(test_value) = test_secret_override {
        // Test mode via parameter - use provided secret (no prompt needed)

        // Security: Validate secret value
        validate_secret_value(test_value)?;

        let mut test_value_copy = test_value.to_string();
        let secret = SecretString::new(test_value_copy.clone().into());
        test_value_copy.zeroize(); // Zero out the copy from memory
        secret
    } else if let Ok(mut test_secret) = env::var("LOCAL_SECRETS_TEST_SECRET") {
        // Test mode via environment - use provided secret (no prompt needed)

        // Security: Validate secret value
        validate_secret_value(&test_secret)?;

        let secret = SecretString::new(test_secret.clone().into());
        test_secret.zeroize(); // Zero out the test secret from memory
        secret
    } else {
        // Production mode - prompt user
        eprint!("Enter secret for {}: ", variable);
        let mut password = rpassword::read_password().context("Failed to read password")?;

        // Security: Validate secret value
        validate_secret_value(&password)?;

        let secret = SecretString::new(password.clone().into());
        password.zeroize(); // Zero out the password from memory
        secret
    };

    // Store the secret
    backend
        .store(variable, &secret)
        .context("Failed to store secret")?;

    println!("Stored secret for {}.", variable);
    Ok(())
}

pub fn delete(backend: &mut dyn SecretBackend, variable: &str) -> Result<()> {
    // Security: Validate variable name for injection attacks
    validate_env_var_name(variable)?;

    let existed = backend
        .delete(variable)
        .context("Failed to delete secret")?;

    if existed {
        println!("Deleted {}.", variable);
    } else {
        eprintln!("Secret {} not found.", variable);
        return Err(anyhow::anyhow!("Secret not found"));
    }

    Ok(())
}

pub fn run_with_env(
    backend: &mut dyn SecretBackend,
    env_vars: &[String],
    no_save_missing: bool,
    command_args: &[String],
) -> Result<()> {
    // Security validation is now performed in main.rs before calling this function
    // This is part of defense-in-depth strategy

    if !env_vars.is_empty() {
        eprintln!("Injecting env vars: {:?}", env_vars);
    }

    let mut cmd = Command::new(&command_args[0]);
    cmd.args(&command_args[1..]);

    // Inject environment variables
    for var in env_vars {
        let secret = match backend.retrieve(var)? {
            Some(secret) => secret,
            None => {
                // Secret not found, handle based on flags
                if let Ok(mut test_secret) = env::var("LOCAL_SECRETS_TEST_SECRET") {
                    // Test mode - use provided test secret
                    eprintln!("Enter secret for missing {}: ", var);

                    // Security: Validate secret value
                    validate_secret_value(&test_secret)?;

                    let secret = SecretString::new(test_secret.clone().into());
                    test_secret.zeroize(); // Zero out the test secret from memory

                    if !no_save_missing {
                        backend.store(var, &secret)?;
                        eprintln!("Stored secret for {}.", var);
                    }

                    secret
                } else if env::var("LOCAL_SECRETS_TEST_MODE").is_ok() {
                    // Test mode but no test secret provided - this should fail
                    return Err(anyhow::anyhow!("Secret {} not found", var));
                } else {
                    // Production mode - prompt user
                    eprint!("Enter secret for missing {}: ", var);
                    let mut password =
                        rpassword::read_password().context("Failed to read password")?;

                    // Security: Validate secret value
                    validate_secret_value(&password)?;

                    let secret = SecretString::new(password.clone().into());
                    password.zeroize(); // Zero out the password from memory

                    if !no_save_missing {
                        backend.store(var, &secret)?;
                        eprintln!("Stored secret for {}.", var);
                    }

                    secret
                }
            }
        };

        cmd.env(var, secret.expose_secret());
    }

    // Execute the command
    let mut child = cmd.spawn().context("Failed to spawn child process")?;

    let exit_status = child.wait().context("Failed to wait for child process")?;

    // Defensive: Handle exit codes gracefully, never panic
    if !exit_status.success() {
        let code = exit_status.code().unwrap_or(1);
        // Defensive: Ensure exit code is in valid range
        let safe_code = if !(0..=255).contains(&code) { 1 } else { code };
        std::process::exit(safe_code);
    }

    Ok(())
}
