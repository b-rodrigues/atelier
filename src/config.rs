use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    dirs_or_default()
}

fn dirs_or_default() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config/atelier")
    } else {
        PathBuf::from(".config/atelier")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub editor: EditorConfig,
    pub repl: ReplConfig,
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub layout: LayoutConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    #[serde(default = "default_positions")]
    pub positions: Vec<String>,
    #[serde(default = "default_initial_maximized")]
    pub initial_maximized: String,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            positions: default_positions(),
            initial_maximized: default_initial_maximized(),
        }
    }
}

fn default_positions() -> Vec<String> {
    vec![
        "editor".to_string(),
        "repl".to_string(),
        "variables".to_string(),
        "terminal".to_string(),
    ]
}

fn default_initial_maximized() -> String {
    "none".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    pub leader: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: EditorConfig {
                command: "nvim".into(),
                args: vec!["--cmd".into(), "set shortmess+=I".into()],
            },
            repl: ReplConfig {
                command: "t".into(),
                args: vec!["repl".into()],
            },
            keybindings: KeybindingsConfig {
                leader: "ctrl-space".into(),
            },
            layout: LayoutConfig::default(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        config_dir().join("config.toml")
    }

    pub fn load() -> Result<Option<Self>> {
        let path = Self::path();
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config at {}", path.display()))?;
        Ok(Some(config))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory at {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }
}
