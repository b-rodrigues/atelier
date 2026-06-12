mod app;
mod config;
mod event;
mod pane;
mod ui;

use crate::app::App;
use crate::config::Config;
use crate::event::Action;
use crate::pane::pty::PtyPane;
use crate::pane::PaneKind;
use anyhow::Result;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Write};

fn first_launch_prompt() -> Result<Config> {
    let mut stdout = stdout();
    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    )?;

    println!();
    println!("  No config found at ~/.config/atelier/config.toml");
    println!();
    println!("  Choose your editor:");
    println!("    [1] nvim    (recommended)");
    println!("    [2] vim");
    println!("    [3] nano");
    println!("    [4] vi");
    print!("  > ");
    stdout.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    let (editor, args) = match choice {
        "1" | "nvim" => ("nvim".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
        "2" | "vim" => ("vim".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
        "3" | "nano" => ("nano".to_string(), vec![]),
        "4" | "vi" => ("vi".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
        _ => ("nvim".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
    };

    let config = Config {
        editor: config::EditorConfig {
            command: editor,
            args,
        },
        ..Default::default()
    };

    config.save()?;

    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    Ok(config)
}

fn run(mut app: App) -> Result<()> {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &mut app))?;

        match event::handle_events(&mut app)? {
            Action::Quit => break,
            Action::None => {}
        }
    }

    app.kill_all();
    cleanup_temp_files();

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn cleanup_temp_files() {
    let _ = std::fs::remove_file("/tmp/atelier-vars.csv");
    let _ = std::fs::remove_file("/tmp/atelier-cmd");
}

fn main() -> Result<()> {
    let config = match Config::load()? {
        Some(c) => c,
        None => first_launch_prompt()?,
    };

    let repo_path = std::env::args().nth(1);

    let mut app = App::new(config, repo_path.clone());

    let mut editor_args = app.config.editor.args.clone();
    let cmd_name = std::path::Path::new(&app.config.editor.command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&app.config.editor.command);
    if editor_args.is_empty() && (cmd_name == "nvim" || cmd_name == "vim" || cmd_name == "vi") {
        editor_args = vec!["--cmd".to_string(), "set shortmess+=I".to_string()];
    }

    let editor = PtyPane::spawn(
        PaneKind::Editor,
        app.config.editor.command.clone(),
        &app.config.editor.command,
        &editor_args,
        repo_path.as_deref(),
    );

    let repl = PtyPane::spawn(
        PaneKind::Repl,
        "T REPL".into(),
        &app.config.repl.command,
        &app.config.repl.args,
        repo_path.as_deref(),
    );

    let vars = pane::vars::VarsPane::new("/tmp/atelier-vars.csv".into());

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let terminal_pty = PtyPane::spawn(
        PaneKind::Terminal,
        "Terminal".into(),
        &shell,
        &["-o".to_string(), "vi".to_string()],
        repo_path.as_deref(),
    );

    match editor {
        Ok(pane) => app.panes.push(Box::new(pane)),
        Err(e) => app.panes.push(Box::new(pane::ErrorPane {
            kind: PaneKind::Editor,
            name: app.config.editor.command.clone(),
            error: e.to_string(),
        })),
    }
    match repl {
        Ok(pane) => app.panes.push(Box::new(pane)),
        Err(e) => app.panes.push(Box::new(pane::ErrorPane {
            kind: PaneKind::Repl,
            name: "T REPL".into(),
            error: e.to_string(),
        })),
    }
    app.panes.push(Box::new(vars));
    match terminal_pty {
        Ok(pane) => app.panes.push(Box::new(pane)),
        Err(e) => app.panes.push(Box::new(pane::ErrorPane {
            kind: PaneKind::Terminal,
            name: "Terminal".into(),
            error: e.to_string(),
        })),
    }

    run(app)
}
