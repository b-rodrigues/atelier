use crate::pane::{Pane, PaneKind};
use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtyPair, PtySize};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;
use std::io::Write;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

pub struct PtyPane {
    kind: PaneKind,
    name: String,
    writer: Box<dyn Write + Send>,
    reader: std::sync::mpsc::Receiver<Vec<u8>>,
    parser: vt100::Parser,
    master: Option<Box<dyn MasterPty + Send>>,
    cols: u16,
    rows: u16,
    dead: bool,
}

impl PtyPane {
    pub fn spawn(
        kind: PaneKind,
        name: String,
        command: &str,
        args: &[String],
        repo_path: Option<&str>,
    ) -> Result<Self> {
        let init_rows: u16 = 24;
        let init_cols: u16 = 80;
        let pty_system = portable_pty::native_pty_system();
        let pair: PtyPair = pty_system.openpty(PtySize {
            rows: init_rows,
            cols: init_cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = if let Some(path) = repo_path {
            let mut c = CommandBuilder::new("nix");
            c.arg("develop");
            c.arg(path);
            c.arg("--command");
            c.arg(command);
            for arg in args {
                c.arg(arg);
            }
            c.env("TLANG_REPO_ROOT", path);
            c.env("REPO_DIR", path);
            c
        } else {
            let mut c = CommandBuilder::new(command);
            for arg in args {
                c.arg(arg);
            }
            c
        };
        cmd.env("ATELIER_ACTIVE", "1");
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }

        let slave = pair.slave;
        slave.spawn_command(cmd)?;

        let writer = pair.master.take_writer()
            .context("Failed to take PTY writer")?;
        let mut reader = pair.master.try_clone_reader()
            .context("Failed to clone PTY reader")?;

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                let mut buf = vec![0u8; 4096];
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        buf.truncate(n);
                        if tx.send(buf).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let parser = vt100::Parser::new(init_rows, init_cols, 0);

        Ok(Self {
            kind,
            name,
            writer: Box::new(writer),
            reader: rx,
            parser,
            master: Some(pair.master),
            cols: init_cols,
            rows: init_rows,
            dead: false,
        })
    }

    fn drain_output(&mut self) {
        if self.dead {
            return;
        }
        loop {
            match self.reader.try_recv() {
                Ok(data) => {
                    self.parser.process(&data);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.dead = true;
                    break;
                }
            }
        }
    }
}

fn map_color(color: vt100::Color) -> Option<ratatui::style::Color> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(idx) => Some(ratatui::style::Color::Indexed(idx)),
        vt100::Color::Rgb(r, g, b) => Some(ratatui::style::Color::Rgb(r, g, b)),
    }
}

fn cell_style(cell: &vt100::Cell) -> ratatui::style::Style {
    let mut style = ratatui::style::Style::default();
    if let Some(fg) = map_color(cell.fgcolor()) {
        style = style.fg(fg);
    }
    if let Some(bg) = map_color(cell.bgcolor()) {
        let is_black = match bg {
            ratatui::style::Color::Black => true,
            ratatui::style::Color::Indexed(0) | ratatui::style::Color::Indexed(16) => true,
            ratatui::style::Color::Rgb(0, 0, 0) => true,
            _ => false,
        };
        if !is_black {
            style = style.bg(bg);
        }
    }
    if cell.bold() {
        style = style.add_modifier(ratatui::style::Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(ratatui::style::Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(ratatui::style::Modifier::UNDERLINED);
    }
    if cell.inverse() {
        style = style.add_modifier(ratatui::style::Modifier::REVERSED);
    }
    style
}

impl Pane for PtyPane {
    fn kind(&self) -> PaneKind {
        self.kind
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool) {
        self.drain_output();

        if self.dead {
            let text = Paragraph::new(vec![
                Line::from(Span::styled(
                    "Process exited / failed to start",
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Red)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("The command exited or could not be found."),
                Line::from(format!("Command: {}", self.name)),
            ])
            .wrap(Wrap { trim: true });
            f.render_widget(text, area);
            return;
        }

        let screen = self.parser.screen();
        let screen_size = screen.size();
        let screen_rows = screen_size.0;
        let screen_cols = screen_size.1;

        let display_rows = area.height as u16;
        let display_cols = area.width as u16;

        let mut lines: Vec<Line> = Vec::new();
        for r in 0..display_rows.min(screen_rows) {
            let mut spans = Vec::new();
            for c in 0..display_cols.min(screen_cols) {
                match screen.cell(r, c) {
                    Some(cell) => {
                        let contents = cell.contents();
                        let style = cell_style(cell);
                        spans.push(Span::styled(contents.to_string(), style));
                    }
                    None => {
                        spans.push(Span::raw(" "));
                    }
                }
            }
            lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);

        if focused {
            let (cursor_row, cursor_col) = screen.cursor_position();
            if cursor_row < display_rows && cursor_col < display_cols {
                f.set_cursor_position((
                    area.x + cursor_col.min(display_cols - 1),
                    area.y + cursor_row.min(display_rows - 1),
                ));
            }
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        let _ = self.writer.write(bytes);
        let _ = self.writer.flush();
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        if cols != self.cols || rows != self.rows {
            self.cols = cols;
            self.rows = rows;
            if let Some(master) = &self.master {
                let _ = master.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
            self.parser.set_size(rows, cols);
        }
    }

    fn kill(&mut self) {
        self.master.take();
    }

    fn get_cursor_line(&self) -> Option<String> {
        let screen = self.parser.screen();
        let (cursor_row, _) = screen.cursor_position();
        let screen_size = screen.size();
        let cols = screen_size.1;

        let mut line = String::new();
        for c in 0..cols {
            if let Some(cell) = screen.cell(cursor_row, c) {
                line.push_str(&cell.contents());
            } else {
                line.push(' ');
            }
        }
        Some(line)
    }
}
