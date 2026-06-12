pub mod pty;
pub mod vars;

use ratatui::layout::Rect;
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneKind {
    Editor,
    Repl,
    Variables,
    Terminal,
}

pub trait Pane {
    fn kind(&self) -> PaneKind;
    fn name(&self) -> &str;
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn write_input(&mut self, bytes: &[u8]);
    fn resize(&mut self, _cols: u16, _rows: u16) {}
    fn kill(&mut self) {}
}
