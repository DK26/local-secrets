use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroize;

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

pub struct MemoryBackend {
    file_path: PathBuf,
}

impl MemoryBackend {
    pub fn new() -> Result<Self> {
        // Use a fixed name for the memory backend so it persists across CLI invocations in tests
        // In a real test environment, each test should run in isolation
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("local-secrets-memory-backend.json");
        Ok(Self {
            file_path: temp_dir,
        })
    }

    fn load_data(&self) -> Result<HashMap<String, String>> {
        if !self.file_path.exists() {
            return Ok(HashMap::new());
        }
        let content =
            fs::read_to_string(&self.file_path).context("Failed to read memory backend file")?;
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }
        let data: HashMap<String, String> =
            serde_json::from_str(&content).context("Failed to parse memory backend file")?;
        Ok(data)
    }

    fn save_data(&self, data: &HashMap<String, String>) -> Result<()> {
        let content =
            serde_json::to_string(data).context("Failed to serialize memory backend data")?;
        fs::write(&self.file_path, content).context("Failed to write memory backend file")?;
        Ok(())
    }
}

impl SecretBackend for MemoryBackend {
    fn store(&mut self, key: &str, value: &SecretString) -> Result<()> {
        // Defensive: Validate inputs before proceeding
        if key.trim().is_empty() {
            return Err(anyhow::anyhow!("Key cannot be empty"));
        }
        if value.expose_secret().is_empty() {
            return Err(anyhow::anyhow!("Cannot store empty secret"));
        }

        let mut data = self.load_data()?;
        let mut secret_value = value.expose_secret().to_string();
        data.insert(key.to_string(), secret_value.clone());
        secret_value.zeroize(); // Zero out the temporary secret copy
        self.save_data(&data)?;
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<SecretString>> {
        // Defensive: Validate input before proceeding
        if key.trim().is_empty() {
            return Err(anyhow::anyhow!("Key cannot be empty"));
        }

        let data = self.load_data()?;
        match data.get(key) {
            Some(value) => {
                let mut value_copy = value.clone();
                let secret = SecretString::new(value_copy.clone().into());
                value_copy.zeroize(); // Zero out the temporary copy
                Ok(Some(secret))
            }
            None => Ok(None),
        }
    }

    fn delete(&mut self, key: &str) -> Result<bool> {
        // Defensive: Validate input before proceeding
        if key.trim().is_empty() {
            return Err(anyhow::anyhow!("Key cannot be empty"));
        }

        let mut data = self.load_data()?;
        let existed = data.remove(key).is_some();
        self.save_data(&data)?;
        Ok(existed)
    }
}
