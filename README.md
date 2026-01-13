# xorcist

A TUI client for [jj (Jujutsu VCS)](https://github.com/martinvonz/jj).

## Features

- **Log View** - Browse commit history with vim-like navigation
- **Native Graph Display** - jj's graph visualization with full ANSI color support
- **Detail View** - View commit metadata and diff summary
- **Conventional Commits** - Automatic emoji formatting (`feat:` → `✨`, `fix:` → `🩹`, etc.)
- **Incremental Loading** - Load history on demand (default: 500 entries, auto-loads more as needed)
- **Confirmation Dialogs** - Safe destructive operations (abandon, squash, push, undo)
- **Bookmark Management** - Set bookmarks on any revision
- **Git Integration** - Fetch and push with jj's git backend
- **Colocated Repository Support** - Works with `.jj` + `.git` repositories

## Requirements

- **Rust** 1.85+ (Edition 2024)
- **jj** 0.20+ (with `shortest()` template support)

## Installation

### From crates.io (recommended)

```bash
cargo install xorcist
```

### From source (GitHub)

```bash
cargo install --git https://github.com/fpdy/xorcist
```

### From local source

```bash
git clone https://github.com/fpdy/xorcist.git
cd xorcist
cargo install --path .
```

## Usage

```bash
# Navigate to a jj repository and run
cd /path/to/jj-repo
xor

# Options
xor -n 100      # Load only 100 entries initially (default: 500)
xor --all       # Load entire history at startup (may be slow)
```

xorcist automatically detects the jj repository root by walking up the directory tree.
When scrolling near the end of the log, additional entries are loaded automatically.

## Key Bindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `Home` | Go to first entry |
| `G` / `End` | Go to last entry |
| `Ctrl+d` / `PageDown` | Scroll down (10 lines) |
| `Ctrl+u` / `PageUp` | Scroll up (10 lines) |

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

The log view shows jj's native graph visualization with full color support:

- `@` - Working copy
- `◆` - Immutable commit
- `○` - Regular commit
- Graph lines (`│`, `├─╮`, `├─╯`, etc.) - Branch/merge visualization
- `[bookmark]` - Bookmarks shown in cyan
- Conventional commit messages are displayed with emoji prefixes

## License

MIT License - see [LICENSE](LICENSE) for details.
