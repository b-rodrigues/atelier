use crate::config::LlmConfig;
use crate::context::AtelierContext;
use crate::pane::{Pane, PaneKind};
use crate::pane::pty::PtyPane;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

enum LlmState {
    Uninitialised,
    AwaitingPath { default_path: String, input: String },
    Running { last_context_hash: u64 },
    Dead(String),
}

pub struct LlmPane {
    inner: Option<PtyPane>,
    state: LlmState,
    config: LlmConfig,
    pending_context: Option<String>,
    default_path: String,
}

impl LlmPane {
    pub fn new(config: LlmConfig, default_path: String) -> Self {
        Self {
            inner: None,
            state: LlmState::Uninitialised,
            config,
            pending_context: None,
            default_path,
        }
    }

    fn spawn_opencode(&mut self, path: &str) {
        let args = self.build_args();
        match PtyPane::spawn(
            PaneKind::Llm,
            "LLM".into(),
            &self.config.command,
            &args,
            Some(path),
        ) {
            Ok(pty) => {
                self.inner = Some(pty);
                if let Some(ctx) = &self.pending_context {
                    if self.config.context_mode == "stdin" {
                        self.inner.as_mut().unwrap().write_input(ctx.as_bytes());
                        self.inner.as_mut().unwrap().write_input(b"\n");
                    }
                }
                self.state = LlmState::Running {
                    last_context_hash: 0,
                };
            }
            Err(e) => {
                eprintln!("Failed to spawn LLM pane: {}", e);
                self.state = LlmState::Dead(format!("Failed to spawn: {}", e));
            }
        }
    }

    fn build_args(&self) -> Vec<String> {
        let mut args = self.config.args.clone();
        match self.config.context_mode.as_str() {
            "flag" if !self.config.context_flag.is_empty() => {
                args.push(self.config.context_flag.clone());
                args.push(self.config.context_path.clone());
            }
            _ => {}
        }
        args
    }

    fn push_context_inner(&mut self, ctx: &AtelierContext) {
        let should_update = match self.state {
            LlmState::Running { last_context_hash } => last_context_hash != ctx.hash,
            _ => false,
        };
        if !should_update {
            return;
        }

        self.state = LlmState::Running {
            last_context_hash: ctx.hash,
        };

        let md = ctx.to_markdown();
        let _ = std::fs::write(&self.config.context_path, &md);
        self.pending_context = Some(md);

        if self.config.context_mode == "stdin" {
            if let Some(pty) = &mut self.inner {
                if let Some(ctx) = &self.pending_context {
                    pty.write_input(ctx.as_bytes());
                    pty.write_input(b"\n");
                }
            }
        }
    }
}

impl Pane for LlmPane {
    fn kind(&self) -> PaneKind {
        PaneKind::Llm
    }

    fn name(&self) -> &str {
        "LLM"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        if matches!(self.state, LlmState::Uninitialised) {
            self.state = LlmState::AwaitingPath {
                default_path: self.default_path.clone(),
                input: String::new(),
            };
        }

        match &mut self.state {
            LlmState::AwaitingPath {
                default_path,
                input,
            } => {
                let path = if input.is_empty() {
                    default_path.as_str()
                } else {
                    input.as_str()
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title("LLM");

                let inner = block.inner(area);
                f.render_widget(block, area);

                let lines = vec![
                    Line::from(Span::styled(
                        "LLM not started. Enter the project path to spawn opencode:",
                        Style::default().fg(Color::Cyan),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Path: "),
                        Span::styled(
                            path,
                            Style::default().fg(Color::Green),
                        ),
                        Span::styled(
                            "▊",
                            Style::default().fg(Color::Green).add_modifier(Modifier::SLOW_BLINK),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press Enter to confirm, Esc to cancel",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
                f.render_widget(paragraph, inner);
            }
            LlmState::Running { .. } => {
                if let Some(pty) = &mut self.inner {
                    pty.render(f, area, focused);
                }
            }
            LlmState::Dead(msg) => {
                let lines = vec![
                    Line::from(Span::styled(
                        "LLM pane failed to start.",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(msg.as_str(), Style::default().fg(Color::Red))),
                ];
                let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
                f.render_widget(paragraph, area);
            }
            _ => {}
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        match &mut self.state {
            LlmState::AwaitingPath {
                default_path,
                input,
            } => match bytes {
                b"\r" | b"\n" => {
                    let path = if input.is_empty() {
                        default_path.clone()
                    } else {
                        input.clone()
                    };
                    self.spawn_opencode(&path);
                }
                b"\x7f" | b"\x08" => {
                    input.pop();
                }
                b"\x1b" => {
                    input.clear();
                }
                _ => {
                    if let Ok(s) = std::str::from_utf8(bytes) {
                        if s.chars().all(|c| c.is_ascii_graphic() || c == ' ' || c == '/' || c == '.' || c == '-' || c == '_' || c == '~') {
                            input.push_str(s);
                        }
                    }
                }
            },
            LlmState::Running { .. } => {
                if let Some(pty) = &mut self.inner {
                    pty.write_input(bytes);
                }
            }
            _ => {}
        }
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        if let Some(pty) = &mut self.inner {
            pty.resize(cols, rows);
        }
    }

    fn kill(&mut self) {
        if let Some(pty) = &mut self.inner {
            pty.kill();
        }
    }

    fn get_cursor_line(&self) -> Option<String> {
        self.inner.as_ref().and_then(|p| p.get_cursor_line())
    }

    fn get_status_line(&self) -> Option<String> {
        self.inner.as_ref().and_then(|p| p.get_status_line())
    }

    fn get_visible_lines(&self) -> Vec<String> {
        self.inner
            .as_ref()
            .map(|p| p.get_visible_lines())
            .unwrap_or_default()
    }

    fn scroll(&mut self, rows: i16) {
        if let Some(pty) = &mut self.inner {
            pty.scroll(rows);
        }
    }

    fn push_context(&mut self, ctx: &AtelierContext) {
        self.push_context_inner(ctx);
    }
}
