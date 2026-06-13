# Atelier TUI

Atelier is a **Rust**-based Terminal User Interface (TUI) and interactive
development environment for the **T programming language**.

It provides an out-of-the-box IDE layout combining your editor, REPL,
environment state, terminal, LLM assistant, pipeline visualization, and
plot viewer in a single unified workspace.

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
│   Variables Viewer              │   Terminal / LLM                │
│   Bottom-Left                   │   Bottom-Right (tabs)           │
│                                 │                                 │
└─────────────────────────────────┴─────────────────────────────────┘
```

- **Editor** (top-left): Your editor (nvim by default) spawned in a PTY.
- **Variables** (bottom-left): Live CSV-based environment inspector.
- **T REPL** (top-right): Interactive T language shell in a PTY.
- **Terminal / LLM** (bottom-right, tabbed): General-purpose shell in a PTY,
  and an LLM assistant pane (opencode) with project context.

The four slots can be reassigned to any pane kind via the `positions` config
option. In addition to the defaults above, you can place the **diagram**
pipeline visualizer or the **plot** viewer in any slot.

## Key Mappings

| Mode       | Key               | Action                           |
|------------|-------------------|----------------------------------|
| Normal     | `Alt-Space`       | Enter navigation mode            |
| Normal     | `Ctrl-d`          | Quit (with confirmation)         |
| Normal     | `Ctrl-c`          | Send SIGINT to focused pane      |
| Normal     | `Ctrl-Tab`        | Next tab (in tabbed panes)       |
| Normal     | `Ctrl-Shift-Tab`  | Previous tab                     |
| Normal     | Keys              | Forwarded to focused PTY pane    |
| Nav        | `1`-`4`           | Focus pane 1–4                   |
| Nav        | `5` / `t`         | Focus transcript (maximized)     |
| Nav        | `l`               | Push context to LLM, focus LLM   |
| Nav        | `Tab` / `BackTab` | Switch tabs in focused pane      |
| Nav        | `f`               | Open file tree browser           |
| Nav        | `b`               | Open buffer list                 |
| Nav        | `p`               | Open project switcher            |
| Nav        | `e`               | Send clipboard to REPL           |
| Nav        | `?`               | Show keybinding help             |
| Nav        | `m`               | Toggle full maximize             |
| Nav        | `v`               | Toggle vertical maximize         |
| Nav        | `h`               | Toggle horizontal maximize       |
| Nav        | `=`               | Restore all pane sizes           |
| Nav        | `j` / `Down`      | Scroll transcript down           |
| Nav        | `k` / `Up`        | Scroll transcript up             |
| Nav        | `q`               | Quit (with confirmation)         |
| Nav        | `Esc`             | Return to normal mode            |
| FileTree   | `Up`/`Down`       | Navigate entries                 |
| FileTree   | `Enter`           | Open file / enter directory      |
| FileTree   | `Esc`/`q`         | Close file tree                  |

### Code Evaluation

Copy a line or selection in the editor (e.g. `yy` in nvim, or visual `y`),
then press `Alt-Space e` to send clipboard contents to the T REPL.

### Diagram and Plot Panes

When a **diagram** or **plot** pane is focused, the following keys are active:

| Pane     | Key                | Action                          |
|----------|--------------------|---------------------------------|
| Diagram  | `r`                | Re-render the diagram (`dot`)   |
| Plot     | `r`                | Rescan the plot directory       |
| Plot     | `Left` / `h`       | Previous plot                   |
| Plot     | `Right` / `l`      | Next plot                       |

## Pipeline Diagram

The **diagram** pane watches a DOT file and renders it as a PNG via
`dot -Tpng`. The diagram is displayed inline using ratatui-image with
halfblock rendering — no external image viewer needed.

Configure the DOT command and arguments (default: `dot -Tpng`):

```toml
[diagram]
dot_command = "dot"
dot_args = ["-Tpng"]
```

T writes pipeline DOT descriptions to `/tmp/atelier/<pipeline_name>.dot`.
The diagram pane picks up changes automatically (rate-limited to 300ms).

## Plot Viewer

The **plot** pane scans a directory for PNG files and lets you browse them
with the arrow keys. Plots are rendered inline with ratatui-image.

Configure the plot directory:

```toml
[plot]
directory = "/tmp/atelier-plots"
```

The directory is scanned every 500ms for new or removed files, so new
plots appear automatically.

## LLM Integration

The **LLM** pane spawns an opencode subprocess inside your project's
development shell (via `nix develop`). Context (editor lines, variables,
REPL state) is assembled into a markdown file and passed to opencode.

Trigger: press `l` in navigation mode to push context and focus the LLM pane.
Use `Ctrl-Tab` or `Tab` in navigation mode to switch between the Terminal
and LLM tabs in the bottom-right slot.

```toml
[llm]
command = "opencode"
args = []
context_path = "/tmp/atelier-llm-context.md"
context_mode = "file"
context_flag = ""
auto_context = true
```

With `auto_context = true`, context is pushed automatically at intervals.

## How to Run

```bash
nix run github:b-rodrigues/atelier
```

Or in a development shell:

```bash
nix develop
atelier
```

Pass a project path as the first argument to have all panes run inside
that project's `nix develop` shell:

```bash
atelier ~/projects/my-t-project
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
leader = "alt-space"

[diagram]
dot_command = "dot"
dot_args = ["-Tpng"]

[plot]
directory = "/tmp/atelier-plots"

[llm]
command = "opencode"
args = []
context_path = "/tmp/atelier-llm-context.md"
context_mode = "file"
context_flag = ""
auto_context = true

[layout]
positions = ["editor", "repl", "variables", "terminal"]
```

Swap panes by reordering the `positions` list. Available kinds:
`editor`, `repl`, `variables`, `terminal`, `diagram`, `plot`.

## Repository Structure

- `src/main.rs`          — Entry point, first-launch prompt, pane setup
- `src/config.rs`        — Config read/write (TOML)
- `src/app.rs`           — Application state
- `src/event.rs`         — Crossterm event loop
- `src/ui.rs`            — Ratatui layout and widgets
- `src/pane/mod.rs`      — Pane trait and enum
- `src/pane/pty.rs`      — PTY-backed pane (portable-pty + vt100)
- `src/pane/vars.rs`     — Variables pane (CSV watcher + Table)
- `src/pane/llm.rs`      — LLM assistant pane (opencode subprocess)
- `src/pane/diagram.rs`  — Pipeline diagram pane (DOT → PNG rendering)
- `src/pane/plot.rs`     — Plot viewer pane (PNG directory browser)
- `src/pane/tabs.rs`     — Tab container for pane grouping
- `src/pane/transcript.rs` — Scrollable event transcript
- `src/renderer.rs`      — Background renderer (spawns dot, decodes PNG)
- `src/context.rs`       — Context assembly for LLM

## License

Atelier is licensed under the EUPL v1.2. See the `LICENSE` file for details.
