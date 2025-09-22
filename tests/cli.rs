use assert_cmd::Command as AssertCommand;
use predicates::prelude::*;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::sync::OnceLock;

const BACKEND_ENV: &str = "LOCAL_SECRETS_BACKEND";
const TEST_MODE_ENV: &str = "LOCAL_SECRETS_TEST_MODE";
const TEST_SECRET_ENV: &str = "LOCAL_SECRETS_TEST_SECRET";

fn target_dir() -> PathBuf {
    std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target"))
}

fn env_probe() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let bin_dir = target_dir().join("test-bin");
        std::fs::create_dir_all(&bin_dir).expect("create test-bin directory");
        let bin_path = bin_dir.join(format!("print-env{}", std::env::consts::EXE_SUFFIX));
        compile_env_probe(&bin_path).expect("compile env probe helper");
        bin_path
    })
    .clone()
}

fn compile_env_probe(bin_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let source = r#"
        use std::{env, process};

        fn main() {
            let var = env::args().nth(1).expect("variable name required");
            match env::var(&var) {
                Ok(value) => print!("{}", value),
                Err(_) => {
                    eprintln!("missing env: {}", var);
                    process::exit(2);
                }
            }
        }
    "#;

    let source_path = bin_path.with_extension("rs");
    std::fs::write(&source_path, source)?;

    let status = StdCommand::new("rustc")
        .arg("--edition=2021")
        .arg(&source_path)
        .arg("-o")
        .arg(bin_path)
        .status()?;

    if !status.success() {
        panic!("failed to compile env probe helper");
    }

    Ok(())
}

fn local_secrets_cmd() -> Result<AssertCommand, Box<dyn Error>> {
    let mut cmd = AssertCommand::cargo_bin("local-secrets")?;
    cmd.env(TEST_MODE_ENV, "1");
    Ok(cmd)
}

#[test]
fn store_then_run_injects_secret_from_memory_backend() -> Result<(), Box<dyn Error>> {
    let helper = env_probe();

    let mut store = local_secrets_cmd()?;
    store
        .env(BACKEND_ENV, "memory")
        .env(TEST_SECRET_ENV, "super-secret-token")
        .arg("store")
        .arg("GITHUB_PAT");
    store
        .assert()
        .success()
        .stdout(predicate::str::contains("Stored secret for GITHUB_PAT"))
        .stderr(predicate::str::contains("Enter secret for GITHUB_PAT"));

    let mut run = local_secrets_cmd()?;
    run.env(BACKEND_ENV, "memory")
        .env_remove(TEST_SECRET_ENV)
        .args(["--env", "GITHUB_PAT", "--"])
        .arg(&helper)
        .arg("GITHUB_PAT");

    let stderr_pred = predicate::str::contains("Injecting env vars: [\"GITHUB_PAT\"]")
        .and(predicate::str::contains("Enter secret").not());

    run.assert()
        .success()
        .stdout(predicate::str::contains("super-secret-token"))
        .stderr(stderr_pred);

    Ok(())
}

#[test]
fn no_save_missing_requires_secret_each_time() -> Result<(), Box<dyn Error>> {
    let helper = env_probe();

    let mut first = local_secrets_cmd()?;
    first
        .env(BACKEND_ENV, "memory")
        .env(TEST_SECRET_ENV, "transient-1")
        .args(["--env", "API_KEY", "--no-save-missing", "--"])
        .arg(&helper)
        .arg("API_KEY");

    first
        .assert()
        .success()
        .stdout(predicate::str::contains("transient-1"))
        .stderr(predicate::str::contains("Enter secret for missing API_KEY"));

    let mut second = local_secrets_cmd()?;
    second
        .env(BACKEND_ENV, "memory")
        .env(TEST_SECRET_ENV, "transient-2")
        .args(["--env", "API_KEY", "--no-save-missing", "--"])
        .arg(&helper)
        .arg("API_KEY");

    let stderr_pred = predicate::str::contains("Enter secret for missing API_KEY")
        .and(predicate::str::contains("Stored secret for API_KEY").not());

    second
        .assert()
        .success()
        .stdout(predicate::str::contains("transient-2"))
        .stderr(stderr_pred);

    Ok(())
}

#[test]
fn delete_removes_secret_from_backend() -> Result<(), Box<dyn Error>> {
    let helper = env_probe();

    let mut store = local_secrets_cmd()?;
    store
        .env(BACKEND_ENV, "memory")
        .env(TEST_SECRET_ENV, "initial-token")
        .arg("store")
        .arg("CI_PAT");
    store
        .assert()
        .success()
        .stdout(predicate::str::contains("Stored secret for CI_PAT"));

    let mut delete = local_secrets_cmd()?;
    delete
        .env(BACKEND_ENV, "memory")
        .arg("delete")
        .arg("CI_PAT");
    delete
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted CI_PAT"));

    let mut run = local_secrets_cmd()?;
    run.env(BACKEND_ENV, "memory")
        .env_remove(TEST_SECRET_ENV)
        .args(["--env", "CI_PAT", "--"])
        .arg(&helper)
        .arg("CI_PAT");

    run.assert()
        .failure()
        .stderr(predicate::str::contains("Secret CI_PAT not found"));

    Ok(())
}
