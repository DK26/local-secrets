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
fn store_then_run_injects_secret_from_keyring_backend() -> Result<(), Box<dyn Error>> {
    let helper = env_probe();

    // Use unique variable name to avoid conflicts with keyring
    let test_var = format!(
        "CLI_TEST_GITHUB_PAT_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let mut store = local_secrets_cmd()?;
    store
        .env_remove(BACKEND_ENV) // Use default keyring backend
        .env(TEST_SECRET_ENV, "super-secret-token")
        .arg("store")
        .arg(&test_var);
    store
        .assert()
        .success()
        .stdout(predicate::str::contains(&format!(
            "Stored secret for {}",
            test_var
        )))
        .stderr(predicate::str::contains(&format!(
            "Enter secret for {}",
            test_var
        )));

    let mut run = local_secrets_cmd()?;
    run.env_remove(BACKEND_ENV) // Use default keyring backend
        .env_remove(TEST_SECRET_ENV)
        .args(["--env", &test_var, "--"])
        .arg(&helper)
        .arg(&test_var);

    let stderr_pred = predicate::str::contains(&format!("Injecting env vars: [\"{}\"]", test_var))
        .and(predicate::str::contains("Enter secret").not());

    run.assert()
        .success()
        .stdout(predicate::str::contains("super-secret-token"))
        .stderr(stderr_pred);

    // Clean up - delete the test secret from keyring
    let mut cleanup = local_secrets_cmd()?;
    cleanup.env_remove(BACKEND_ENV).arg("delete").arg(&test_var);
    let _ = cleanup.output(); // Best effort cleanup

    Ok(())
}

#[test]
fn no_save_missing_requires_secret_each_time() -> Result<(), Box<dyn Error>> {
    let helper = env_probe();

    // Use unique variable name
    let test_var = format!(
        "CLI_TEST_API_KEY_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let mut first = local_secrets_cmd()?;
    first
        .env_remove(BACKEND_ENV) // Use default keyring backend
        .env(TEST_SECRET_ENV, "transient-1")
        .args(["--env", &test_var, "--no-save-missing", "--"])
        .arg(&helper)
        .arg(&test_var);

    first
        .assert()
        .success()
        .stdout(predicate::str::contains("transient-1"))
        .stderr(predicate::str::contains(&format!(
            "Enter secret for missing {}",
            test_var
        )));

    let mut second = local_secrets_cmd()?;
    second
        .env_remove(BACKEND_ENV) // Use default keyring backend
        .env(TEST_SECRET_ENV, "transient-2")
        .args(["--env", &test_var, "--no-save-missing", "--"])
        .arg(&helper)
        .arg(&test_var);

    let stderr_pred = predicate::str::contains(&format!("Enter secret for missing {}", test_var))
        .and(predicate::str::contains(&format!("Stored secret for {}", test_var)).not());

    second
        .assert()
        .success()
        .stdout(predicate::str::contains("transient-2"))
        .stderr(stderr_pred);

    Ok(())
}

#[test]
fn delete_removes_secret_from_keyring_backend() -> Result<(), Box<dyn Error>> {
    let helper = env_probe();

    // Use unique variable name
    let test_var = format!(
        "CLI_TEST_CI_PAT_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let mut store = local_secrets_cmd()?;
    store
        .env_remove(BACKEND_ENV) // Use default keyring backend
        .env(TEST_SECRET_ENV, "initial-token")
        .arg("store")
        .arg(&test_var);
    store
        .assert()
        .success()
        .stdout(predicate::str::contains(&format!(
            "Stored secret for {}",
            test_var
        )));

    let mut delete = local_secrets_cmd()?;
    delete
        .env_remove(BACKEND_ENV) // Use default keyring backend
        .arg("delete")
        .arg(&test_var);
    delete
        .assert()
        .success()
        .stdout(predicate::str::contains(&format!("Deleted {}", test_var)));

    let mut run = local_secrets_cmd()?;
    run.env_remove(BACKEND_ENV) // Use default keyring backend
        .env_remove(TEST_SECRET_ENV)
        .args(["--env", &test_var, "--"])
        .arg(&helper)
        .arg(&test_var);

    run.assert()
        .failure()
        .stderr(predicate::str::contains(&format!(
            "Secret {} not found",
            test_var
        )));

    Ok(())
}
