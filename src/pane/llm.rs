use crate::config::LlmConfig;
use crate::context::AtelierContext;
use crate::pane::{Pane, PaneKind};
use crate::pane::pty::PtyPane;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

enum LlmState {
    Uninitialised,
    Running { last_context_hash: u64 },
    Dead,
}

pub struct LlmPane {
    inner: Option<PtyPane>,
    state: LlmState,
    config: LlmConfig,
    pending_context: Option<String>,
}

impl LlmPane {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            inner: None,
            state: LlmState::Uninitialised,
            config,
            pending_context: None,
        }
    }

    fn push_context_inner(&mut self, ctx: &AtelierContext) {
        if let LlmState::Running {
            last_context_hash,
        } = self.state
        {
            if ctx.hash == last_context_hash {
                return;
            }
        }
        let md = ctx.to_markdown();
        let _ = std::fs::write(&self.config.context_path, &md);
        self.pending_context = Some(md);
        self.spawn_or_refresh(ctx.hash);
    }

    fn spawn_or_refresh(&mut self, hash: u64) {
        self.kill();
        let args = self.build_args();
        match PtyPane::spawn(
            PaneKind::Llm,
            "LLM".into(),
            &self.config.command,
            &args,
            None,
        ) {
            Ok(mut pty) => {
                if self.config.context_mode == "stdin" {
                    if let Some(ctx) = &self.pending_context {
                        pty.write_input(ctx.as_bytes());
                        pty.write_input(b"\n");
                    }
                }
                self.inner = Some(pty);
                self.state = LlmState::Running {
                    last_context_hash: hash,
                };
            }
            Err(e) => {
                eprintln!("Failed to spawn LLM pane: {}", e);
                self.state = LlmState::Dead;
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
}

impl Pane for LlmPane {
    fn kind(&self) -> PaneKind {
        PaneKind::Llm
    }

    fn name(&self) -> &str {
        "LLM"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        match &mut self.inner {
            Some(pty) => pty.render(f, area, focused),
            None => {
                let status = match self.state {
                    LlmState::Uninitialised => "Press l in Nav mode to start LLM with context.",
                    LlmState::Dead => "LLM process failed to start.",
                    _ => "Initialising...",
                };
                let text = Paragraph::new(vec![
                    Line::from(Span::styled(
                        "LLM pane not started.",
                        ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        status,
                        ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
                    )),
                ])
                .wrap(Wrap { trim: true });
                f.render_widget(text, area);
            }
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        if let Some(pty) = &mut self.inner {
            pty.write_input(bytes);
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

    fn push_context(&mut self, ctx: &AtelierContext) {
        self.push_context_inner(ctx);
    }
}
