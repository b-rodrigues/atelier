# Atelier TUI

Atelier is a `tmux`-based Terminal User Interface (TUI) and interactive development environment for the **T programming language**. 

It provides an out-of-the-box IDE layout combining your editor, REPL, environment state, and terminal in a single unified workspace.

## Layout

Atelier splits your terminal window into four dedicated, responsive panes:

```
┌─────────────────────────────────┬─────────────────────────────────┐
│                                 │                                 │
│   nvim (Editor)                 │   T REPL                        │
│   Top-Left [Pane 0]             │   Top-Right [Pane 2]            │
│                                 │                                 │
├─────────────────────────────────┼─────────────────────────────────┤
│                                 │                                 │
│   Variables Viewer              │   File Browser / Terminal       │
│   Bottom-Left [Pane 1]          │   Bottom-Right [Pane 3]         │
│                                 │                                 │
└─────────────────────────────────┴─────────────────────────────────┘
```

- **Top-Left [Pane 0] (nvim)**: The editor pane pre-configured with interactive evaluation mappings.
- **Bottom-Left [Pane 1] (Variables)**: A live environment inspector displaying user-defined variables, their types, and formatted previews.
- **Top-Right [Pane 2] (T REPL)**: The active interactive T language shell.
- **Bottom-Right [Pane 3] (Terminal)**: A general-purpose shell initialized in the project directory for browsing files and executing shell commands.

## Key Mappings

### Code Evaluation
When writing code in the Neovim editor pane, you can send code directly to the REPL:
- **Normal Mode**: Press `<leader>e` (Space + e) or `Ctrl+Enter` to evaluate the current line.
- **Visual Mode**: Press `<leader>e` (Space + e) or `Ctrl+Enter` to evaluate the selected text block.

### Exiting the TUI
To cleanly shut down the entire Atelier session:
1. Select the **T REPL** pane (`Ctrl-b`, then arrow keys).
2. Press **`Ctrl-d`** once to exit the active T REPL session.
3. Press **`Ctrl-d`** a second time to exit the shell, which automatically kills the tmux session and closes all other panes.

## How to Run

Atelier is packaged and integrated directly into the `tlang` developer environment. 

1. Navigate to the `tlang` repository:
   ```bash
   cd /home/brodrigues/Documents/repos/tlang
   ```
2. Enter the development shell:
   ```bash
   nix develop
   ```
3. Run the launcher:
   ```bash
   atelier
   ```

## Configuration

You can configure the active editor inside `~/.config/ateliers/settings.toml`. 

Example:
```toml
editor = "nano" # Options: "nvim" (default) or "nano"
```

- **`nvim`**: Spawns Neovim with full code evaluation keybindings.
- **`nano`**: Spawns Nano. The TUI status bar dynamically hides Neovim evaluation helpers when Nano is active.

## Repository Structure

- `atelier`: The TUI launcher script (manages tmux window splits and process lifecycles).
- `atelier-watcher.sh`: Background watcher process piping code from Neovim to the T REPL and updating the environment inspector.
- `atelier-vars.py`: Python script formatting and displaying active variables with ANSI colors and Unicode borders.
- `atelier-init.lua`: Neovim configuration loading your default editor preferences and overlaying evaluation keymaps.

## License

Atelier is licensed under the EUPL v1.2. See the `LICENSE` file for details.
