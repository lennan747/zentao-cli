use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 持久化会话。只保存 server origin 和 session cookie，不保存密码。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub server: String,
    pub cookie: String,
}

impl StoredSession {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let session = serde_json::from_str(&content)?;
        Ok(Some(session))
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(path)?.permissions();
            perm.set_mode(0o600);
            std::fs::set_permissions(path, perm)?;
        }
        Ok(())
    }

    pub fn session_path(profile: &str) -> PathBuf {
        let dir = super::config::Config::config_dir();
        dir.join(format!("session-{}.json", profile))
    }
}
