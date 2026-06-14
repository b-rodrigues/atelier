use crate::context::AtelierContext;
use crate::pane::{Pane, PaneKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct TabContainer {
    children: Vec<Box<dyn Pane>>,
    active: usize,
    name: String,
    kind: PaneKind,
}

impl TabContainer {
    pub fn new(children: Vec<Box<dyn Pane>>, kind: PaneKind) -> Self {
        let name = Self::build_name(&children, 0);
        Self {
            children,
            active: 0,
            name,
            kind,
        }
    }

    fn build_name(children: &[Box<dyn Pane>], active: usize) -> String {
        let names: Vec<&str> = children.iter().map(|c| c.name()).collect();
        if active < names.len() {
            format!("{} [{}]", names[active], names.join(" | "))
        } else {
            names.join(" | ")
        }
    }

    #[allow(dead_code)]
    pub fn active_mut(&mut self) -> Option<&mut Box<dyn Pane>> {
        self.children.get_mut(self.active)
    }

    #[allow(dead_code)]
    pub fn active(&self) -> Option<&Box<dyn Pane>> {
        self.children.get(self.active)
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let mut spans = Vec::new();
        for (i, child) in self.children.iter().enumerate() {
            if i == self.active {
                spans.push(Span::styled(
                    format!(" {} ", child.name()),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(
                    format!(" {} ", child.name()),
                    Style::default().fg(Color::Gray),
                ));
            }
            if i < self.children.len() - 1 {
                spans.push(Span::raw(" "));
            }
        }
        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, area);
    }
}

impl Pane for TabContainer {
    fn kind(&self) -> PaneKind {
        self.kind
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        let tab_height = 1u16;
        if area.height <= tab_height {
            return;
        }
        let child_area = Rect {
            x: area.x,
            y: area.y + tab_height,
            width: area.width,
            height: area.height - tab_height,
        };
        self.render_tab_bar(f, Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: tab_height,
        });
        if let Some(child) = self.children.get_mut(self.active) {
            child.render(f, child_area, focused);
        } else {
            let text = Paragraph::new(Line::from(Span::styled(
                "No active tab",
                Style::default().fg(Color::DarkGray),
            )));
            f.render_widget(text, child_area);
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        if let Some(child) = self.children.get_mut(self.active) {
            child.write_input(bytes);
        }
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        for child in self.children.iter_mut() {
            child.resize(cols, rows.saturating_sub(1));
        }
    }

    fn kill(&mut self) {
        for child in self.children.iter_mut() {
            child.kill();
        }
    }

    fn get_cursor_line(&self) -> Option<String> {
        self.children
            .get(self.active)
            .and_then(|c| c.get_cursor_line())
    }

    fn get_status_line(&self) -> Option<String> {
        self.children
            .get(self.active)
            .and_then(|c| c.get_status_line())
    }

    fn get_visible_lines(&self) -> Vec<String> {
        self.children
            .get(self.active)
            .map(|c| c.get_visible_lines())
            .unwrap_or_default()
    }

    fn scroll(&mut self, rows: i16) {
        if let Some(child) = self.children.get_mut(self.active) {
            child.scroll(rows);
        }
    }

    fn push_context(&mut self, ctx: &AtelierContext) {
        for child in self.children.iter_mut() {
            child.push_context(ctx);
        }
    }

    fn switch_tab(&mut self, delta: i8) {
        if self.children.is_empty() {
            return;
        }
        let len = self.children.len();
        let new_active = if delta > 0 {
            (self.active + 1) % len
        } else {
            (self.active + len - 1) % len
        };
        self.active = new_active;
        self.name = Self::build_name(&self.children, self.active);
    }

    fn switch_to_tab(&mut self, kind: PaneKind) -> bool {
        if let Some(pos) = self.children.iter().position(|c| c.kind() == kind) {
            self.active = pos;
            self.name = Self::build_name(&self.children, self.active);
            true
        } else {
            false
        }
    }
}
