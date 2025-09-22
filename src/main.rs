use anyhow::Result;
use clap::{Parser, Subcommand};
use mimalloc::MiMalloc;
use std::process::ExitCode;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod backend;
mod commands;
mod security;

use backend::{KeyringBackend, SecretBackend};
use security::validate_cli_security;

#[derive(Parser)]
#[command(name = "local-secrets")]
#[command(about = "Securely store secrets in your OS keyring and inject them into child processes")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Environment variable name to inject (can be used multiple times)
    #[arg(long, action = clap::ArgAction::Append)]
    env: Vec<String>,

    /// Don't save missing secrets to the keyring
    #[arg(long)]
    no_save_missing: bool,

    /// Test-only parameter: Provide secret value for automated testing (only available in test builds)
    #[cfg(feature = "test-secret-param")]
    #[arg(long, hide = true)]
    test_secret: Option<String>,

    /// Command and arguments to execute (everything after --)
    #[arg(last = true)]
    command_args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a secret in the keyring
    Store {
        /// Environment variable name
        variable: String,
        /// Test-only parameter: Provide secret value for automated testing (only available in test builds)
        #[cfg(feature = "test-secret-param")]
        #[arg(long, hide = true)]
        test_secret: Option<String>,
    },
    /// Delete a secret from the keyring  
    Delete {
        /// Environment variable name
        variable: String,
    },
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("Error: {:#}", err);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Use keyring backend for secure secret storage
    let mut backend: Box<dyn SecretBackend> = Box::new(KeyringBackend::new());

    match cli.command {
        Some(Commands::Store {
            variable,
            #[cfg(feature = "test-secret-param")]
            test_secret,
        }) => {
            #[cfg(feature = "test-secret-param")]
            {
                commands::store_with_test_value(&mut *backend, &variable, test_secret.as_deref())?;
            }
            #[cfg(not(feature = "test-secret-param"))]
            {
                commands::store(&mut *backend, &variable)?;
            }
        }
        Some(Commands::Delete { variable }) => {
            commands::delete(&mut *backend, &variable)?;
        }
        None => {
            // Check if command arguments are provided
            if cli.command_args.is_empty() && cli.env.is_empty() {
                // No subcommand and no command to run - show help
                Cli::parse_from(["local-secrets", "--help"]);
            } else if cli.command_args.is_empty() {
                anyhow::bail!("No command specified. Provide command arguments after --");
            } else {
                // Security validation before execution
                validate_cli_security(&cli.env, &cli.command_args)?;

                // Run mode - inject environment variables and execute command
                commands::run_with_env(
                    &mut *backend,
                    &cli.env,
                    cli.no_save_missing,
                    &cli.command_args,
                )?;
            }
        }
    }

    Ok(())
}
