use crate::app::{App, InputMode, Overlay, MaximizedState};
use crate::pane::PaneKind;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
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
        Event::Mouse(mouse) => {
            if let Some(pane) = app.focused_pane_mut() {
                match mouse.kind {
                    MouseEventKind::ScrollUp => pane.scroll(3),
                    MouseEventKind::ScrollDown => pane.scroll(-3),
                    _ => {}
                }
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
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.overlay = None;
                }
                KeyCode::Char(' ') => {
                    if app.buffer_input.is_empty() {
                        app.overlay = None;
                    }
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    app.buffer_input.push(c);
                }
                KeyCode::Backspace => {
                    app.buffer_input.pop();
                }
                KeyCode::Enter => {
                    if !app.buffer_input.is_empty() {
                        let cmd = format!("\x1b\x1b:b {}\r", app.buffer_input);
                        for pane in app.panes.iter_mut() {
                            if pane.kind() == PaneKind::Editor {
                                pane.write_input(cmd.as_bytes());
                            }
                        }
                        app.overlay = None;
                    }
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
        Some(Overlay::ProjectSwitcher) => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.overlay = None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.project_switcher_selection > 0 {
                        app.project_switcher_selection -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.project_switcher_selection + 1 < app.recent_projects.len() {
                        app.project_switcher_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(path) = app.recent_projects.get(app.project_switcher_selection).cloned() {
                        app.should_relaunch = Some(path);
                        app.overlay = None;
                    }
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
        KeyCode::Esc | KeyCode::Char('q') => {
            app.overlay = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.filetree_selection > 0 {
                app.filetree_selection -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.filetree_selection + 1 < app.filetree_entries.len() {
                app.filetree_selection += 1;
            }
        }
        KeyCode::Enter => {
            let entries = app.filetree_entries.clone();
            if let Some((path, is_dir)) = entries.get(app.filetree_selection) {
                if *is_dir {
                    if path == ".." {
                        if let Some(parent) = std::path::Path::new(&app.filetree_current).parent() {
                            app.filetree_current = parent.to_string_lossy().to_string();
                        }
                    } else {
                        app.filetree_current = path.clone();
                    }
                    refresh_filetree(app);
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

fn refresh_filetree(app: &mut App) {
    let start = app.filetree_current.clone();
    let mut entries = Vec::new();
    
    let base_path = std::path::Path::new(&app.filetree_base);
    let start_path = std::path::Path::new(&start);
    if start_path != base_path {
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
        (KeyCode::Char(' '), KeyModifiers::ALT) => {
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
        (KeyCode::Tab, KeyModifiers::CONTROL) => {
            if let Some(pane) = app.focused_pane_mut() {
                pane.switch_tab(1);
            }
            Ok(Action::None)
        }
        (KeyCode::Tab, mods) if mods == KeyModifiers::CONTROL | KeyModifiers::SHIFT => {
            if let Some(pane) = app.focused_pane_mut() {
                pane.switch_tab(-1);
            }
            Ok(Action::None)
        }
        (KeyCode::Up, KeyModifiers::ALT) => {
            if let Some(pane) = app.focused_pane_mut() {
                pane.scroll(3);
            }
            Ok(Action::None)
        }
        (KeyCode::Down, KeyModifiers::ALT) => {
            if let Some(pane) = app.focused_pane_mut() {
                pane.scroll(-3);
            }
            Ok(Action::None)
        }
        _ => {
            if key.code == KeyCode::Enter {
                if let Some(name) = app.focused_pane().and_then(|p| p.explain_name()) {
                    let cmd = format!("explain({})\r", name);
                    if let Some(idx) = app.panes.iter().position(|p| p.kind() == PaneKind::Repl) {
                        app.panes[idx].write_input(cmd.as_bytes());
                    }
                    return Ok(Action::None);
                }
            }
            if let Some(pane) = app.focused_pane_mut() {
                let bytes = key_event_to_bytes(key);
                pane.write_input(&bytes);
            }
            Ok(Action::None)
        }
    }
}

fn handle_navigation_key(app: &mut App, key: KeyEvent) -> Result<Action> {
    if key.modifiers == KeyModifiers::ALT {
        match key.code {
            KeyCode::Up => {
                if let Some(pane) = app.focused_pane_mut() {
                    pane.scroll(3);
                }
                return Ok(Action::None);
            }
            KeyCode::Down => {
                if let Some(pane) = app.focused_pane_mut() {
                    pane.scroll(-3);
                }
                return Ok(Action::None);
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Char('1') => {
            let preserve = app.should_preserve_maximized(0);
            if let Some(idx) = app.pane_index_at_physical_pos(0) {
                app.focus = idx;
            }
            if !preserve {
                app.maximized = MaximizedState::None;
            }
        }
        KeyCode::Char('2') => {
            let preserve = app.should_preserve_maximized(1);
            if let Some(idx) = app.pane_index_at_physical_pos(1) {
                app.focus = idx;
            }
            if !preserve {
                app.maximized = MaximizedState::None;
            }
        }
        KeyCode::Char('3') => {
            let preserve = app.should_preserve_maximized(2);
            if let Some(idx) = app.pane_index_at_physical_pos(2) {
                app.focus = idx;
            }
            if !preserve {
                app.maximized = MaximizedState::None;
            }
        }
        KeyCode::Char('4') => {
            let preserve = app.should_preserve_maximized(3);
            if let Some(idx) = app.pane_index_at_physical_pos(3) {
                app.focus = idx;
            }
            if !preserve {
                app.maximized = MaximizedState::None;
            }
        }
        KeyCode::Char('5') | KeyCode::Char('t') => {
            if let Some(idx) = app.panes.iter().position(|p| p.kind() == PaneKind::Transcript) {
                app.focus = idx;
                app.maximized = MaximizedState::Full;
            }
        }
        KeyCode::Char('c') => {
            if let Some(idx) = app.pane_index_at_physical_pos(2) {
                app.focus = idx;
                app.panes.get_mut(idx).map(|p| p.switch_to_tab(PaneKind::Plot));
                app.maximized = MaximizedState::Full;
            }
        }
        KeyCode::Char('d') => {
            if let Some(idx) = app.panes.iter().position(|p| p.kind() == PaneKind::Diagram) {
                app.focus = idx;
                app.maximized = MaximizedState::Full;
            }
        }
        KeyCode::Enter => {
            if let Some(pane) = app.focused_pane() {
                if pane.kind() == PaneKind::Transcript {
                    app.send_to_repl(false);
                    return Ok(Action::None);
                }
            }
        }
        KeyCode::Char('o') => {
            if let Some(pane) = app.focused_pane() {
                if pane.kind() == PaneKind::Transcript {
                    app.send_to_editor();
                    return Ok(Action::None);
                }
            }
        }
        KeyCode::Char('q') => {
            app.overlay = Some(Overlay::QuitConfirm);
            app.mode = InputMode::Normal;
            return Ok(Action::None);
        }
        KeyCode::Char('f') => {
            app.filetree_current = app.filetree_base.clone();
            refresh_filetree(app);
            app.overlay = Some(Overlay::FileTree);
        }
        KeyCode::Char('b') => {
            app.overlay = Some(Overlay::BufferList);
            app.buffer_input = String::new();
            for pane in app.panes.iter_mut() {
                if pane.kind() == PaneKind::Editor {
                    pane.write_input(b"\x1b\x1b:redir! > /tmp/atelier-buffers.txt | silent ls | redir END\r");
                }
            }
        }
        KeyCode::Char('?') => {
            app.overlay = Some(Overlay::Help);
        }
        KeyCode::Char('e') => {
            app.send_to_editor();
        }
        KeyCode::Char('l') => {
            app.send_to_repl(false);
        }
        KeyCode::Char('r') => {
            app.send_to_repl(true);
        }
        KeyCode::Char('a') => {
            app.refresh_llm_context();
            if let Some(idx) = app.pane_index_at_physical_pos(3) {
                app.focus = idx;
            }
        }
        KeyCode::Char('p') => {
            app.recent_projects = crate::config::RecentProjects::load().paths;
            app.project_switcher_selection = 0;
            app.overlay = Some(Overlay::ProjectSwitcher);
        }
        KeyCode::Char('m') => {
            if app.focused_pane().map(|p| p.kind() == PaneKind::Transcript).unwrap_or(false) {
                if app.maximized == MaximizedState::Full {
                    app.maximized = MaximizedState::None;
                    if let Some(idx) = app.pane_index_at_physical_pos(0) {
                        app.focus = idx;
                    }
                } else {
                    app.maximized = MaximizedState::Full;
                }
            } else {
                app.maximized = if app.maximized == MaximizedState::Full {
                    MaximizedState::None
                } else {
                    MaximizedState::Full
                };
            }
        }
        KeyCode::Char('v') => {
            if app.focused_pane().map(|p| p.kind() == PaneKind::Transcript).unwrap_or(false) {
                app.maximized = MaximizedState::None;
                if let Some(idx) = app.pane_index_at_physical_pos(0) {
                    app.focus = idx;
                }
            } else {
                app.maximized = if app.maximized == MaximizedState::Vertical {
                    MaximizedState::None
                } else {
                    MaximizedState::Vertical
                };
            }
        }
        KeyCode::Char('h') => {
            if app.focused_pane().map(|p| p.kind() == PaneKind::Transcript).unwrap_or(false) {
                app.maximized = MaximizedState::None;
                if let Some(idx) = app.pane_index_at_physical_pos(0) {
                    app.focus = idx;
                }
            } else {
                app.maximized = if app.maximized == MaximizedState::Horizontal {
                    MaximizedState::None
                } else {
                    MaximizedState::Horizontal
                };
            }
        }
        KeyCode::Char('=') => {
            if app.focused_pane().map(|p| p.kind() == PaneKind::Transcript).unwrap_or(false) {
                app.maximized = MaximizedState::None;
                if let Some(idx) = app.pane_index_at_physical_pos(0) {
                    app.focus = idx;
                }
            } else {
                app.maximized = MaximizedState::None;
            }
        }
        KeyCode::Tab => {
            if let Some(pane) = app.focused_pane_mut() {
                pane.switch_tab(1);
            }
            return Ok(Action::None);
        }
        KeyCode::BackTab => {
            if let Some(pane) = app.focused_pane_mut() {
                pane.switch_tab(-1);
            }
            return Ok(Action::None);
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
