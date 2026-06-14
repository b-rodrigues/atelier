use crate::pane::{Pane, PaneKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone)]
struct VarEntry {
    name: String,
    var_type: String,
    value: String,
}

pub struct VarsPane {
    entries: Vec<VarEntry>,
    scroll: usize,
    selected_index: usize,
    csv_path: PathBuf,
    last_modified: Option<SystemTime>,
}

impl VarsPane {
    pub fn new(csv_path: PathBuf) -> Self {
        let mut pane = Self {
            entries: Vec::new(),
            scroll: 0,
            selected_index: 0,
            csv_path,
            last_modified: None,
        };
        pane.check_updates();
        pane
    }

    fn read_csv(path: &PathBuf) -> Vec<VarEntry> {
        if !path.exists() {
            return vec![];
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut entries = Vec::new();
        let mut lines = content.lines();
        let _header = lines.next();

        for line in lines {
            let parts: Vec<&str> = line.splitn(3, ',').collect();
            if parts.len() == 3 {
                entries.push(VarEntry {
                    name: parts[0].trim_matches('"').to_string(),
                    var_type: parts[1].trim_matches('"').to_string(),
                    value: parts[2].trim_matches('"').to_string(),
                });
            }
        }
        entries
    }

    fn check_updates(&mut self) {
        if let Ok(metadata) = std::fs::metadata(&self.csv_path) {
            if let Ok(modified) = metadata.modified() {
                if Some(modified) != self.last_modified {
                    self.entries = Self::read_csv(&self.csv_path);
                    self.last_modified = Some(modified);
                    self.selected_index = 0;
                    self.scroll = 0;
                }
            }
        } else {
            if !self.entries.is_empty() {
                self.entries.clear();
                self.last_modified = None;
            }
        }
    }

    fn style_for_type(var_type: &str) -> Style {
        match var_type {
            "Int" | "Float" => Style::default().fg(Color::Green),
            "String" => Style::default().fg(Color::Magenta),
            _ => Style::default().fg(Color::White),
        }
    }
}

impl Pane for VarsPane {
    fn kind(&self) -> PaneKind {
        PaneKind::Variables
    }

    fn name(&self) -> &str {
        "Variables"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        self.check_updates();

        let rows = area.height as usize;
        let cols = area.width as usize;

        let mut lines: Vec<Line> = Vec::new();

        let name_col = (cols as f32 * 0.25) as usize;
        let type_col = (cols as f32 * 0.2) as usize;

        let header = Line::from(vec![
            Span::styled(
                format!("{:width$}", "Name", width = name_col),
                Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:width$}", "Type", width = type_col),
                Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "Value",
                Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]);
        lines.push(header);

        let max_display = rows.saturating_sub(1);
        if max_display == 0 { return; }

        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }

        if self.selected_index < self.scroll {
            self.scroll = self.selected_index;
        }
        if self.selected_index >= self.scroll + max_display {
            self.scroll = self.selected_index.saturating_add(1).saturating_sub(max_display);
        }

        let start = self.scroll.min(self.entries.len().saturating_sub(max_display));
        let end = (start + max_display).min(self.entries.len());

        for (i, entry) in self.entries[start..end].iter().enumerate() {
            let global_idx = start + i;
            let mut value = entry.value.clone();
            if value.len() > cols.saturating_sub(name_col + type_col + 3) {
                value.truncate(cols.saturating_sub(name_col + type_col + 6));
                value.push_str("...");
            }

            let is_selected = focused && global_idx == self.selected_index;
            let base_style = Self::style_for_type(&entry.var_type);

            let name_span = if is_selected {
                Span::styled(
                    format!("{:width$}", entry.name, width = name_col),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                )
            } else {
                Span::raw(format!("{:width$}", entry.name, width = name_col))
            };

            let type_span = if is_selected {
                Span::styled(
                    format!("{:width$}", entry.var_type, width = type_col),
                    base_style.bg(Color::DarkGray),
                )
            } else {
                Span::raw(format!("{:width$}", entry.var_type, width = type_col))
            };

            let value_span = if is_selected {
                Span::styled(value, base_style.bg(Color::DarkGray))
            } else {
                Span::styled(value, base_style)
            };

            lines.push(Line::from(
                vec![name_span, Span::raw(" "), type_span, Span::raw(" "), value_span],
            ));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn write_input(&mut self, bytes: &[u8]) {
        if bytes == b"j" || bytes == b"\x1b[B" {
            if !self.entries.is_empty() {
                self.selected_index = (self.selected_index + 1) % self.entries.len();
            }
        } else if bytes == b"k" || bytes == b"\x1b[A" {
            if !self.entries.is_empty() {
                self.selected_index = if self.selected_index == 0 {
                    self.entries.len() - 1
                } else {
                    self.selected_index - 1
                };
            }
        }
    }

    fn explain_name(&self) -> Option<String> {
        self.entries.get(self.selected_index).map(|e| e.name.clone())
    }
}
