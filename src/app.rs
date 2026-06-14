use crate::config::Config;
use crate::pane::{Pane, PaneKind};
use crate::session::Session;
use std::time::Instant;

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
    ProjectSwitcher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaximizedState {
    None,
    Full,
    Vertical,
    Horizontal,
}

pub struct App {
    pub config: Config,
    pub panes: Vec<Box<dyn Pane>>,
    pub focus: usize,
    pub mode: InputMode,
    pub overlay: Option<Overlay>,
    pub maximized: MaximizedState,
    pub buffer_input: String,
    pub should_quit: bool,
    pub should_relaunch: Option<String>,
    pub recent_projects: Vec<String>,
    pub project_switcher_selection: usize,
    pub filetree_base: String,
    pub filetree_current: String,
    pub filetree_scroll: usize,
    pub filetree_entries: Vec<(String, bool)>,
    pub filetree_selection: usize,
    pub last_llm_context_refresh: Instant,
}

impl App {
    pub fn new(config: Config, repo_path: Option<String>) -> Self {
        let base = repo_path.unwrap_or_else(|| ".".to_string());
        let initial_maximized = match config.layout.initial_maximized.as_str() {
            "full" => MaximizedState::Full,
            "vertical" => MaximizedState::Vertical,
            "horizontal" => MaximizedState::Horizontal,
            _ => MaximizedState::None,
        };
        Self {
            config,
            panes: Vec::new(),
            focus: 0,
            mode: InputMode::Normal,
            overlay: None,
            maximized: initial_maximized,
            buffer_input: String::new(),
            should_quit: false,
            should_relaunch: None,
            recent_projects: Vec::new(),
            project_switcher_selection: 0,
            filetree_base: base.clone(),
            filetree_current: base,
            filetree_scroll: 0,
            filetree_entries: Vec::new(),
            filetree_selection: 0,
            last_llm_context_refresh: Instant::now(),
        }
    }


    pub fn focused_pane_mut(&mut self) -> Option<&mut Box<dyn Pane>> {
        self.panes.get_mut(self.focus)
    }

    pub fn focused_pane(&self) -> Option<&Box<dyn Pane>> {
        self.panes.get(self.focus)
    }

    pub fn open_in_editor(&mut self, path: &str) {
        let is_nano = self.config.editor.command.contains("nano");
        let msg = if is_nano {
            // nano: Alt-F to toggle new buffer, Ctrl-R to insert/open file, path, Enter
            format!("\x1bF\x12{}\r", path)
        } else {
            // vim/nvim: Esc Esc :e path Enter
            format!("\x1b\x1b:e {}\r", path)
        };
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

    pub fn save_session(&self) {
        let cwd = std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string());
        let editor_buffers = self.read_editor_buffers();
        let repl_history = self
            .panes
            .iter()
            .find(|p| p.kind() == PaneKind::Repl)
            .map(|p| p.get_visible_lines())
            .unwrap_or_default();
        let session = Session {
            cwd,
            editor_buffers,
            repl_history,
        };
        session.save();
    }

    pub fn restore_session(&mut self) {
        if let Some(session) = Session::load() {
            if let Some(ref cwd) = session.cwd {
                let _ = std::env::set_current_dir(cwd);
            }
            if !session.editor_buffers.is_empty() {
                let buffers = session.editor_buffers.clone();
                // Defer buffer opening to after editor is ready — handled in main
                for (i, pane) in self.panes.iter().enumerate() {
                    if pane.kind() == PaneKind::Editor {
                        let cmd = format!("\x1b\x1b: silent! e {}\r", buffers.join(" "));
                        if let Some(editor) = self.panes.get_mut(i) {
                            editor.write_input(cmd.as_bytes());
                        }
                        break;
                    }
                }
            }
        }
    }

    pub fn save_all_and_quit(&mut self) {
        let is_nano = self.config.editor.command.contains("nano");
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.kind() == PaneKind::Editor {
                if let Some(editor) = self.panes.get_mut(i) {
                    if is_nano {
                        // nano: Ctrl-O (write-out), Enter, Ctrl-X (exit)
                        editor.write_input(b"\x0f\r\x18");
                    } else {
                        // vim: Esc Esc :wa Enter
                        editor.write_input(b"\x1b\x1b:wa\r");
                    }
                }
                break;
            }
        }
        self.save_session();
        self.should_quit = true;
    }

    pub fn refresh_llm_context(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_llm_context_refresh).as_millis() < 500 {
            return;
        }
        self.last_llm_context_refresh = now;
        let ctx = crate::context::AtelierContext::gather(self);
        for pane in self.panes.iter_mut() {
            pane.push_context(&ctx);
        }
    }

    pub fn kill_all(&mut self) {
        for pane in self.panes.iter_mut() {
            pane.kill();
        }
    }

fn clean_editor_line(line: &str) -> String {
    let trimmed = line.trim_end();
    
    // 1. If there is a vertical separator in the first 12 characters, strip up to it.
    if let Some(bar_idx) = trimmed.chars().take(12).position(|c| c == '│' || c == '|' || c == '┃' || c == '▕') {
        let suffix: String = trimmed.chars().skip(bar_idx + 1).collect();
        return suffix.trim_start().to_string();
    }
    
    // 2. Otherwise, check if the line starts with optional spaces, followed by digits, followed by a space.
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() && chars[i] == ' ' {
        i += 1;
    }
    let digit_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i > digit_start && i < chars.len() && chars[i] == ' ' {
        let mut j = i;
        while j < chars.len() && chars[j] == ' ' {
            j += 1;
        }
        if j < chars.len() {
            let next_c = chars[j];
            if next_c != '+' && next_c != '-' && next_c != '*' && next_c != '/' && next_c != '=' && next_c != '%' && next_c != '^' {
                return chars[j..].iter().collect();
            }
        }
    }
    
    trimmed.to_string()
}

    pub fn send_to_repl(&mut self, force_clipboard: bool) {
        let line_to_send = if let Some(pane) = self.panes.get(self.focus) {
            if pane.kind() == PaneKind::Transcript {
                pane.get_selected_entry().map(|s| s.to_string())
            } else if !force_clipboard {
                if let Some(line) = pane.get_cursor_line() {
                    let cleaned = Self::clean_editor_line(&line);
                    if !cleaned.is_empty() {
                        Some(cleaned)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let line = match line_to_send {
            Some(l) => l,
            None => read_clipboard().unwrap_or_default(),
        };

        if line.is_empty() {
            return;
        }
        let mut response = String::new();
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.kind() == PaneKind::Repl {
                if let Some(repl) = self.panes.get_mut(i) {
                    repl.write_input(line.as_bytes());
                    repl.write_input(b"\r");
                    response = repl.get_visible_lines().join("\n");
                }
                break;
            }
        }
        let from_transcript = self.panes.get(self.focus)
            .map(|p| p.kind() == PaneKind::Transcript)
            .unwrap_or(false);
        if !from_transcript {
            for pane in self.panes.iter_mut() {
                if pane.kind() == PaneKind::Transcript {
                    pane.push_transcript_entry(&line, &response);
                    break;
                }
            }
        }
    }

    pub fn send_to_editor(&mut self) {
        let text = if let Some(pane) = self.panes.get(self.focus) {
            if pane.kind() == PaneKind::Transcript {
                pane.get_selected_entry().map(|s| s.to_string())
            } else {
                read_clipboard()
            }
        } else {
            None
        };

        let text = match text {
            Some(t) => t,
            None => return,
        };

        let paste_path = std::env::temp_dir().join(format!(
            "atelier-editor-paste-{}.txt",
            std::process::id()
        ));
        let _ = std::fs::write(&paste_path, &text);
        for pane in self.panes.iter_mut() {
            if pane.kind() == PaneKind::Editor {
                pane.write_input(b"\x1b\x1b");
                pane.write_input(
                    format!(":r {}\r", paste_path.to_string_lossy()).as_bytes(),
                );
                break;
            }
        }
    }

    pub fn read_editor_buffers(&self) -> Vec<String> {
        let buffers_path = std::path::Path::new("/tmp/atelier-buffers.txt");
        let content = std::fs::read_to_string(buffers_path).unwrap_or_default();
        content
            .lines()
            .filter(|l| {
                let trimmed = l.trim();
                !trimmed.is_empty() && !trimmed.contains("Press ENTER")
            })
            .map(|l| l.trim().to_string())
            .collect()
    }

    pub fn pane_index_at_physical_pos(&self, pos: usize) -> Option<usize> {
        if pos >= self.config.layout.positions.len() {
            return None;
        }
        let kind_str = &self.config.layout.positions[pos];
        let kind = match kind_str.as_str() {
            "editor" => PaneKind::Editor,
            "repl" => PaneKind::Repl,
            "variables" => PaneKind::Variables,
            "terminal" => PaneKind::Terminal,
            "diagram" => PaneKind::Diagram,
            "plot" => PaneKind::Plot,
            _ => return None,
        };
        self.panes.iter().position(|p| p.kind() == kind)
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
