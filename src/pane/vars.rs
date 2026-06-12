use crate::pane::{Pane, PaneKind};
use notify::{Config, Event, RecommendedWatcher, Watcher};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone)]
struct VarEntry {
    name: String,
    var_type: String,
    value: String,
}

pub struct VarsPane {
    entries: Vec<VarEntry>,
    scroll: usize,
    csv_path: PathBuf,
    _watcher: RecommendedWatcher,
    notify_rx: mpsc::Receiver<Result<Event, notify::Error>>,
}

impl VarsPane {
    pub fn new(csv_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .unwrap();

        if csv_path.exists() {
            let _ = watcher.watch(&csv_path, notify::RecursiveMode::NonRecursive);
        }
        if let Some(parent) = csv_path.parent() {
            if parent.exists() {
                let _ = watcher.watch(parent, notify::RecursiveMode::NonRecursive);
            }
        }

        let entries = Self::read_csv(&csv_path);

        Self {
            entries,
            scroll: 0,
            csv_path,
            _watcher: watcher,
            notify_rx: rx,
        }
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
        loop {
            match self.notify_rx.recv_timeout(Duration::from_millis(0)) {
                Ok(Ok(_)) => {
                    self.entries = Self::read_csv(&self.csv_path);
                }
                Ok(Err(_)) => break,
                Err(mpsc::RecvTimeoutError::Timeout) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
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

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.check_updates();

        let rows = area.height as usize;
        let cols = area.width as usize;

        let mut lines: Vec<Line> = Vec::new();

        if self.entries.is_empty() {
            let text = Paragraph::new(Line::from(Span::styled(
                "No user variables defined yet.",
                Style::default().fg(Color::DarkGray),
            )))
            .wrap(Wrap { trim: false });
            f.render_widget(text, area);
            return;
        }

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
        let start = self.scroll.min(self.entries.len().saturating_sub(max_display));
        let end = (start + max_display).min(self.entries.len());

        for entry in &self.entries[start..end] {
            let mut value = entry.value.clone();
            if value.len() > cols.saturating_sub(name_col + type_col + 3) {
                value.truncate(cols.saturating_sub(name_col + type_col + 6));
                value.push_str("...");
            }

            let style = Self::style_for_type(&entry.var_type);
            lines.push(Line::from(vec![
                Span::raw(format!("{:width$}", entry.name, width = name_col)),
                Span::raw(" "),
                Span::raw(format!("{:width$}", entry.var_type, width = type_col)),
                Span::raw(" "),
                Span::styled(value, style),
            ]));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn write_input(&mut self, bytes: &[u8]) {
        if bytes == b"j" {
            self.scroll = self.scroll.saturating_add(1);
        } else if bytes == b"k" {
            self.scroll = self.scroll.saturating_sub(1);
        }
    }
}
