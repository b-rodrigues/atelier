use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
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
    #[serde(default)]
    pub diagram: DiagramConfig,
    #[serde(default)]
    pub plots: PlotConfig,
    #[serde(default)]
    pub llm: LlmConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiagramConfig {
    pub renderer: String,
    pub command: String,
    pub args: Vec<String>,
    pub watch_file: String,
}

impl Default for DiagramConfig {
    fn default() -> Self {
        Self {
            renderer: "dot".into(),
            command: "dot".into(),
            args: vec!["-Tpng".into()],
            watch_file: "/tmp/atelier/pipeline.dot".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlotConfig {
    pub watch_dir: String,
}

impl Default for PlotConfig {
    fn default() -> Self {
        Self {
            watch_dir: "/tmp/atelier-plots".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    pub command: String,
    pub args: Vec<String>,
    pub context_path: String,
    pub context_mode: String,
    pub context_flag: String,
    pub auto_context: bool,
}

fn default_llm_command() -> String {
    "opencode".into()
}

fn default_context_path() -> String {
    "/tmp/atelier-llm-context.md".into()
}

fn default_context_mode() -> String {
    "file".into()
}

fn default_auto_context() -> bool {
    true
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            command: default_llm_command(),
            args: vec![],
            context_path: default_context_path(),
            context_mode: default_context_mode(),
            context_flag: String::new(),
            auto_context: default_auto_context(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProjects {
    pub paths: Vec<String>,
}

impl RecentProjects {
    pub fn path() -> PathBuf {
        config_dir().join("recent.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            return Self { paths: Vec::new() };
        }
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| toml::from_str(&c).ok())
            .unwrap_or(Self { paths: Vec::new() })
    }

    pub fn save(&self) {
        if let Ok(content) = toml::to_string_pretty(self) {
            if let Some(parent) = Self::path().parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(Self::path(), content);
        }
    }

    pub fn add_project(path: &str) {
        let mut recent = Self::load();
        recent.paths.retain(|p| p != path);
        recent.paths.insert(0, path.to_string());
        if recent.paths.len() > 10 {
            recent.paths.truncate(10);
        }
        recent.save();
    }
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
            diagram: DiagramConfig::default(),
            plots: PlotConfig::default(),
            llm: LlmConfig::default(),
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
