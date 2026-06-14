use crate::pane::{Pane, PaneKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct TranscriptPane {
    entries: Vec<(String, String)>,
    scroll_offset: usize,
}

impl TranscriptPane {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scroll_offset: 0,
        }
    }

    fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Atelier REPL Transcript\n\n");
        for (i, (sent, response)) in self.entries.iter().enumerate() {
            md.push_str(&format!("## Entry {}\n\n", i + 1));
            md.push_str("### Sent\n\n```\n");
            md.push_str(sent);
            md.push_str("\n```\n\n");
            md.push_str("### Response\n\n```\n");
            md.push_str(response);
            md.push_str("\n```\n\n");
        }
        md
    }
}

impl Pane for TranscriptPane {
    fn kind(&self) -> PaneKind {
        PaneKind::Transcript
    }

    fn name(&self) -> &str {
        "Transcript"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        let mut lines: Vec<Line> = Vec::new();

        if self.entries.is_empty() {
            lines.push(Line::from(Span::styled(
                "No entries yet. Send code to the REPL (Nav mode → l/r) to populate the transcript.",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Nav mode keys:  e  send to editor  |  l  send line  |  r  send region",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            let start = self.scroll_offset.min(self.entries.len().saturating_sub(1));
            for (i, (sent, _)) in self.entries.iter().enumerate().skip(start) {
                let is_selected = i == self.scroll_offset.min(self.entries.len().saturating_sub(1));
                let prefix = if is_selected { "▸" } else { " " };
                let header = format!("{} Entry {}", prefix, i + 1);
                lines.push(Line::from(Span::styled(
                    header,
                    if is_selected {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    },
                )));
                lines.push(Line::from(Span::styled(
                    format!("  {}", sent),
                    Style::default().fg(Color::Green),
                )));
                lines.push(Line::from(""));
            }

            let idx = self.scroll_offset.min(self.entries.len().saturating_sub(1)) + 1;
            lines.push(Line::from(Span::styled(
                format!("{} / {} entries", idx, self.entries.len()),
                Style::default().fg(Color::DarkGray),
            )));
            if self.entries.len() > 1 {
                lines.push(Line::from(Span::styled(
                    "  Alt-↑/↓  scroll",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            lines.push(Line::from(Span::styled(
                "  Enter  send to REPL",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "  o      send to editor",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let block = Block::default()
            .title(if focused {
                Span::styled(
                    " Transcript ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(" Transcript ", Style::default().fg(Color::Gray))
            })
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        f.render_widget(block, area);
        f.render_widget(paragraph, inner);
    }

    fn scroll(&mut self, rows: i16) {
        if rows > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(rows as usize);
        } else {
            let max_scroll = self.entries.len().saturating_sub(1);
            self.scroll_offset = self.scroll_offset.saturating_add((-rows) as usize).min(max_scroll);
        }
    }

    fn get_selected_entry(&self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        let idx = self.scroll_offset.min(self.entries.len().saturating_sub(1));
        self.entries.get(idx).map(|(sent, _)| sent.as_str())
    }

    fn push_transcript_entry(&mut self, sent: &str, response: &str) {
        self.entries.push((sent.to_string(), response.to_string()));
        self.scroll_offset = self.entries.len().saturating_sub(1);
    }

    fn save_artifact(&mut self) -> Option<String> {
        let md = self.to_markdown();
        let _ = std::fs::write("/tmp/atelier-transcript.md", &md);
        Some(md)
    }

    fn write_input(&mut self, _bytes: &[u8]) {}
}
