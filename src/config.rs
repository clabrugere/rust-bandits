use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub log_level: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct AccountantConfig {}

#[derive(Debug, Deserialize, Clone)]
pub struct PolicyCacheConfig {
    pub path: PathBuf,
    pub persist_every: u64,
}

#[derive(Debug, Deserialize)]
pub struct SupervisorConfig {
    pub ping_every: u64,
}

#[derive(Debug, Deserialize)]
pub struct BanditConfig {
    pub cache_every: u64,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub accountant: AccountantConfig,
    pub cache: PolicyCacheConfig,
    pub supervisor: SupervisorConfig,
    pub bandit: BanditConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let builder = Config::builder()
            .add_source(File::with_name("config"))
            .add_source(Environment::with_prefix("APP"))
            .build()?;

        builder.try_deserialize()
    }
}
