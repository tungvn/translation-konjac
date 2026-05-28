use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub gateway_url: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub target_language: String,
    pub delta_threshold: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway_url: String::new(),
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: String::new(),
            target_language: "English".to_string(),
            delta_threshold: 0.05,
        }
    }
}

impl AppConfig {
    pub fn load_or_default(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("config.toml");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, app_data_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&app_data_dir)?;
        let path = app_data_dir.join("config.toml");
        std::fs::write(path, toml::to_string(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_or_default_returns_defaults_when_no_file() {
        let dir = PathBuf::from("/tmp/konjac-test-nonexistent-12345");
        let config = AppConfig::load_or_default(dir);
        assert_eq!(config.target_language, "English");
        assert!((config.delta_threshold - 0.05).abs() < f32::EPSILON);
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-5.4-nano");
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
