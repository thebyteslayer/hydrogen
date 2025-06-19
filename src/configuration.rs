// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;
use crate::cluster;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("JSON serialization error: {0}")]
    JsonSerialize(#[from] serde_json::Error),
}

type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HydrogenConfig {
    #[serde(rename = "bind-ip")]
    pub bind_ip: String,
    #[serde(rename = "bind-port")]
    pub bind_port: u16,
    pub cluster_enabled: bool,
    pub whisper_timeout: u32,
}

impl Default for HydrogenConfig {
    fn default() -> Self {
        Self {
            bind_ip: "0.0.0.0".to_string(),
            bind_port: 1825,
            cluster_enabled: false,
            whisper_timeout: 1,
        }
    }
}

impl HydrogenConfig {
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_ip, self.bind_port)
    }

    pub fn load_or_create() -> ConfigResult<Self> {
        let config_path = "hydrogen.toml";
        
        let config = if Path::new(config_path).exists() {
            Self::load_and_heal(config_path)?
        } else {
            let default_config = Self::default();
            default_config.save_to_file(config_path)?;
            default_config
        };
        
        if config.cluster_enabled {
            cluster::generate_cluster_file(&config)?;
        }
        
        Ok(config)
    }

    fn load_and_heal(path: &str) -> ConfigResult<Self> {
        let content = fs::read_to_string(path)?;
        
        match toml::from_str::<HydrogenConfig>(&content) {
            Ok(config) => {
                let healed_config = Self::heal_config(config);
                healed_config.save_to_file(path)?;
                Ok(healed_config)
            }
            Err(_) => {
                let partial_config = Self::parse_partial_config(&content)?;
                let healed_config = Self::heal_config(partial_config);
                healed_config.save_to_file(path)?;
                Ok(healed_config)
            }
        }
    }

    fn parse_partial_config(content: &str) -> ConfigResult<Self> {
        let toml_value: toml::Value = toml::from_str(content)?;
        
        let mut config = Self::default();
        
        if let toml::Value::Table(table) = toml_value {
            if let Some(toml::Value::String(ip)) = table.get("bind-ip") {
                config.bind_ip = ip.clone();
            }
            if let Some(toml::Value::Integer(port)) = table.get("bind-port") {
                config.bind_port = *port as u16;
            }
            if let Some(toml::Value::Boolean(enabled)) = table.get("cluster_enabled") {
                config.cluster_enabled = *enabled;
            }
            if let Some(toml::Value::Integer(timeout)) = table.get("whisper_timeout") {
                config.whisper_timeout = *timeout as u32;
            }
        }
        
        Ok(config)
    }

    fn heal_config(config: HydrogenConfig) -> Self {
        config
    }

    fn save_to_file(&self, path: &str) -> ConfigResult<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
} 