use crate::pane::{Pane, PaneKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;
use ratatui_image::{StatefulImage, Resize};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use image::DynamicImage;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

enum LoadResult {
    Loaded(DynamicImage),
    Error(String),
}

pub struct PlotPane {
    directory: PathBuf,
    last_scan_time: Instant,
    files: Vec<PathBuf>,
    selected_index: usize,
    load_in_progress: bool,
    loaded_index: Option<usize>,
    protocol: Option<StatefulProtocol>,
    rx: Option<mpsc::Receiver<LoadResult>>,
    picker: Picker,
    status_text: Option<String>,
}

fn load_image_async(path: PathBuf, tx: mpsc::Sender<LoadResult>) {
    std::thread::spawn(move || {
        let result = (|| -> Result<DynamicImage, String> {
            let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode {}: {}", path.display(), e))
        })();
        match result {
            Ok(dyn_img) => { let _ = tx.send(LoadResult::Loaded(dyn_img)); }
            Err(msg) => { let _ = tx.send(LoadResult::Error(msg)); }
        }
    });
}

impl PlotPane {
    pub fn new(directory: PathBuf) -> Self {
        let mut pane = Self {
            directory,
            last_scan_time: Instant::now(),
            files: Vec::new(),
            selected_index: 0,
            load_in_progress: false,
            loaded_index: None,
            protocol: None,
            rx: None,
            picker: Picker::halfblocks(),
            status_text: None,
        };
        pane.scan();
        pane
    }

    fn scan(&mut self) {
        self.files = match std::fs::read_dir(&self.directory) {
            Ok(entries) => {
                let mut paths: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| {
                        p.extension().is_some_and(|ext| {
                            matches!(ext.to_str(), Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"))
                        })
                    })
                    .collect();
                paths.sort();
                paths
            }
            Err(_) => {
                self.status_text = Some("Plot directory not found".to_string());
                Vec::new()
            }
        };

        self.selected_index = 0;
        self.loaded_index = None;
        self.protocol = None;
        self.load_in_progress = false;
        if !self.files.is_empty() {
            self.load_current();
        }
    }

    fn load_current(&mut self) {
        if self.files.is_empty() {
            self.protocol = None;
            self.loaded_index = None;
            return;
        }

        if self.load_in_progress {
            return;
        }

        if Some(self.selected_index) == self.loaded_index && self.protocol.is_some() {
            return;
        }

        let path = self.files[self.selected_index].clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.load_in_progress = true;
        self.protocol = None;
        self.status_text = Some("Loading...".to_string());
        load_image_async(path, tx);
    }

    fn check_loads(&mut self) {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(LoadResult::Loaded(dyn_img)) => {
                    self.protocol = Some(self.picker.new_resize_protocol(dyn_img));
                    self.loaded_index = Some(self.selected_index);
                    self.load_in_progress = false;
                    self.status_text = None;
                    self.rx = None;
                }
                Ok(LoadResult::Error(msg)) => {
                    self.status_text = Some(msg);
                    self.load_in_progress = false;
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status_text = Some("Load process failed".to_string());
                    self.load_in_progress = false;
                    self.rx = None;
                }
            }
        }
    }

    fn maybe_rescan(&mut self) {
        if self.last_scan_time.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_scan_time = Instant::now();

        let dir_exists = Path::new(&self.directory).exists();
        if !dir_exists && !self.files.is_empty() {
            self.files.clear();
            self.selected_index = 0;
            self.loaded_index = None;
            self.protocol = None;
            self.status_text = Some("Plot directory not found".to_string());
            return;
        }

        if dir_exists {
            let count = match std::fs::read_dir(&self.directory) {
                Ok(entries) => entries.filter_map(|e| e.ok()).count(),
                Err(_) => 0,
            };

            if count != self.files.len() {
                let old_selected = self.files.get(self.selected_index).cloned();
                self.files.clear();
                if let Ok(entries) = std::fs::read_dir(&self.directory) {
                    let mut paths: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok().map(|e| e.path()))
                        .filter(|p| {
                            p.extension().is_some_and(|ext| {
                                matches!(ext.to_str(), Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"))
                            })
                        })
                        .collect();
                    paths.sort();
                    self.files = paths;
                }
                if let Some(old) = old_selected {
                    if let Some(pos) = self.files.iter().position(|p| *p == old) {
                        self.selected_index = pos;
                    } else {
                        self.selected_index = self.files.len().saturating_sub(1);
                    }
                }
                self.loaded_index = None;
                self.protocol = None;
                if !self.files.is_empty() {
                    self.load_current();
                }
            }
        }
    }
}

impl Pane for PlotPane {
    fn kind(&self) -> PaneKind {
        PaneKind::Plot
    }

    fn name(&self) -> &str {
        "Plot"
    }

    fn set_image_backend(&mut self, picker: Option<&Picker>) {
        if let Some(p) = picker {
            self.picker = p.clone();
        }
        self.loaded_index = None;
        self.protocol = None;
        if !self.files.is_empty() {
            self.load_current();
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, _focused: bool) {
        self.check_loads();
        self.maybe_rescan();

        if let Some(ref mut protocol) = self.protocol {
            let si = StatefulImage::new().resize(Resize::Fit(None));
            f.render_stateful_widget(si, area, protocol);
        } else {
            let mut lines = Vec::new();

            if let Some(ref status) = self.status_text {
                lines.push(Line::from(Span::styled(
                    status,
                    Style::default().fg(Color::Yellow),
                )));
            } else if self.files.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No plots found.",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "Loading...",
                    Style::default().fg(Color::Yellow),
                )));
            }

            if !self.files.is_empty() {
                lines.push(Line::from(""));
                let idx = self.selected_index + 1;
                let total = self.files.len();
                lines.push(Line::from(Span::styled(
                    format!("[{idx}/{total}]"),
                    Style::default().fg(Color::DarkGray),
                )));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Path: {}", self.directory.display()),
                Style::default().fg(Color::DarkGray),
            )));

            let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        match bytes {
            b"r" => {
                self.last_scan_time = Instant::now() - Duration::from_secs(1);
                self.maybe_rescan();
            }

            b"\x1b[D" | b"h" => {
                if self.files.is_empty() {
                    return;
                }
                self.selected_index = if self.selected_index == 0 {
                    self.files.len() - 1
                } else {
                    self.selected_index - 1
                };
                self.load_current();
            }
            b"\x1b[C" | b"l" => {
                if self.files.is_empty() {
                    return;
                }
                self.selected_index = if self.selected_index + 1 >= self.files.len() {
                    0
                } else {
                    self.selected_index + 1
                };
                self.load_current();
            }
            _ => {}
        }
    }
}
