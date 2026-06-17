use crate::pane::{Pane, PaneKind};
use crate::renderer::{spawn_dot_render, RenderState};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;
use ratatui_image::{StatefulImage, Resize};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

pub struct DiagramPane {
    watch_path: PathBuf,
    last_modified: Option<SystemTime>,
    last_check_time: Instant,
    render_state: RenderState,
    render_in_progress: bool,
    pending_render: bool,
    rx: Option<mpsc::Receiver<RenderState>>,
    dot_command: String,
    dot_args: Vec<String>,
    source_text: String,
    picker: Picker,
    protocol: Option<StatefulProtocol>,
}

impl DiagramPane {
    pub fn new(
        watch_path: PathBuf,
        dot_command: String,
        dot_args: Vec<String>,
    ) -> Self {
        let mut pane = Self {
            watch_path,
            last_modified: None,
            last_check_time: Instant::now(),
            render_state: RenderState::Idle,
            render_in_progress: false,
            pending_render: false,
            rx: None,
            dot_command,
            dot_args,
            source_text: String::new(),
            picker: Picker::halfblocks(),
            protocol: None,
        };
        pane.check_updates();
        pane
    }

    fn check_updates(&mut self) {
        // Always drain channel
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(RenderState::DoneDecoded(dyn_img)) => {
                    self.render_state = RenderState::DoneDecoded(dyn_img.clone());
                    self.render_in_progress = false;
                    self.protocol = Some(self.picker.new_resize_protocol(dyn_img));
                    self.rx = None;
                    if self.pending_render {
                        self.pending_render = false;
                        self.last_modified = None;
                    }
                }
                Ok(RenderState::Error(msg)) => {
                    self.render_state = RenderState::Error(msg);
                    self.render_in_progress = false;
                    self.rx = None;
                    if self.pending_render {
                        self.pending_render = false;
                        self.last_modified = None;
                    }
                }
                Ok(RenderState::Rendering) => {
                    self.render_state = RenderState::Rendering;
                }
                Ok(RenderState::Idle) => {}
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    if matches!(self.render_state, RenderState::Rendering) {
                        self.render_state = RenderState::Error("Render process disconnected".into());
                    }
                    self.render_in_progress = false;
                    self.rx = None;
                }
            }
        }

        // Rate-limit filesystem check to 300ms
        if self.last_check_time.elapsed() < Duration::from_millis(300) {
            return;
        }
        self.last_check_time = Instant::now();

        let changed = match std::fs::metadata(&self.watch_path) {
            Ok(meta) => {
                if let Ok(modified) = meta.modified() {
                    if Some(modified) != self.last_modified {
                        self.last_modified = Some(modified);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Err(_) => {
                if self.last_modified.is_some() {
                    self.last_modified = None;
                    self.source_text.clear();
                    self.render_state = RenderState::Idle;
                    self.protocol = None;
                }
                return;
            }
        };

        if changed {
            self.source_text = std::fs::read_to_string(&self.watch_path).unwrap_or_default();
            self.protocol = None;

            if self.render_in_progress {
                self.pending_render = true;
                return;
            }

            let (tx, rx) = mpsc::channel();
            self.rx = Some(rx);
            let src = self.watch_path.clone();
            let cmd = self.dot_command.clone();
            let args = self.dot_args.clone();
            self.render_in_progress = true;
            spawn_dot_render(src, cmd, args, tx);
        }
    }
}

impl Pane for DiagramPane {
    fn kind(&self) -> PaneKind {
        PaneKind::Diagram
    }

    fn name(&self) -> &str {
        "Diagram"
    }

    fn set_image_backend(&mut self, picker: Option<&Picker>) {
        if let Some(p) = picker {
            self.picker = p.clone();
        }
        self.protocol = None;
    }

    fn render(&mut self, f: &mut Frame, area: Rect, _focused: bool) {
        self.check_updates();

        if let Some(ref mut protocol) = self.protocol {
            let si = StatefulImage::new().resize(Resize::Fit(None));
            f.render_stateful_widget(si, area, protocol);
        } else {
            let lines: Vec<Line> = match &self.render_state {
                RenderState::Idle => {
                    if self.source_text.is_empty() {
                        vec![
                            Line::from(Span::styled(
                                "No pipeline diagram available.",
                                Style::default().fg(Color::DarkGray),
                            )),
                            Line::from(Span::styled(
                                "T writes pipeline DOT to:",
                                Style::default().fg(Color::DarkGray),
                            )),
                            Line::from(Span::styled(
                                self.watch_path.display().to_string(),
                                Style::default().fg(Color::Cyan),
                            )),
                        ]
                    } else {
                        vec![
                            Line::from(Span::styled(
                                "Pipeline diagram source:",
                                Style::default().fg(Color::Yellow),
                            )),
                            Line::from(""),
                            Line::from(Span::raw(&self.source_text)),
                        ]
                    }
                }
                RenderState::Rendering => {
                    vec![
                        Line::from(Span::styled(
                            " Rendering diagram... ",
                            Style::default().fg(Color::Yellow),
                        )),
                        Line::from(Span::styled(
                            "  dot -Tpng ...",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ]
                }
                RenderState::DoneDecoded(_) => {
                    vec![Line::from(Span::styled(
                        " Processing diagram... ",
                        Style::default().fg(Color::Yellow),
                    ))]
                }
                RenderState::Error(msg) => {
                    vec![
                        Line::from(Span::styled(
                            " Diagram Error ",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(msg, Style::default().fg(Color::Red))),
                    ]
                }
            };
            let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        match bytes {
            b"r" => {
                self.last_modified = None;
                self.check_updates();
            }
            _ => {}
        }
    }
}
