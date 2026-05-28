use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "translation-konjac";
const KEYRING_USER: &str = "api_key";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub gateway_url: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub target_language: String,
    pub delta_threshold: f32,
}

// Persisted to disk — no sensitive fields.
#[derive(Serialize, Deserialize)]
struct StoredConfig {
    #[serde(default)]
    gateway_url: String,
    #[serde(default = "default_provider")]
    provider: String,
    #[serde(default = "default_model")]
    model: String,
    #[serde(default = "default_language")]
    target_language: String,
    #[serde(default = "default_threshold")]
    delta_threshold: f32,
}

fn default_provider() -> String { "openai".to_string() }
fn default_model() -> String { "gpt-5.4-mini".to_string() }
fn default_language() -> String { "English".to_string() }
fn default_threshold() -> f32 { 0.05 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway_url: String::new(),
            provider: default_provider(),
            model: default_model(),
            api_key: String::new(),
            target_language: default_language(),
            delta_threshold: default_threshold(),
        }
    }
}

impl AppConfig {
    pub fn load_or_default(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("config.toml");
        let stored: StoredConfig = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_else(|| StoredConfig {
                gateway_url: String::new(),
                provider: default_provider(),
                model: default_model(),
                target_language: default_language(),
                delta_threshold: default_threshold(),
            });

        let api_key = Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .ok()
            .and_then(|e| e.get_password().ok())
            .unwrap_or_default();

        Self {
            gateway_url: stored.gateway_url,
            provider: stored.provider,
            model: stored.model,
            api_key,
            target_language: stored.target_language,
            delta_threshold: stored.delta_threshold,
        }
    }

    pub fn save(&self, app_data_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&app_data_dir)?;
        let stored = StoredConfig {
            gateway_url: self.gateway_url.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            target_language: self.target_language.clone(),
            delta_threshold: self.delta_threshold,
        };
        std::fs::write(app_data_dir.join("config.toml"), toml::to_string(&stored)?)?;

        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        if self.api_key.is_empty() {
            let _ = entry.delete_credential();
        } else {
            entry.set_password(&self.api_key)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_default_returns_defaults_when_no_file() {
        let dir = PathBuf::from("/tmp/konjac-test-nonexistent-12345");
        let config = AppConfig::load_or_default(dir);
        assert_eq!(config.target_language, "English");
        assert!((config.delta_threshold - 0.05).abs() < f32::EPSILON);
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-5.4-mini");
    }

    #[test]
    fn save_and_reload_round_trips() {
        let dir = PathBuf::from("/tmp/konjac-test-config-roundtrip");
        let mut cfg = AppConfig::default();
        cfg.target_language = "Vietnamese".to_string();
        cfg.delta_threshold = 0.1;
        cfg.save(dir.clone()).unwrap();

        let loaded = AppConfig::load_or_default(dir.clone());
        assert_eq!(loaded.target_language, "Vietnamese");
        assert!((loaded.delta_threshold - 0.1).abs() < f32::EPSILON);

        std::fs::remove_dir_all(dir).ok();
    }
}
