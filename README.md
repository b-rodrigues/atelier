# Atelier TUI

Atelier is a **Rust**-based Terminal User Interface (TUI) and interactive
development environment for the **T programming language**.

It provides an out-of-the-box IDE layout combining your editor, REPL,
environment state, and terminal in a single unified workspace.

## Layout

Atelier splits your terminal window into four dedicated, responsive panes:

```
┌─────────────────────────────────┬─────────────────────────────────┐
│                                 │                                 │
│   Editor (nvim)                 │   T REPL                        │
│   Top-Left                      │   Top-Right                     │
│                                 │                                 │
├─────────────────────────────────┼─────────────────────────────────┤
│                                 │                                 │
│   Variables Viewer              │   Terminal                      │
│   Bottom-Left                   │   Bottom-Right                  │
│                                 │                                 │
└─────────────────────────────────┴─────────────────────────────────┘
```

- **Editor** (top-left): Your editor (nvim by default) spawned in a PTY.
- **Variables** (bottom-left): Live CSV-based environment inspector.
- **T REPL** (top-right): Interactive T language shell in a PTY.
- **Terminal** (bottom-right): General-purpose shell in a PTY.

## Key Mappings

| Mode    | Key           | Action                         |
|---------|---------------|--------------------------------|
| Normal  | `Ctrl-Space`  | Enter navigation mode          |
| Normal  | `Ctrl-d`      | Quit (with confirmation)       |
| Normal  | Keys          | Forwarded to focused PTY pane  |
| Nav     | `1`-`4`       | Focus pane 1–4                 |
| Nav     | `f`           | Open file tree browser         |
| Nav     | `e`           | Send clipboard to REPL         |
| Nav     | `?`           | Show keybinding help           |
| Nav     | `Esc`         | Return to normal mode          |
| Nav     | `q`           | Quit (with confirmation)       |
| FileTree| `Up`/`Down`   | Navigate entries               |
| FileTree| `Enter`       | Open file / enter directory    |
| FileTree| `Esc`/`q`     | Close file tree                |

### Code Evaluation

Copy a line or selection in the editor (e.g. `yy` in nvim, or visual `y`),
then press `Ctrl-Space e` to send clipboard contents to the T REPL.

## How to Run

```bash
nix run github:b-rodrigues/atelier
```

Or in a development shell:

```bash
nix develop
atelier
```

On first launch, Atelier prompts you to choose an editor (nvim/vim/nano/vi)
and creates `~/.config/atelier/config.toml`.

## Configuration

Configuration is stored in `~/.config/atelier/config.toml`:

```toml
[editor]
command = "nvim"
args = []

[repl]
command = "t"
args = ["repl"]

[keybindings]
leader = "ctrl-space"
```

## Repository Structure

- `src/main.rs`          — Entry point, first-launch prompt, pane setup
- `src/config.rs`        — Config read/write (TOML)
- `src/app.rs`           — Application state
- `src/event.rs`         — Crossterm event loop
- `src/ui.rs`            — Ratatui layout and widgets
- `src/pane/mod.rs`      — Pane trait and enum
- `src/pane/pty.rs`      — PTY-backed pane (portable-pty + vt100)
- `src/pane/vars.rs`     — Variables pane (CSV watcher + Table)

## License

Atelier is licensed under the EUPL v1.2. See the `LICENSE` file for details.
