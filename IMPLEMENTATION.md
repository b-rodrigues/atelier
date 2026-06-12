# Atelier — Ratatui Rewrite Implementation Plan

> **Status**: Design / Pre-implementation  
> **Decision**: Replace the current `bash + tmux` launcher with a native Rust binary built on [Ratatui](https://ratatui.rs/) + PTY widgets.

---

## 1. Motivation

The current architecture is a collection of shell scripts that orchestrate `tmux` panes. This creates fundamental limitations:

| Problem | Root cause |
|---|---|
| Global shortcuts (`Space q q`) only work inside Neovim | Tmux never intercepts Space; it goes straight to the focused program |
| Vi-mode "everywhere" is impossible | Each pane is an independent PTY — no shared modal state |
| `Ctrl-d` twice to quit is fragile | Depends on shell exit propagation through tmux |
| Variable viewer requires a running Python process | No shared event loop, polling a CSV file |
| Leader key unavailable in the REPL or terminal panes | Linenoise / bash have no concept of a tmux leader |

The fix is to own the event loop — one binary intercepts all keypresses, maintains a global modal state, and dispatches to sub-processes via PTY.

---

## 2. Target Architecture

```
┌─────────────────────────────────┬─────────────────────────────────┐
│  Editor  (PTY widget)           │  T REPL  (PTY widget)           │
│  nvim / nano / vim              │  `t repl` or `dune exec …`      │
│  Vim mode: natural (it's nvim)  │  Linenoise still works fine     │
├─────────────────────────────────┼─────────────────────────────────┤
│  Variables  (native widget)     │  Terminal  (PTY widget)         │
│  Ratatui Table, no Python       │  $SHELL, general-purpose        │
│  Updated via /tmp/atelier-vars  │                                 │
└─────────────────────────────────┴─────────────────────────────────┘
         ▲ All input flows through the Atelier event loop first
```

### Key properties

- **Single Rust binary** (`atelier`) — replaces the current bash script entirely.
- **Global modal layer** — `Ctrl-Space` (configurable) enters Navigation Mode. In Navigation Mode all keys are consumed by Atelier, not forwarded to PTYs.
- **Editor is just a PTY** — no Neovim RPC. Spawn `nvim`/`nano`/`vim` inside a PTY widget. Their own keybindings work unchanged.
- **Variables pane is a native widget** — reads `/tmp/atelier-vars.csv`, no Python process. Automatically refreshed by `inotify`/polling in the event loop.
- **Terminal pane is a PTY widget** — generic shell, replaces the current bottom-right tmux pane.
- **Config stored** in `~/.config/atelier/config.toml` — editor choice and other preferences persisted across launches.

---

## 3. First-Launch Flow

```
$ atelier [<repo-path>]

No config found at ~/.config/atelier/config.toml

Choose your editor:
  [1] nvim    (recommended)
  [2] vim
  [3] nano
  [4] vi

> _
```

- Prompt rendered in the raw terminal (no TUI yet).
- Selection written to `~/.config/atelier/config.toml`.
- TUI launches immediately after.
- On subsequent launches the prompt is skipped.

Config file format:

```toml
[editor]
command = "nvim"          # one of: nvim, vim, nano, vi, or an absolute path
args    = []              # extra args forwarded verbatim

[repl]
command = "t"
args    = ["repl"]

[keybindings]
leader  = "ctrl-space"    # key that enters Navigation Mode
```

---

## 4. Rust Crate Structure

```
atelier/
├── src/
│   ├── main.rs           # entry point: config check → first-launch prompt → run()
│   ├── app.rs            # App state: focus, panes, modal mode
│   ├── config.rs         # serde: read/write ~/.config/atelier/config.toml
│   ├── event.rs          # crossterm event polling + leader-key state machine
│   ├── pane/
│   │   ├── mod.rs        # Pane trait
│   │   ├── pty.rs        # PTY-backed pane (editor, REPL, terminal)
│   │   └── vars.rs       # native Variables table pane
│   └── ui.rs             # Ratatui layout + render pass
├── Cargo.toml
├── flake.nix             # (updated) adds the Rust toolchain
└── IMPLEMENTATION.md     # this file
```

---

## 5. Dependencies

```toml
[dependencies]
ratatui       = "0.29"          # TUI framework
crossterm     = "0.28"          # raw terminal events + control
portable-pty  = "0.8"           # cross-platform PTY spawn
vt100         = "0.15"          # VT100/ANSI parser for PTY output → Ratatui cells
serde         = { version = "1", features = ["derive"] }
toml          = "0.8"           # config file format
notify        = "6"             # inotify watcher for /tmp/atelier-vars.csv
anyhow        = "1"             # ergonomic error handling
```

> `portable-pty` handles PTY creation and process spawning in a cross-platform way.  
> `vt100` parses the raw byte stream from the PTY into a screen-buffer of styled cells that Ratatui can render.

---

## 6. Event Loop Design

```
loop {
    poll crossterm events (keyboard, resize, …)

    if event is leader key (Ctrl-Space):
        enter NavigationMode
        continue

    if in NavigationMode:
        dispatch to global_action(key)   // pane switching, quit, etc.
        exit NavigationMode
        continue

    // Normal mode: forward raw bytes to the focused PTY
    focused_pane.write_input(raw_bytes)

    // Re-render every frame (or on PTY output / inotify event)
    terminal.draw(|f| ui::render(f, &app))
}
```

### Navigation Mode actions (`Ctrl-Space` → key)

| Key | Action |
|---|---|
| `1` | Focus Editor pane |
| `2` | Focus REPL pane |
| `3` | Focus Variables pane |
| `4` | Focus Terminal pane |
| `q q` | Save all + quit (sends `:wqa` to nvim PTY, then kills all PTYs) |
| `f` | Toggle file tree overlay (built-in Ratatui widget) |
| `b` | Show buffer list overlay |
| `e` | Send current nvim line to REPL |
| `?` | Show keybinding help overlay |

---

## 7. PTY Pane

Each PTY pane owns:

```rust
struct PtyPane {
    pty:     Box<dyn MasterPty>,    // portable-pty handle
    child:   Box<dyn Child>,        // the spawned process
    parser:  vt100::Parser,         // ANSI parser → screen buffer
    size:    (u16, u16),            // current (cols, rows)
}
```

On each Ratatui frame, `PtyPane::render()`:
1. Drains all available bytes from the PTY fd into `parser`.
2. Iterates `parser.screen().cells()` and draws styled `Span`s into a Ratatui `Paragraph`.
3. Positions the cursor using `parser.screen().cursor_position()`.

On resize, `PtyPane::resize(cols, rows)` calls `pty.resize()` and sends `SIGWINCH` to the child.

---

## 8. Variables Pane

No external process. The pane:

1. Holds a `notify::Watcher` on `/tmp/atelier-vars.csv`.
2. On change event, re-reads the CSV and updates an in-memory `Vec<(String, String, String)>` (name, type, value preview).
3. Renders as a Ratatui `Table` with scroll.
4. `j`/`k` scroll when this pane is focused (the pane handles them directly; Navigation Mode is not required).

---

## 9. File Tree Overlay

Triggered by `Ctrl-Space f`. Renders a floating Ratatui widget over the layout:

```
┌── File Browser ────────────────────┐
│  src/                              │
│  ▶ repl.ml                         │
│  ▶ ast.ml                          │
│  tests/                            │
│  ▶ test_roundtrip.t                │
└────────────────────────────────────┘
  j/k: move   Enter: open in editor   Esc: close
```

- `Enter` on a file sends `:e <path><CR>` to the Editor PTY (works for nvim and vim; for nano it restarts the PTY with the new file path).
- Pure Ratatui — no external file manager.

---

## 10. Build & Nix Integration

`Cargo.toml` will live at `atelier/Cargo.toml`. The `flake.nix` is updated to:

1. Add `rustToolchain` input (fenix or nixpkgs rustc).
2. Add a `packages.atelier` derivation (`buildRustPackage`).
3. Keep the existing `devShell` so `nix develop` still works for the T REPL.

The installed `atelier` binary replaces the existing bash script. The Python watcher, `atelier-watcher.sh`, and `atelier-vars.py` are retired once Phase 4 is complete.

---

## 11. Files Retired by This Rewrite

| File | Replacement |
|---|---|
| `atelier` (bash script) | `src/main.rs` entry point |
| `atelier-init.lua` | Nvim is launched plain; user's own `init.lua` is respected; Atelier keybindings no longer live here |
| `atelier-vars.py` | `src/pane/vars.rs` (native Ratatui widget) |
| `atelier-watcher.sh` | Handled inside the Rust event loop |

---

## 12. Implementation Phases

### Phase 1 — Scaffold (week 1)
- [ ] `cargo init` in `atelier/`
- [ ] `config.rs`: read/write `config.toml`, first-launch prompt
- [ ] Basic Ratatui layout: 4 panes, hardcoded placeholders
- [ ] `event.rs`: leader-key state machine, pane switching

### Phase 2 — PTY Panes (week 2)
- [ ] `pty.rs`: spawn editor and REPL in PTY, pipe output through `vt100`
- [ ] Render PTY screen buffer into Ratatui cells
- [ ] Cursor forwarding and resize handling (`SIGWINCH`)

### Phase 3 — Native Widgets (week 2–3)
- [ ] `vars.rs`: CSV watcher + Ratatui Table
- [ ] File tree overlay
- [ ] Buffer list overlay

### Phase 4 — Polish (week 3)
- [ ] Keybinding help overlay (`Ctrl-Space ?`)
- [ ] Nix flake update (`buildRustPackage`)
- [ ] Retire old bash/Python files
- [ ] README update

---

## 13. Open Questions

1. **Buffer list for non-nvim editors**: For `nano`, there is no buffer concept. The `b` shortcut could be hidden or show recently opened files instead.
2. **Send-to-REPL for non-nvim editors**: `Ctrl-Space e` currently reads the current line from nvim. For nano this is not trivial — one option is to rely on the clipboard (`wl-clipboard`/`xclip`) as a middle layer.
3. **Atelier as a `t` subcommand**: Could expose `t tui` instead of a separate binary. Deferred to after Phase 4.
4. **nvim socket vs PTY-only**: Connecting to nvim's `--listen` socket enables richer integration (true cursor position, buffer contents for send-to-REPL) but adds complexity. Start PTY-only; add socket later if needed.
