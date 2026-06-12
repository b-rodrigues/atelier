use crate::app::{App, InputMode, Overlay};
use crate::pane::PaneKind;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub enum Action {
    Quit,
    None,
}

pub fn handle_events(app: &mut App) -> Result<Action> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(Action::None);
    }

    let event = event::read()?;

    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Resize(cols, rows) => {
            for pane in app.panes.iter_mut() {
                pane.resize(cols, rows);
            }
            Ok(Action::None)
        }
        _ => Ok(Action::None),
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<Action> {
    if app.overlay.is_some() {
        return handle_overlay_key(app, key);
    }
    match app.mode {
        InputMode::Navigation => handle_navigation_key(app, key),
        InputMode::Normal => handle_normal_key(app, key),
    }
}

fn handle_overlay_key(app: &mut App, key: KeyEvent) -> Result<Action> {
    match app.overlay {
        Some(Overlay::FileTree) => handle_filetree_key(app, key),
        Some(Overlay::BufferList) => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char(' ') => {
                    app.overlay = None;
                }
                _ => {}
            }
            Ok(Action::None)
        }
        Some(Overlay::Help) => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char(' ') => {
                    app.overlay = None;
                }
                _ => {}
            }
            Ok(Action::None)
        }
        Some(Overlay::QuitConfirm) => {
            match key.code {
                KeyCode::Char('y') => {
                    app.save_all_and_quit();
                    return Ok(Action::Quit);
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    app.overlay = None;
                }
                _ => {}
            }
            Ok(Action::None)
        }
        None => unreachable!(),
    }
}

fn handle_filetree_key(app: &mut App, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Esc => {
            app.overlay = None;
        }
        KeyCode::Char('q') => {
            app.overlay = None;
        }
        KeyCode::Up => {
            if app.filetree_selection > 0 {
                app.filetree_selection -= 1;
            }
        }
        KeyCode::Down => {
            if app.filetree_selection + 1 < app.filetree_entries.len() {
                app.filetree_selection += 1;
            }
        }
        KeyCode::Enter => {
            let entries = &app.filetree_entries.clone();
            if let Some((path, is_dir)) = entries.get(app.filetree_selection) {
                if *is_dir {
                    refresh_filetree(app, Some(path));
                } else {
                    app.open_in_editor(path);
                    app.overlay = None;
                }
            }
        }
        _ => {}
    }
    Ok(Action::None)
}

fn refresh_filetree(app: &mut App, subdir: Option<&str>) {
    let start = subdir.map(|s| s.to_string()).unwrap_or_else(|| app.filetree_base.clone());
    let mut entries = Vec::new();
    if subdir.is_some() && start != app.filetree_base {
        entries.push(("..".to_string(), true));
    }
    if let Ok(readdir) = std::fs::read_dir(&start) {
        for entry in readdir.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            entries.push((path.to_string_lossy().to_string(), is_dir));
        }
    }
    entries.sort_by(|a, b| {
        if a.1 != b.1 {
            b.1.cmp(&a.1)
        } else {
            a.0.cmp(&b.0)
        }
    });
    app.filetree_entries = entries;
    app.filetree_selection = 0;
    app.filetree_scroll = 0;
}

fn handle_normal_key(app: &mut App, key: KeyEvent) -> Result<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char(' '), KeyModifiers::CONTROL) => {
            app.mode = InputMode::Navigation;
            Ok(Action::None)
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            app.save_all_and_quit();
            Ok(Action::Quit)
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            if let Some(pane) = app.focused_pane_mut() {
                pane.write_input(b"\x03");
            }
            Ok(Action::None)
        }
        _ => {
            if let Some(pane) = app.focused_pane_mut() {
                let bytes = key_event_to_bytes(key);
                pane.write_input(&bytes);
            }
            Ok(Action::None)
        }
    }
}

fn handle_navigation_key(app: &mut App, key: KeyEvent) -> Result<Action> {
    match key.code {
        KeyCode::Char('1') => app.focus_pane(PaneKind::Editor),
        KeyCode::Char('2') => app.focus_pane(PaneKind::Repl),
        KeyCode::Char('3') => app.focus_pane(PaneKind::Variables),
        KeyCode::Char('4') => app.focus_pane(PaneKind::Terminal),
        KeyCode::Char('q') => {
            app.overlay = Some(Overlay::QuitConfirm);
            app.mode = InputMode::Normal;
            return Ok(Action::None);
        }
        KeyCode::Char('f') => {
            refresh_filetree(app, None);
            app.overlay = Some(Overlay::FileTree);
        }
        KeyCode::Char('b') => {
            app.overlay = Some(Overlay::BufferList);
        }
        KeyCode::Char('?') => {
            app.overlay = Some(Overlay::Help);
        }
        KeyCode::Char('e') => {
            app.send_to_repl();
        }
        KeyCode::Esc => {}
        _ => {
            app.mode = InputMode::Normal;
            return Ok(Action::None);
        }
    };
    app.mode = InputMode::Normal;
    Ok(Action::None)
}

fn key_event_to_bytes(key: KeyEvent) -> Vec<u8> {
    use KeyCode::*;
    match (key.code, key.modifiers) {
        (Enter, _) => b"\r".to_vec(),
        (Backspace, _) => b"\x7f".to_vec(),
        (Tab, _) => b"\t".to_vec(),
        (Esc, _) => b"\x1b".to_vec(),
        (Char(c), KeyModifiers::CONTROL) => {
            let code = if c >= 'a' && c <= 'z' {
                (c as u8) - b'a' + 1
            } else {
                c as u8
            };
            vec![code]
        }
        (Char(c), KeyModifiers::ALT) => {
            vec![0x1b, c as u8]
        }
        (Char(c), _) => vec![c as u8],
        (Left, _) => b"\x1b[D".to_vec(),
        (Right, _) => b"\x1b[C".to_vec(),
        (Up, _) => b"\x1b[A".to_vec(),
        (Down, _) => b"\x1b[B".to_vec(),
        (Home, _) => b"\x1b[H".to_vec(),
        (End, _) => b"\x1b[F".to_vec(),
        (Delete, _) => b"\x1b[3~".to_vec(),
        (PageUp, _) => b"\x1b[5~".to_vec(),
        (PageDown, _) => b"\x1b[6~".to_vec(),
        _ => vec![],
    }
}
