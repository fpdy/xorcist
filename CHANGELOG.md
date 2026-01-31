# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-01-31

### Added

- Rebase command (`r` key) - rebase revisions to any destination with `jj rebase -d`

### Fixed

- Diff view file selection and display issues

### Documentation

- Added Diff View and Detail View key bindings to README
- Added rebase command (`r`) to key bindings documentation

## [0.1.1] - 2026-01-28

### Added

- Diff View - browse changed files and view file-level diffs with syntax highlighting
- Horizontal scrolling in diff view (`←`/`→` keys)

## [0.1.0] - 2026-01-14

### Added

- Initial release
- Log View with vim-like navigation
- Native jj graph visualization with full ANSI color support
- Detail View for commit metadata and diff summary
- Conventional Commits emoji formatting
- Incremental loading (default: 500 entries)
- Confirmation dialogs for destructive operations
- Bookmark management
- Git integration (fetch/push)
- Colocated repository support
