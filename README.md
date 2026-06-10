# Atelier TUI

Atelier is a `tmux`-based Terminal User Interface (TUI) and interactive development environment for the **T programming language**. 

It provides an out-of-the-box IDE layout combining your editor, REPL, environment state, and terminal in a single unified workspace.

## Layout

Atelier splits your terminal window into four dedicated, responsive panes:

```
┌─────────────────────────────────┬─────────────────────────────────┐
│                                 │                                 │
│   nvim (Editor)                 │   T REPL                        │
│   Top-Left [Pane 0]             │   Top-Right [Pane 1]            │
│                                 │                                 │
├─────────────────────────────────┼─────────────────────────────────┤
│                                 │                                 │
│   Variables Viewer              │   File Browser / Terminal       │
│   Bottom-Left [Pane 2]          │   Bottom-Right [Pane 3]         │
│                                 │                                 │
└─────────────────────────────────┴─────────────────────────────────┘
```

- **Top-Left (nvim)**: The editor pane pre-configured with interactive evaluation mappings.
- **Top-Right (T REPL)**: The active interactive T language shell.
- **Bottom-Left (Variables)**: A live environment inspector displaying user-defined variables, their types, and formatted previews.
- **Bottom-Right (Terminal)**: A general-purpose shell initialized in the project directory for browsing files and executing shell commands.

## Key Mappings

When writing code in the Neovim editor pane, you can send code directly to the REPL:
- **Normal Mode**: Press `<leader>e` (Space + e) or `Ctrl+Enter` to evaluate the current line.
- **Visual Mode**: Press `<leader>e` (Space + e) or `Ctrl+Enter` to evaluate the selected text block.

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

## Repository Structure

- `atelier`: The TUI launcher script (manages tmux window splits and process lifecycles).
- `atelier-watcher.sh`: Background watcher process piping code from Neovim to the T REPL and updating the environment inspector.
- `atelier-vars.py`: Python script formatting and displaying active variables with ANSI colors and Unicode borders.
- `atelier-init.lua`: Neovim configuration loading your default editor preferences and overlaying evaluation keymaps.

## License

Atelier is licensed under the EUPL v1.2. See the `LICENSE` file for details.
