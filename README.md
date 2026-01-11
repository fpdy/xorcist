# xorcist

A TUI client for [jj (Jujutsu VCS)](https://github.com/martinvonz/jj).

## Features

- **Log View** - Browse commit history with vim-like navigation
- **Detail View** - View commit metadata and diff summary
- **Confirmation Dialogs** - Safe destructive operations (abandon, squash, push, undo)
- **Bookmark Management** - Set bookmarks on any revision
- **Git Integration** - Fetch and push with jj's git backend
- **Colocated Repository Support** - Works with `.jj` + `.git` repositories

## Requirements

- **Rust** 1.85+ (Edition 2024)
- **jj** 0.20+ (with `shortest()` template support)

## Installation

```bash
git clone https://github.com/user/xorcist.git
cd xorcist
cargo build --release
cp target/release/xor ~/.local/bin/
```

## Usage

```bash
# Navigate to a jj repository and run
cd /path/to/jj-repo
xor
```

xorcist automatically detects the jj repository root by walking up the directory tree.

## Key Bindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `Home` | Go to first entry |
| `G` / `End` | Go to last entry |
| `Ctrl+d` / `PageDown` | Page down |
| `Ctrl+u` / `PageUp` | Page up |

### Actions

| Key | Action |
|-----|--------|
| `Enter` | Open detail view |
| `q` / `Esc` | Quit / Close view |
| `?` | Toggle help |

### jj Commands

| Key | Command | Confirmation |
|-----|---------|--------------|
| `n` | `jj new` | No |
| `N` | `jj new -m` (with message input) | No |
| `e` | `jj edit` | No |
| `d` | `jj describe -m` (message input) | No |
| `b` | `jj bookmark set` (name input) | No |
| `a` | `jj abandon` | Yes |
| `s` | `jj squash` | Yes |
| `f` | `jj git fetch` | No |
| `p` | `jj git push` | Yes |
| `u` | `jj undo` | Yes |

## Display

- `@` - Working copy
- `◆` - Immutable commit
- `○` - Regular commit
- *Italic* - Empty commit (no changes)
- `[bookmark]` - Bookmarks shown in cyan

## License

MIT License - see [LICENSE](LICENSE) for details.
