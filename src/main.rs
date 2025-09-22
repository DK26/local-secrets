use anyhow::Result;
use clap::{Parser, Subcommand};
use mimalloc::MiMalloc;
use std::env;
use std::process::ExitCode;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod backend;
mod commands;

use backend::{KeyringBackend, MemoryBackend, SecretBackend};

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

    // Determine which backend to use
    let mut backend: Box<dyn SecretBackend> = match env::var("LOCAL_SECRETS_BACKEND").as_deref() {
        Ok("memory") => Box::new(MemoryBackend::new()?),
        _ => Box::new(KeyringBackend::new()),
    };

    match cli.command {
        Some(Commands::Store { variable }) => {
            commands::store(&mut *backend, &variable)?;
        }
        Some(Commands::Delete { variable }) => {
            commands::delete(&mut *backend, &variable)?;
        }
        None => {
            // Run mode - inject environment variables and execute command
            commands::run_with_env(
                &mut *backend,
                &cli.env,
                cli.no_save_missing,
                &cli.command_args,
            )?;
        }
    }

    Ok(())
}
