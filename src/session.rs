use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub cwd: Option<String>,
    pub editor_buffers: Vec<String>,
    pub repl_history: Vec<String>,
}

impl Session {
    pub fn path() -> std::path::PathBuf {
        crate::config::config_dir().join("session.toml")
    }

    pub fn save(&self) {
        if let Ok(content) = toml::to_string_pretty(self) {
            let path = Self::path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, content);
        }
    }

    pub fn load() -> Option<Self> {
        let path = Self::path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }
}
