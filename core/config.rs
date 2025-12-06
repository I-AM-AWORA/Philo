use anyhow::{Result, Context};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct SourceConfig {
    pub id: String,                // global kaynak id (örn "philo")
    pub name: String,              // görünen ad
    pub namespace: String,         // paket id namespace (örn "philo")
    pub default_provider: String,  // varsayılan sağlayıcı
    pub providers: Vec<String>,    // aktif sağlayıcılar
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub repo_path: String,
    pub prefer_formats: Vec<String>,
    pub max_size_mb: u64,
    pub providers: Vec<String>,    // geri uyumluluk için
    pub user_agent: Option<String>,
    pub source: SourceConfig,      // global source
}

pub fn load_config() -> Result<Config> {
    let default = Config {
        repo_path: "./philo_repo".into(),
        prefer_formats: vec!["pdf".into(), "epub".into()],
        max_size_mb: 200,
        providers: vec!["openlibrary".into(), "googlebooks".into(), "archive".into()],
        user_agent: Some("Philo/0.1".into()),
        source: SourceConfig {
            id: "philo".into(),
            name: "Philo Registry".into(),
            namespace: "philo".into(),
            default_provider: "openlibrary".into(),
            providers: vec!["openlibrary".into(), "googlebooks".into(), "archive".into()],
        },
    };

    let path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("philo/config.yaml");

    if !path.exists() {
        fs::create_dir_all(&default.repo_path).ok();
        return Ok(default);
    }

    let s = fs::read_to_string(&path).with_context(|| format!("Config okunamadı: {}", path.display()))?;
    let mut cfg: Config = serde_yaml::from_str(&s)?;
    if cfg.providers.is_empty() {
        cfg.providers = cfg.source.providers.clone();
    }
    fs::create_dir_all(&cfg.repo_path).ok();
    Ok(cfg)
}
