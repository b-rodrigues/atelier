use crate::app::{App, InputMode, Overlay, MaximizedState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let status_height = if matches!(app.mode, InputMode::Navigation) && app.overlay.is_none() {
        3
    } else {
        1
    };

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(status_height)])
        .split(area);

    let main_area = main[0];
    let mut pane_areas = [Rect::default(); 4];

    // Map physical positions to pane indices
    use crate::pane::PaneKind;
    let mut physical_to_pane_idx = [0usize; 4];
    let mut kinds = vec![PaneKind::Editor, PaneKind::Repl, PaneKind::Variables, PaneKind::Terminal];
    for (phys_pos, kind_str) in app.config.layout.positions.iter().enumerate() {
        if phys_pos >= 4 { break; }
        let kind = match kind_str.as_str() {
            "editor" => PaneKind::Editor,
            "repl" => PaneKind::Repl,
            "variables" => PaneKind::Variables,
            "terminal" => PaneKind::Terminal,
            "diagram" => PaneKind::Diagram,
            "plot" => PaneKind::Plot,
            _ => continue,
        };
        kinds[phys_pos] = kind;
    }
    for (phys_pos, kind) in kinds.iter().enumerate() {
        if let Some(idx) = app.panes.iter().position(|p| p.kind() == *kind) {
            physical_to_pane_idx[phys_pos] = idx;
        } else {
            physical_to_pane_idx[phys_pos] = std::cmp::min(phys_pos, app.panes.len().saturating_sub(1));
        }
    }

    let focused_physical_pos = physical_to_pane_idx
        .iter()
        .position(|&idx| idx == app.focus)
        .unwrap_or(0);

    let mut physical_areas = [Rect::default(); 4];

    match app.maximized {
        MaximizedState::Full => {
            for i in 0..4 {
                if i == focused_physical_pos {
                    physical_areas[i] = main_area;
                } else {
                    physical_areas[i] = Rect::default();
                }
            }
        }
        MaximizedState::Vertical => {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_area);
            
            if focused_physical_pos == 0 || focused_physical_pos == 2 {
                if focused_physical_pos == 0 {
                    physical_areas[0] = cols[0];
                    physical_areas[2] = Rect::default();
                } else {
                    physical_areas[2] = cols[0];
                    physical_areas[0] = Rect::default();
                }
                let r_split = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(cols[1]);
                physical_areas[1] = r_split[0];
                physical_areas[3] = r_split[1];
            } else {
                if focused_physical_pos == 1 {
                    physical_areas[1] = cols[1];
                    physical_areas[3] = Rect::default();
                } else {
                    physical_areas[3] = cols[1];
                    physical_areas[1] = Rect::default();
                }
                let l_split = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(cols[0]);
                physical_areas[0] = l_split[0];
                physical_areas[2] = l_split[1];
            }
        }
        MaximizedState::Horizontal => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_area);
            
            if focused_physical_pos == 0 || focused_physical_pos == 1 {
                if focused_physical_pos == 0 {
                    physical_areas[0] = rows[0];
                    physical_areas[1] = Rect::default();
                } else {
                    physical_areas[1] = rows[0];
                    physical_areas[0] = Rect::default();
                }
                let b_split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rows[1]);
                physical_areas[2] = b_split[0];
                physical_areas[3] = b_split[1];
            } else {
                if focused_physical_pos == 2 {
                    physical_areas[2] = rows[1];
                    physical_areas[3] = Rect::default();
                } else {
                    physical_areas[3] = rows[1];
                    physical_areas[2] = Rect::default();
                }
                let t_split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rows[0]);
                physical_areas[0] = t_split[0];
                physical_areas[1] = t_split[1];
            }
        }
        MaximizedState::None => {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_area);
            
            let l_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(cols[0]);
            
            let r_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(cols[1]);
            
            physical_areas[0] = l_split[0];
            physical_areas[1] = r_split[0];
            physical_areas[2] = l_split[1];
            physical_areas[3] = r_split[1];
        }
    }

    for phys_pos in 0..4 {
        let pane_idx = physical_to_pane_idx[phys_pos];
        pane_areas[pane_idx] = physical_areas[phys_pos];
    }

    let pane_indices = [0usize, 1, 2, 3];
    for idx in pane_indices {
        let area = pane_areas[idx];
        if area.width == 0 || area.height == 0 {
            continue;
        }
        if let Some(pane) = app.panes.get_mut(idx) {
            let is_focused = idx == app.focus;
            let block = make_block(pane.name(), is_focused);
            let inner = block.inner(area);
            pane.resize(inner.width, inner.height);
            f.render_widget(block, area);
            pane.render(f, inner, is_focused);
        } else {
            let placeholder = format!("Pane {} (not yet initialized)", idx + 1);
            let block = make_block(&placeholder, false);
            f.render_widget(block, area);
        }
    }

    // Render non-grid panes full-screen when focused
    if app.focus >= 4 && app.maximized == MaximizedState::Full {
        if let Some(pane) = app.panes.get_mut(app.focus) {
            let block = make_block(pane.name(), true);
            let inner = block.inner(main_area);
            pane.resize(inner.width, inner.height);
            f.render_widget(Clear, main_area);
            f.render_widget(block, main_area);
            pane.render(f, inner, true);
        }
    }

    if let Some(overlay) = app.overlay {
        render_overlay(f, area, overlay, app);
    }

    let status = status_bar(app);
    f.render_widget(status, main[1]);
}

fn render_overlay(f: &mut Frame, area: Rect, overlay: Overlay, app: &App) {
    match overlay {
        Overlay::FileTree => render_filetree_overlay(f, area, app),
        Overlay::BufferList => render_bufferlist_overlay(f, area, app),
        Overlay::Help => render_help_overlay(f, area),
        Overlay::QuitConfirm => render_quit_overlay(f, area),
        Overlay::ProjectSwitcher => render_project_switcher_overlay(f, area, app),
    }
}

fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height * (100 - percent_y)) / 200),
            Constraint::Length((r.height * percent_y) / 100),
            Constraint::Length((r.height * (100 - percent_y)) / 200),
        ])
        .split(r);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width * (100 - percent_x)) / 200),
            Constraint::Length((r.width * percent_x) / 100),
            Constraint::Length((r.width * (100 - percent_x)) / 200),
        ])
        .split(popup_layout[1]);

    horizontal[1]
}

fn render_filetree_overlay(f: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 50, 60);
    f.render_widget(Clear, rect);

    let items: Vec<ListItem> = app
        .filetree_entries
        .iter()
        .enumerate()
        .map(|(i, (path, is_dir))| {
            let display_name = if path == ".." {
                "..".to_string()
            } else {
                std::path::Path::new(path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            };
            let content = if *is_dir {
                if path == ".." {
                    "  📁 ..".to_string()
                } else {
                    format!("  📁 {}/", display_name)
                }
            } else {
                format!("  📄 {}", display_name)
            };
            let style = if i == app.filetree_selection {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if *is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" File Tree ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

    f.render_widget(list, rect);
}

fn render_bufferlist_overlay(f: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 60, 40);
    f.render_widget(Clear, rect);

    let mut text = vec![
        Line::from(Span::styled(
            " Open Buffers in Editor ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if let Ok(content) = std::fs::read_to_string("/tmp/atelier-buffers.txt") {
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.is_empty() {
            text.push(Line::from("  No open buffers or editor not loaded."));
        } else {
            for line in lines {
                text.push(Line::from(format!("  {}", line)));
            }
        }
    } else {
        text.push(Line::from("  Querying editor buffers..."));
    }

    text.push(Line::from(""));
    if app.buffer_input.is_empty() {
        text.push(Line::from(Span::styled(
            "  Type buffer number and press Enter to switch",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        text.push(Line::from(vec![
            Span::raw("  Switch to buffer: "),
            Span::styled(&app.buffer_input, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" (Press Enter)"),
        ]));
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        " Press Esc/q to close overlay",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Buffer List ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, rect);
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let rect = centered_rect(area, 50, 70);
    f.render_widget(Clear, rect);

    let alt_key = if cfg!(target_os = "macos") { "Option" } else { "Alt" };
    let help_text = vec![
        Line::from(Span::styled(" Keybindings", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(format!(" {}-Space   Enter navigation mode", alt_key)),
        Line::from(" Ctrl-d      Quit (saves files)"),
        Line::from(""),
        Line::from(Span::styled(" Navigation Mode:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(" Alt-↑/↓    Scroll focused pane"),
        Line::from(" 1-4         Focus pane 1-4"),
        Line::from(" 5/t         Focus REPL transcript"),
        Line::from(" e           Send to editor (clipboard / transcript selection)"),
        Line::from(" l           Send line to REPL (cursor line / transcript selection)"),
        Line::from(" r           Send region to REPL (clipboard / transcript selection)"),
        Line::from(" a           Push context to Assistant pane + focus it"),
        Line::from(" f           File tree browser"),
        Line::from(" b           Buffer list hint"),
        Line::from(" p           Project switcher (recent repos)"),
        Line::from(" m           Maximize/restore focused pane fully"),
        Line::from(" v           Maximize/restore focused pane vertically"),
        Line::from(" h           Maximize/restore focused pane horizontally"),
        Line::from(" =           Restore all pane sizes"),
        Line::from(" c           Focus Plot pane"),
        Line::from(" d           Focus Diagram pane"),
        Line::from(" ?           Show this help"),
        Line::from(" Esc         Back to normal mode"),
        Line::from(" q           Quit"),
        Line::from(""),
        Line::from(Span::styled(" Transcript Pane:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(" Alt-↑/↓    Scroll entries"),
        Line::from(" Enter      Send selected entry to REPL"),
        Line::from(" o          Send selected entry to editor"),
        Line::from(""),
        Line::from(Span::styled(" Normal Mode:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(" All keys    Forwarded to focused PTY pane"),
        Line::from(" Alt-↑/↓    Scroll focused pane"),
        Line::from(" Ctrl-c      Send SIGINT"),
        Line::from(" Ctrl-Tab    Switch tab in Terminal/Assistant pane"),
        Line::from(""),
        Line::from(Span::styled(" File Tree:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(" Up/Down     Navigate"),
        Line::from(" Enter       Open file / Enter directory"),
        Line::from(" Esc/q       Close"),
        Line::from(""),
        Line::from(" Press any key to close"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, rect);
}

fn render_project_switcher_overlay(f: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(area, 50, 40);
    f.render_widget(Clear, rect);

    let items: Vec<ListItem> = app
        .recent_projects
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let display = if path.len() > 60 {
                format!("...{}", &path[path.len().saturating_sub(57)..])
            } else {
                path.clone()
            };
            let style = if i == app.project_switcher_selection {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("  {}", display)).style(style)
        })
        .collect();

    let list = if items.is_empty() {
        let empty = vec![ListItem::new("  No recent projects").style(Style::default().fg(Color::DarkGray))];
        List::new(empty)
    } else {
        List::new(items)
    }
    .block(
        Block::default()
            .title(" Recent Projects ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black)),
    )
    .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

    f.render_widget(list, rect);
}

fn render_quit_overlay(f: &mut Frame, area: Rect) {
    let rect = centered_rect(area, 30, 15);
    f.render_widget(Clear, rect);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Really quit? (y/n) ",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Quit ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(Color::White))
        .alignment(ratatui::layout::HorizontalAlignment::Center);

    f.render_widget(paragraph, rect);
}

fn make_block(title: &str, focused: bool) -> Block<'static> {
    let border_style = if focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_span = if focused {
        Span::styled(
            format!(" {} ", title),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" {} ", title),
            Style::default().fg(Color::Gray),
        )
    };

    Block::default()
        .title(title_span)
        .borders(Borders::ALL)
        .border_style(border_style)
}

fn status_bar(app: &App) -> Paragraph<'static> {
    let mode_str = if app.overlay.is_some() {
        "OVERLAY"
    } else {
        match app.mode {
            InputMode::Normal => "NORMAL",
            InputMode::Navigation => "NAV",
        }
    };

    let pane_name = app
        .focused_pane()
        .map(|p| p.name().to_string())
        .unwrap_or_default();

    let bg = if app.overlay.is_some() {
        Color::Red
    } else if matches!(app.mode, InputMode::Navigation) {
        Color::Yellow
    } else {
        Color::Green
    };

    let mut lines = Vec::new();

    if let Some(overlay) = app.overlay {
        let mut spans = vec![
            Span::styled(
                format!(" {} ", mode_str),
                Style::default()
                    .fg(Color::Black)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" {}  │ ", pane_name)),
        ];
        match overlay {
            Overlay::FileTree => {
                spans.push(Span::styled("▲▼", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Navigate  "));
                spans.push(Span::styled("Enter", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Open  "));
                spans.push(Span::styled("Esc/q", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Close"));
            }
            Overlay::BufferList | Overlay::Help => {
                spans.push(Span::styled("Esc/q/Space", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Close"));
            }
            Overlay::QuitConfirm => {
                spans.push(Span::raw("Really quit?  "));
                spans.push(Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
                spans.push(Span::raw(" Yes  "));
                spans.push(Span::styled("n/Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                spans.push(Span::raw(" No"));
            }
            Overlay::ProjectSwitcher => {
                spans.push(Span::styled("▲▼", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Navigate  "));
                spans.push(Span::styled("Enter", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Switch project  "));
                spans.push(Span::styled("Esc/q", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Close"));
            }
        }
        lines.push(Line::from(spans));
    } else {
        match app.mode {
            InputMode::Normal => {
                let mut spans = vec![
                    Span::styled(
                        format!(" {} ", mode_str),
                        Style::default()
                            .fg(Color::Black)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(" {}  │ ", pane_name)),
                ];
                let alt_key = if cfg!(target_os = "macos") { "Option" } else { "Alt" };
                spans.push(Span::styled(format!("{}-Space", alt_key), Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Nav Mode  "));
                spans.push(Span::styled("Ctrl-d", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Save & Quit  "));
                spans.push(Span::styled("Ctrl-c", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" Interrupt"));
                lines.push(Line::from(spans));
            }
            InputMode::Navigation => {
                let mut line1_spans = vec![
                    Span::styled(
                        format!(" {} ", mode_str),
                        Style::default()
                            .fg(Color::Black)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(" {}  │ ", pane_name)),
                ];
                line1_spans.push(Span::styled("1-4", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Focus  "));
                line1_spans.push(Span::styled("5/t", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Transcript  "));
                line1_spans.push(Span::styled("e", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Editor  "));
                line1_spans.push(Span::styled("l", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Line  "));
                line1_spans.push(Span::styled("r", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Region  "));
                line1_spans.push(Span::styled("a", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Assistant  "));
                line1_spans.push(Span::styled("f", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Tree  "));
                line1_spans.push(Span::styled("b", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Buffers  "));
                line1_spans.push(Span::styled("p", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Projects  "));
                line1_spans.push(Span::styled("?", Style::default().fg(Color::Yellow)));
                line1_spans.push(Span::raw(" Help"));
                lines.push(Line::from(line1_spans));

                // Line 2
                let prefix_len = mode_str.len() + 2 + pane_name.len() + 3;
                let line2_padding = format!("{}│ ", " ".repeat(prefix_len));
                let mut line2_spans = vec![
                    Span::raw(line2_padding),
                ];
                line2_spans.push(Span::styled("m", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Max Full  "));
                line2_spans.push(Span::styled("v", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Max Vert  "));
                line2_spans.push(Span::styled("h", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Max Horiz  "));
                line2_spans.push(Span::styled("=", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Restore  "));
                line2_spans.push(Span::styled("c", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Plot  "));
                line2_spans.push(Span::styled("d", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Diagram  "));
                line2_spans.push(Span::styled("Esc", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Normal  "));
                line2_spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
                line2_spans.push(Span::raw(" Quit"));
                lines.push(Line::from(line2_spans));
            }
        }
    }

    Paragraph::new(lines).wrap(Wrap { trim: false })
}
