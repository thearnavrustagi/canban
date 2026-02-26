# canban

A terminal Kanban board application written in Rust with vim keybindings.

## Features

- **Vim-native navigation** -- `h/j/k/l` to move between columns and tasks, modal editing for text input
- **4 columns** -- Ready, Doing, Done, Archived with colored borders
- **Task metadata** -- title, description, tags, due dates, time tracking in Doing
- **Multiple boards** -- create and switch between boards with `b`
- **Search/filter** -- press `/` to filter tasks by title, tag, or description
- **Command mode** -- `:w` to save, `:q` to quit, `:wq` for both
- **CSV import/export** -- `canban export -o board.csv` and `canban import -i board.csv`
- **Auto-save** -- changes are persisted automatically on each tick
- **XDG-compliant storage** -- config in `~/.config/canban/`, data in `~/.local/share/canban/` (macOS: `~/Library/Application Support/canban/`)

## Installation

```bash
# From source
git clone <repo-url> && cd canban
cargo install --path .

# Or directly
cargo install canban
```

## Usage

```bash
# Launch the TUI (creates a default board on first run)
canban

# List all boards
canban boards

# Export active board to CSV
canban export -o board.csv

# Import a board from CSV
canban import -i board.csv
```

## Keybindings

### Normal Mode

| Key | Action |
|---|---|
| `h` / `l` | Move between columns |
| `j` / `k` | Move between tasks |
| `g` / `G` | Jump to first / last task |
| `1`-`4` | Jump to column by number |
| `Tab` | Cycle columns |
| `n` / `a` | New task |
| `Enter` / `e` | Edit task |
| `r` | Rename task |
| `d` | Delete task |
| `Space` / `m` | Advance task to next column |
| `M` | Move task to previous column |
| `t` | Set tag |
| `D` | Set due date |
| `/` | Search |
| `?` | Help overlay |
| `:` | Command mode |
| `b` | Switch board |
| `q` | Quit |

### Insert / Dialog Mode

| Key | Action |
|---|---|
| `Tab` / `Shift-Tab` | Next / previous field |
| `Enter` | Confirm |
| `Esc` | Cancel |
| Arrow keys | Move cursor |

### Command Mode

| Command | Action |
|---|---|
| `:q` / `:quit` | Quit |
| `:w` / `:save` | Save |
| `:wq` | Save and quit |

## Data Storage

- **Config**: `$XDG_CONFIG_HOME/canban/config.toml`
- **Boards**: `$XDG_DATA_HOME/canban/boards/<name>/tasks.json`

Each board is stored as a JSON file. CSV is supported as an import/export format.

## License

MIT
