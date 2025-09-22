use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};

pub trait SecretBackend {
    fn store(&mut self, key: &str, value: &SecretString) -> Result<()>;
    fn retrieve(&self, key: &str) -> Result<Option<SecretString>>;
    fn delete(&mut self, key: &str) -> Result<bool>; // returns true if existed
}

pub struct KeyringBackend {
    service: String,
}

impl KeyringBackend {
    pub fn new() -> Self {
        Self {
            service: "local-secrets".to_string(),
        }
    }
}

impl SecretBackend for KeyringBackend {
    fn store(&mut self, key: &str, value: &SecretString) -> Result<()> {
        // Defensive: Validate inputs before proceeding
        if key.trim().is_empty() {
            return Err(anyhow::anyhow!("Key cannot be empty"));
        }
        if value.expose_secret().is_empty() {
            return Err(anyhow::anyhow!("Cannot store empty secret"));
        }

        let entry =
            keyring::Entry::new(&self.service, key).context("Failed to create keyring entry")?;
        entry
            .set_password(value.expose_secret())
            .context("Failed to store secret in keyring")?;
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<SecretString>> {
        // Defensive: Validate input before proceeding
        if key.trim().is_empty() {
            return Err(anyhow::anyhow!("Key cannot be empty"));
        }

        let entry =
            keyring::Entry::new(&self.service, key).context("Failed to create keyring entry")?;
        match entry.get_password() {
            Ok(password) => Ok(Some(SecretString::new(password.into()))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err).context("Failed to retrieve secret from keyring")?,
        }
    }

    fn delete(&mut self, key: &str) -> Result<bool> {
        // Defensive: Validate input before proceeding
        if key.trim().is_empty() {
            return Err(anyhow::anyhow!("Key cannot be empty"));
        }

        let entry =
            keyring::Entry::new(&self.service, key).context("Failed to create keyring entry")?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(err) => Err(err).context("Failed to delete secret from keyring")?,
        }
    }
}
