use anyhow::{Context, Result};
use std::path::Path;

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

    // Check for command injection patterns
    let dangerous_patterns = [
        "$(",   // Command substitution
        "`",    // Backtick command substitution
        ";",    // Command separator
        "&",    // Command chaining
        "|",    // Pipe
        "\\",   // Escape sequences
        "../",  // Path traversal
        "..\\", // Windows path traversal
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

/// Sanitizes file paths to prevent directory traversal attacks
pub fn sanitize_path(input_path: &str) -> Result<String> {
    // Reject obviously malicious paths
    if input_path.contains("../") || input_path.contains("..\\") {
        return Err(anyhow::anyhow!("Path contains directory traversal"));
    }

    // Normalize the path and ensure it doesn't escape
    let path = Path::new(input_path);

    // Get canonical path if possible, or normalize components
    let sanitized = if let Ok(canonical) = path.canonicalize() {
        canonical.to_string_lossy().to_string()
    } else {
        // If canonicalize fails (path doesn't exist), manually normalize
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    if components.is_empty() {
                        return Err(anyhow::anyhow!("Path attempts to escape root"));
                    }
                    components.pop();
                }
                std::path::Component::Normal(name) => {
                    components.push(name.to_string_lossy().to_string());
                }
                std::path::Component::CurDir => {
                    // Skip current directory references
                }
                _ => {
                    // Keep other components (Prefix, RootDir)
                    components.push(component.as_os_str().to_string_lossy().to_string());
                }
            }
        }
        components.join(std::path::MAIN_SEPARATOR_STR)
    };

    // Additional validation on the sanitized path
    if sanitized.is_empty() {
        return Err(anyhow::anyhow!("Empty path after sanitization"));
    }

    Ok(sanitized)
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

/// Sanitizes error messages to prevent information disclosure
pub fn sanitize_error_message(error_msg: &str) -> String {
    let mut sanitized = error_msg.to_string();

    // Remove common patterns that might leak sensitive information
    let patterns_to_remove = [
        // File system paths
        (r"(?i)/[a-zA-Z0-9/._-]*", "[PATH]"),
        (r"(?i)[A-Z]:\\[a-zA-Z0-9\\._-]*", "[PATH]"),
        // Environment variables in error messages
        (r"(?i)\$[A-Z_][A-Z0-9_]*", "[ENV_VAR]"),
        // Potential secrets (long base64-like strings)
        (r"[A-Za-z0-9+/]{20,}={0,2}", "[REDACTED]"),
        // IP addresses
        (r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b", "[IP_ADDRESS]"),
        // URLs
        (r"https?://[^\s]+", "[URL]"),
    ];

    for (pattern, replacement) in &patterns_to_remove {
        if let Ok(re) = regex::Regex::new(pattern) {
            sanitized = re.replace_all(&sanitized, *replacement).to_string();
        }
    }

    sanitized
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
    fn test_sanitize_path() {
        assert!(sanitize_path("../etc/passwd").is_err());
        assert!(sanitize_path("..\\windows\\system32").is_err());
        assert!(sanitize_path("normal_file.txt").is_ok());
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
