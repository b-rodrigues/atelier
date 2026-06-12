use crate::config::Config;
use crate::pane::{Pane, PaneKind};

pub enum InputMode {
    Normal,
    Navigation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    FileTree,
    BufferList,
    Help,
    QuitConfirm,
}

pub struct App {
    pub config: Config,
    pub panes: Vec<Box<dyn Pane>>,
    pub focus: usize,
    pub mode: InputMode,
    pub overlay: Option<Overlay>,
    pub should_quit: bool,
    pub filetree_base: String,
    pub filetree_scroll: usize,
    pub filetree_entries: Vec<(String, bool)>,
    pub filetree_selection: usize,
}

impl App {
    pub fn new(config: Config, repo_path: Option<String>) -> Self {
        Self {
            config,
            panes: Vec::new(),
            focus: 0,
            mode: InputMode::Normal,
            overlay: None,
            should_quit: false,
            filetree_base: repo_path.unwrap_or_else(|| ".".to_string()),
            filetree_scroll: 0,
            filetree_entries: Vec::new(),
            filetree_selection: 0,
        }
    }

    pub fn focus_pane(&mut self, kind: PaneKind) {
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.kind() == kind {
                self.focus = i;
                return;
            }
        }
    }

    pub fn focused_pane_mut(&mut self) -> Option<&mut Box<dyn Pane>> {
        self.panes.get_mut(self.focus)
    }

    pub fn focused_pane(&self) -> Option<&Box<dyn Pane>> {
        self.panes.get(self.focus)
    }

    pub fn open_in_editor(&mut self, path: &str) {
        let msg = format!("\x1b:e {}\r", path);
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.kind() == PaneKind::Editor {
                if let Some(editor) = self.panes.get_mut(i) {
                    let bytes = msg.as_bytes();
                    editor.write_input(bytes);
                }
                return;
            }
        }
    }

    pub fn save_all_and_quit(&mut self) {
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.kind() == PaneKind::Editor {
                if let Some(editor) = self.panes.get_mut(i) {
                    editor.write_input(b"\x1b:wqa\r");
                }
                break;
            }
        }
        self.should_quit = true;
    }

    pub fn kill_all(&mut self) {
        for pane in self.panes.iter_mut() {
            pane.kill();
        }
    }

    pub fn send_to_repl(&mut self) {
        let line = read_clipboard().unwrap_or_default();
        if line.is_empty() {
            return;
        }
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.kind() == PaneKind::Repl {
                if let Some(repl) = self.panes.get_mut(i) {
                    repl.write_input(line.as_bytes());
                    repl.write_input(b"\r");
                }
                return;
            }
        }
    }
}

fn read_clipboard() -> Option<String> {
    for cmd in &["wl-paste", "xclip -o -selection clipboard", "xsel -b"] {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let output = std::process::Command::new(parts[0])
            .args(&parts[1..])
            .output()
            .ok()?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}
