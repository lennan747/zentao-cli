use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 单个 profile 配置。不保存密码。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    pub server: String,
    pub account: String,
    #[serde(default)]
    pub timeout_seconds: u64,
}

/// 应用配置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn config_dir() -> PathBuf {
        // 测试和临时环境可通过 ZENTAO_CLI_HOME 覆盖配置目录。
        if let Ok(dir) = std::env::var("ZENTAO_CLI_HOME") {
            if !dir.is_empty() {
                return PathBuf::from(dir);
            }
        }
        directories::ProjectDirs::from("com", "example", "zentao-cli")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".zentao-cli"))
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }
}
