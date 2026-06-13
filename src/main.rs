mod app;
mod config;
mod context;
mod event;
mod pane;
mod renderer;
mod session;
mod ui;

use crate::app::App;
use crate::config::Config;
use crate::config::RecentProjects;
use crate::event::Action;
use crate::pane::diagram::DiagramPane;
use crate::pane::llm::LlmPane;
use crate::pane::plot::PlotPane;
use crate::pane::pty::PtyPane;
use crate::pane::tabs::TabContainer;
use crate::pane::transcript::TranscriptPane;
use crate::pane::Pane;
use crate::pane::PaneKind;
use anyhow::Result;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Write};

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

fn first_launch_prompt() -> Result<Config> {
    let nvim_ok = command_exists("nvim");
    let vim_ok = command_exists("vim");
    let nano_ok = command_exists("nano");
    let vi_ok = command_exists("vi");

    let default_choice = if nvim_ok {
        "1"
    } else if nano_ok {
        "3"
    } else if vim_ok {
        "2"
    } else if vi_ok {
        "4"
    } else {
        "1"
    };

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
    println!(
        "    [1] nvim    {}",
        if nvim_ok {
            "(recommended)"
        } else {
            "(not found)"
        }
    );
    println!("    [2] vim     {}", if vim_ok { "" } else { "(not found)" });
    println!("    [3] nano    {}", if nano_ok { "" } else { "(not found)" });
    println!("    [4] vi      {}", if vi_ok { "" } else { "(not found)" });
    print!("  > ");
    stdout.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let mut choice = input.trim();
    if choice.is_empty() {
        choice = default_choice;
    }

    let (editor, args) = match choice {
        "1" | "nvim" => ("nvim".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
        "2" | "vim" => ("vim".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
        "3" | "nano" => ("nano".to_string(), vec![]),
        "4" | "vi" => ("vi".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
        _ => match default_choice {
            "2" => ("vim".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
            "3" => ("nano".to_string(), vec![]),
            "4" => ("vi".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
            _ => ("nvim".to_string(), vec!["--cmd".to_string(), "set shortmess+=I".to_string()]),
        },
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    Ok(config)
}

fn run(mut app: App) -> Result<Option<String>> {
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
            Action::None => {
                if app.config.llm.auto_context {
                    app.refresh_llm_context();
                }
            }
        }
    }

    let new_path = app.should_relaunch.clone();

    // Save artifacts (transcript markdown, etc.)
    for pane in app.panes.iter_mut() {
        pane.save_artifact();
    }

    app.kill_all();
    cleanup_temp_files();

    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(new_path)
}

fn cleanup_temp_files() {
    let _ = std::fs::remove_file("/tmp/atelier-vars.csv");
    let _ = std::fs::remove_file("/tmp/atelier-cmd");
    let _ = std::fs::remove_file("/tmp/atelier-buffers.txt");
    let _ = std::fs::remove_file("/tmp/atelier-llm-context.md");
    let _ = std::fs::remove_dir_all("/tmp/atelier");
    let _ = std::fs::remove_dir_all("/tmp/atelier-plots");
}

fn main() -> Result<()> {
    cleanup_temp_files();

    let config = match Config::load()? {
        Some(c) => c,
        None => first_launch_prompt()?,
    };

    let repo_path = std::env::args().nth(1);

    if let Some(ref path) = repo_path {
        RecentProjects::add_project(path);
    }

    let mut app = App::new(config, repo_path.clone());

    // Restore session on launch
    app.restore_session();

    let mut editor_args = app.config.editor.args.clone();
    let cmd_name = std::path::Path::new(&app.config.editor.command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&app.config.editor.command);
    if cmd_name == "nvim" || cmd_name == "vim" || cmd_name == "vi" {
        if editor_args.is_empty() {
            editor_args = vec!["--cmd".to_string(), "set shortmess+=I".to_string()];
        }
        editor_args.push("--cmd".to_string());
        editor_args.push("autocmd ColorScheme * highlight Normal ctermbg=NONE guibg=NONE | highlight NormalNC ctermbg=NONE guibg=NONE | highlight NonText ctermbg=NONE guibg=NONE | highlight SignColumn ctermbg=NONE guibg=NONE | highlight LineNr ctermbg=NONE guibg=NONE | highlight EndOfBuffer ctermbg=NONE guibg=NONE".to_string());
        editor_args.push("--cmd".to_string());
        editor_args.push("highlight Normal ctermbg=NONE guibg=NONE | highlight NormalNC ctermbg=NONE guibg=NONE | highlight NonText ctermbg=NONE guibg=NONE | highlight SignColumn ctermbg=NONE guibg=NONE | highlight LineNr ctermbg=NONE guibg=NONE | highlight EndOfBuffer ctermbg=NONE guibg=NONE".to_string());
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

    let default_path = repo_path.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });
    let llm = LlmPane::new(app.config.llm.clone(), default_path);

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

    let bottom_right: Box<dyn Pane> = match terminal_pty {
        Ok(pane) => {
            let children: Vec<Box<dyn Pane>> = vec![Box::new(pane), Box::new(llm)];
            Box::new(TabContainer::new(children))
        }
        Err(e) => {
            let children: Vec<Box<dyn Pane>> = vec![
                Box::new(pane::ErrorPane {
                    kind: PaneKind::Terminal,
                    name: "Terminal".into(),
                    error: e.to_string(),
                }),
                Box::new(llm),
            ];
            Box::new(TabContainer::new(children))
        }
    };
    app.panes.push(bottom_right);

    let diagram = DiagramPane::new(
        app.config.diagram.watch_file.clone().into(),
        app.config.diagram.command.clone(),
        app.config.diagram.args.clone(),
    );
    app.panes.push(Box::new(diagram));

    let plots = PlotPane::new(
        app.config.plots.watch_dir.clone().into(),
    );
    app.panes.push(Box::new(plots));

    let transcript = TranscriptPane::new();
    app.panes.push(Box::new(transcript));

    let new_path = run(app)?;

    if let Some(path) = new_path {
        let exe = std::env::current_exe().ok();
        if let Some(exe) = exe {
            let _ = std::process::Command::new(exe).arg(&path).spawn();
        }
    }

    Ok(())
}
