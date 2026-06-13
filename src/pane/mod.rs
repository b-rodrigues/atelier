pub mod diagram;
pub mod llm;
pub mod plot;
pub mod pty;
pub mod tabs;
pub mod transcript;
pub mod vars;

use ratatui::layout::Rect;
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneKind {
    Editor,
    Repl,
    Variables,
    Terminal,
    Diagram,
    Plot,
    Llm,
    Transcript,
}

pub trait Pane {
    fn kind(&self) -> PaneKind;
    fn name(&self) -> &str;
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool);
    fn write_input(&mut self, bytes: &[u8]);
    fn resize(&mut self, _cols: u16, _rows: u16) {}
    fn kill(&mut self) {}
    fn get_cursor_line(&self) -> Option<String> {
        None
    }

    fn get_status_line(&self) -> Option<String> {
        None
    }

    fn get_visible_lines(&self) -> Vec<String> {
        vec![]
    }

    fn push_context(&mut self, _ctx: &crate::context::AtelierContext) {}

    fn switch_tab(&mut self, _delta: i8) {}

    fn scroll(&mut self, _rows: i16) {}

    fn push_transcript_entry(&mut self, _sent: &str, _response: &str) {}

    fn save_artifact(&mut self) -> Option<String> {
        None
    }
}

pub struct ErrorPane {
    pub kind: PaneKind,
    pub name: String,
    pub error: String,
}

impl Pane for ErrorPane {
    fn kind(&self) -> PaneKind {
        self.kind
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn render(&mut self, f: &mut Frame, area: Rect, _focused: bool) {
        let text = ratatui::widgets::Paragraph::new(vec![
            ratatui::text::Line::from(ratatui::text::Span::styled(
                "Failed to spawn pane:",
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Red)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(self.error.as_str()),
        ])
        .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(text, area);
    }
    fn write_input(&mut self, _bytes: &[u8]) {}
}
