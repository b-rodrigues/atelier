use crate::app::{App, InputMode, Overlay};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main[0]);

    let left_panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(panes[0]);

    let right_panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(panes[1]);

    let pane_areas = [
        (0usize, left_panes[0]),
        (1, left_panes[1]),
        (2, right_panes[0]),
        (3, right_panes[1]),
    ];

    for (idx, area) in pane_areas {
        if let Some(pane) = app.panes.get_mut(idx) {
            let is_focused = idx == app.focus;
            let block = make_block(pane.name(), is_focused);
            let inner = block.inner(area);
            f.render_widget(block, area);
            pane.render(f, inner);
        } else {
            let placeholder = format!("Pane {} (not yet initialized)", idx + 1);
            let block = make_block(&placeholder, false);
            f.render_widget(block, area);
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
        Overlay::BufferList => render_bufferlist_overlay(f, area),
        Overlay::Help => render_help_overlay(f, area),
        Overlay::QuitConfirm => render_quit_overlay(f, area),
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
            let content = if *is_dir {
                format!("  {} /", path)
            } else {
                format!("  {}", path)
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

fn render_bufferlist_overlay(f: &mut Frame, area: Rect) {
    let rect = centered_rect(area, 40, 30);
    f.render_widget(Clear, rect);

    let text = vec![
        Line::from(Span::styled(
            " Buffer list sent to nvim (:ls)",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(" Use :b<number> to switch buffers"),
        Line::from(" Press Esc/q to close"),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Buffers ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(Color::White))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(paragraph, rect);
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let rect = centered_rect(area, 50, 70);
    f.render_widget(Clear, rect);

    let help_text = vec![
        Line::from(Span::styled(" Keybindings", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(" Ctrl-Space  Enter navigation mode"),
        Line::from(" Ctrl-d      Quit (saves files)"),
        Line::from(""),
        Line::from(Span::styled(" Navigation Mode:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(" 1-4         Focus pane 1-4"),
        Line::from(" f           File tree browser"),
        Line::from(" b           Buffer list hint"),
        Line::from(" e           Send clipboard to REPL"),
        Line::from(" ?           Show this help"),
        Line::from(" Esc         Back to normal mode"),
        Line::from(" q           Quit"),
        Line::from(""),
        Line::from(Span::styled(" Normal Mode:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(" All keys    Forwarded to focused PTY pane"),
        Line::from(" Ctrl-c      Send SIGINT"),
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
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(paragraph, rect);
}

fn make_block(title: &str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(style)
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

    let text = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .fg(Color::Black)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {} ", pane_name)),
    ]);

    Paragraph::new(text).wrap(Wrap { trim: false })
}
