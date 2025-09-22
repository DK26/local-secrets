use anyhow::{Context, Result};

/// Security validation functions for input sanitization and attack prevention
/// Based on vulnerability research from similar tools and security best practices.
/// Validates environment variable names to prevent injection attacks and system compromise
pub fn validate_env_var_name(name: &str) -> Result<()> {
    // Check for empty or whitespace-only names
    if name.trim().is_empty() {
        return Err(anyhow::anyhow!("Environment variable name cannot be empty"));
    }

    // Check length limit to prevent resource exhaustion
    if name.len() > 256 {
        return Err(anyhow::anyhow!(
            "Environment variable name too long (max 256 characters)"
        ));
    }

    // Check for null bytes and other dangerous control characters
    if name.contains('\0') {
        return Err(anyhow::anyhow!(
            "Environment variable name contains null byte"
        ));
    }

    if name.chars().any(|c| c.is_control() && c != '\t') {
        return Err(anyhow::anyhow!(
            "Environment variable name contains control characters"
        ));
    }

    // Environment variable names must not start with a number
    if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Err(anyhow::anyhow!(
            "Environment variable name cannot start with a number"
        ));
    }

    // Environment variable names must only contain valid characters
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(anyhow::anyhow!(
            "Environment variable name contains invalid characters (only A-Z, 0-9, _ allowed)"
        ));
    }

    // Check for command injection and environment pollution patterns
    let dangerous_patterns = [
        "$(",   // Command substitution
        "`",    // Backtick command substitution
        ";",    // Command separator
        "&",    // Command chaining
        "|",    // Pipe
        ">",    // Redirection
        "<",    // Input redirection
        "\\",   // Escape sequences
        "../",  // Environment variable pollution (could confuse shell scripts)
        "..\\", // Windows environment variable pollution
    ];

    for pattern in &dangerous_patterns {
        if name.contains(pattern) {
            return Err(anyhow::anyhow!(
                "Environment variable name contains dangerous pattern: {}",
                pattern
            ));
        }
    }

    // Check for critical system variables that shouldn't be overridden
    let critical_vars = [
        "PATH",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "HOME",
        "USER",
        "SHELL",
        "PWD",
        "OLDPWD",
        "IFS",
        "PS1",
        "PS2",
        "TERM",
        "TZ",
        // Windows critical variables
        "COMSPEC",
        "PATHEXT",
        "SYSTEMROOT",
        "WINDIR",
        "PROGRAMFILES",
        "APPDATA",
    ];

    for critical in &critical_vars {
        if name.eq_ignore_ascii_case(critical) {
            eprintln!(
                "Warning: Overriding critical system variable '{}' - this may cause unexpected behavior", 
                critical
            );
        }
    }

    // Check for suspicious patterns that might indicate attacks
    if name.starts_with('/') || name.starts_with('\\') || name.contains("://") {
        return Err(anyhow::anyhow!(
            "Environment variable name looks like a file path or URL"
        ));
    }

    Ok(())
}

/// Validates secret values to prevent various injection attacks
pub fn validate_secret_value(value: &str) -> Result<()> {
    // Check length limit to prevent resource exhaustion
    if value.len() > 1_048_576 {
        // 1MB limit
        return Err(anyhow::anyhow!("Secret value too long (max 1MB)"));
    }

    // Check for null bytes (could cause issues with C APIs)
    if value.contains('\0') {
        return Err(anyhow::anyhow!("Secret value contains null byte"));
    }

    // Note: We don't validate secret content beyond null bytes and length,
    // as secrets legitimately might contain any characters, including
    // special shell characters, Unicode, etc.

    Ok(())
}

/// Validates command arguments to prevent injection attacks
pub fn validate_command_args(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(anyhow::anyhow!("No command specified"));
    }

    let command = &args[0];

    // Check for empty or suspicious command
    if command.trim().is_empty() {
        return Err(anyhow::anyhow!("Empty command specified"));
    }

    // Check for obvious shell injection patterns in command
    let dangerous_command_patterns = [";", "&", "|", "`", "$(", "&&", "||", ">>", "<<"];

    for pattern in &dangerous_command_patterns {
        if command.contains(pattern) {
            return Err(anyhow::anyhow!(
                "Command contains dangerous pattern: {}",
                pattern
            ));
        }
    }

    // Validate each argument
    for (i, arg) in args.iter().enumerate() {
        // Check for null bytes
        if arg.contains('\0') {
            return Err(anyhow::anyhow!("Argument {} contains null byte", i));
        }

        // Check length
        if arg.len() > 32_768 {
            // 32KB limit per argument
            return Err(anyhow::anyhow!("Argument {} too long (max 32KB)", i));
        }
    }

    Ok(())
}

/// Validates the overall CLI arguments for security issues
pub fn validate_cli_security(env_vars: &[String], command_args: &[String]) -> Result<()> {
    // Validate environment variable names
    for env_var in env_vars {
        validate_env_var_name(env_var)
            .with_context(|| format!("Invalid environment variable name: {}", env_var))?;
    }

    // Validate command arguments if provided
    if !command_args.is_empty() {
        validate_command_args(command_args).context("Invalid command arguments")?;
    }

    // Check for suspicious combinations
    if env_vars.len() > 1000 {
        return Err(anyhow::anyhow!(
            "Too many environment variables specified (max 1000)"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_env_var_name_valid() {
        assert!(validate_env_var_name("VALID_VAR").is_ok());
        assert!(validate_env_var_name("path123").is_ok());
        assert!(validate_env_var_name("MY_SECRET").is_ok());
    }

    #[test]
    fn test_validate_env_var_name_invalid() {
        assert!(validate_env_var_name("").is_err());
        assert!(validate_env_var_name("   ").is_err());
        assert!(validate_env_var_name("VAR;rm -rf /").is_err());
        assert!(validate_env_var_name("$(echo bad)").is_err());
        assert!(validate_env_var_name("`echo bad`").is_err());
        assert!(validate_env_var_name("VAR\0NULL").is_err());
        assert!(validate_env_var_name("../etc/passwd").is_err());
    }

    #[test]
    fn test_validate_secret_value() {
        assert!(validate_secret_value("normal secret").is_ok());
        assert!(validate_secret_value("secret with spaces and symbols!@#$%").is_ok());
        assert!(validate_secret_value("").is_ok()); // Empty secrets are technically valid
        assert!(validate_secret_value("secret\0with\0nulls").is_err());

        let long_secret = "x".repeat(2_000_000);
        assert!(validate_secret_value(&long_secret).is_err());
    }

    #[test]
    fn test_validate_command_args() {
        assert!(validate_command_args(&["echo".to_string(), "hello".to_string()]).is_ok());
        assert!(validate_command_args(&[]).is_err());
        assert!(validate_command_args(&["".to_string()]).is_err());
        assert!(validate_command_args(&["echo; rm -rf /".to_string()]).is_err());
        assert!(validate_command_args(&["echo $(whoami)".to_string()]).is_err());
    }
}
