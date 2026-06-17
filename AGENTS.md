# Atelier — Agent Guide

## Architecture

Atelier is a Rust TUI IDE for the T programming language. It uses ratatui for
rendering, crossterm for terminal interaction, and portable-pty for subprocess
management. Panes are `Box<dyn Pane>` stored in `App::panes`. The `Pane` trait
in `pane/mod.rs` defines the interface.

## Best Practices & Safety

### Error handling — use anyhow::Result, never panic/unwrap/expect

Every fallible function returns `anyhow::Result<T>`. Propagate errors with `?`.
Never call `.unwrap()`, `.expect()`, or `panic!()`. If an error path is truly
unreachable, comment why and still use `.context("...")?` rather than unwrap.

```rust
// BAD:
let content = std::fs::read_to_string(path).unwrap();

// GOOD:
let content = std::fs::read_to_string(path)
    .with_context(|| format!("Failed to read {}", path.display()))?;
```

### Thread safety — Send + Sync, channels, no shared mutable state

- Background work uses `std::sync::mpsc::channel()` — never `Arc<Mutex<T>>`
  for shared state.
- All pane state is mutated on the main thread (in `event.rs` or the
  terminal.draw closure). Background threads only send data back via channels.
- Verify new types are `Send + Sync` when passed across threads.

### Subprocess safety — validate all inputs

`PtyPane::spawn()` runs user-configured commands. Never interpolate user input
or file paths into shell commands — all argument injection must be impossible.
Config values for `command` and `args` come from the user's own
`~/.config/atelier/config.toml`, so trust is assumed, but any new subprocess
spawning must follow the same pattern.

### Input handling — defensive, rate-limited, bounded

- `write_input()` sends raw bytes to PTY processes. Always validate/enforce
  bounds before writing.
- File watchers (`plot.rs`, `diagram.rs`) use `Instant` rate limits (300-500ms)
  to prevent filesystem thrashing.
- All `read_dir()` calls filter extensions explicitly — never blindly load
  files from watched directories.

### Resource cleanup — RAII, kill on drop

- `Pane::kill()` must clean up subprocesses and temp files.
- Temp files in `/tmp/atelier-*` are cleaned in `cleanup_temp_files()`. New
  temp files must be registered there.
- The `run()` function calls `app.kill_all()` and `cleanup_temp_files()` on
  exit.

### No unsafe code

This project uses zero `unsafe` blocks. Any new code must avoid unsafe Rust
entirely.

## Testing

No test infrastructure exists yet. When adding tests:
- Unit tests go in a `#[cfg(test)] mod tests { }` block at the bottom of the
  source file.
- Integration tests go in `tests/`.
- Use `assert_eq!`, `assert!`, and standard `#[test]` — no external test
  crate is required yet.

## Build & Run

- Build: `nix develop -c cargo build` (or just `cargo build` in the nix shell)
- No lint/CI scripts yet

## Project Structure

```
src/
├── main.rs          — Entry, CLI arg, pane wiring, image protocol detection
├── config.rs        — ~/.config/atelier/config.toml (serde + toml)
├── app.rs           — App state, input modes, session save/restore
├── event.rs         — Crossterm event loop, key dispatch
├── ui.rs            — Ratatui layout + rendering
├── context.rs       — LLM context assembly
├── session.rs       — Session persistence
├── renderer.rs      — Background dot -Tpng renderer
└── pane/
    ├── mod.rs       — Pane trait + PaneKind enum
    ├── pty.rs       — PTY subprocess pane
    ├── vars.rs      — Variables CSV watcher
    ├── llm.rs       — LLM subprocess pane
    ├── diagram.rs   — DOT → PNG pipeline diagram
    ├── plot.rs      — PNG plot viewer
    ├── tabs.rs      — Tab container (wraps children, renders tab bar)
    └── transcript.rs— Event transcript
```

## Conventions

- Error handling: `anyhow::Result<T>`. Context enriches errors.
- Concurrency: `std::sync::mpsc`, never Arc/Mutex for shared state.
- Rate limiting: `Instant` checks at 300-500ms intervals for file watches.
- Imports: external crates first, blank line, then `crate::` imports.
- Pane identification: `PaneKind` enum comparisons only — never downcast.
- Config: serde + Default + `#[serde(default)]`. TOML key = Rust field name
  (e.g. `[plots]` not `[plot]`).
- Image rendering: `ratatui-image` `Picker` passed via
  `Pane::set_image_backend()`. Both font_size and protocol_type are
  transferred.
- No tests, no unsafe blocks, no async runtime.

## Dependencies (key)

| Crate | Version | Purpose |
|---|---|---|
| ratatui | 0.30 | TUI framework |
| crossterm | 0.28 | Terminal backend |
| ratatui-image | 11 | Image rendering |
| image | 0.25 | PNG decode |
| portable-pty | 0.8 | PTY spawning |
| vt100 | 0.15 | PTY output parsing |
| serde + toml | — | Config |

## Gotchas

- `PlotPane` is inside a `TabContainer` (wrapped with vars pane). Never assume
  direct access.
- No editor buffer tracking — reads `/tmp/atelier-buffers.txt` from an nvim
  autocmd.
- `from_query_stdio()` must be called after `EnterAlternateScreen` but before
  the event loop. Currently in `run()` in main.rs.
- Config field name `plots` maps to TOML section `[plots]`, not `[plot]`.
- The T REPL gets `TLANG_REPO_ROOT` and `REPO_DIR` env vars set.
