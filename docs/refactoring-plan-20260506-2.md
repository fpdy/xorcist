# Refactoring Plan (2026-05-06)

Task: `refactor-plan-20260506-2`

## Investigation scope

Read-only worker scans covered:

- Repository architecture: `Cargo.toml`, `README.md`, `AGENTS.md`, `src/app`, `src/ui`, `src/jj`, `src/keys.rs`.
- jj parser/test contracts: `src/jj/graph_log.rs`, `src/jj/show.rs`, `src/jj/runner.rs`, `src/app/commands.rs`, `src/app/tests.rs`.
- Current `jj` working copy and recent history, especially modified files: `AGENTS.pizor.md`, `README.md`, `src/jj/show.rs`, `src/ui/detail.rs`, `src/ui/diff.rs`, `src/ui/log.rs`, `src/ui/mod.rs`, `src/ui/overlays.rs`.

Current working copy checkpoint observed by workers: change id `qozmlzxx`. Commit ids in this document are observational and may become stale as the working copy is updated; before implementation, re-check the current state with `jj status` and `jj diff --summary`.

## Important current-change constraints

Do **not** re-plan these as new work unless the current diff is abandoned or substantially changes:

- `DiffStatus` display de-duplication into `src/ui/mod.rs::diff_status_presentation` is already present in the working copy.
- README/help/status wording alignment is already being edited.
- `src/jj/show.rs` already adds copied/renamed diff summary parser coverage.
- `AGENTS.pizor.md` orchestration handoff changes are unrelated to implementation refactors and should be avoided by implementation batches.

## Prioritized refactor plan

### P1 — Centralize command/key/help metadata

**Problem**

Command behavior and documentation are spread across `src/keys.rs`, `src/app/commands.rs`, `src/app/input.rs`, `src/ui/overlays.rs`, `README.md`, and tests. Adding or changing a jj command can require synchronized updates to runner calls, app methods, key handling, modal/input state, help/status UI, and documentation.

**Proposed refactor**

Introduce a small command/action metadata layer, for example `CommandSpec` or `ActionSpec`, that records stable action id, key bindings, display name, help text, and modal/input behavior where practical. Use it to drive help overlay rows and tests for key/help consistency. Keep actual command execution in existing app command methods initially.

The metadata must model view/state scope explicitly because the same key can mean different things in different layers. Include fields such as:

- `view_scope`: log, detail, diff, or global.
- `priority_layer`: input, modal, help, or active view.
- `key_bindings` and display/help labels.
- `requires_input`.
- `requires_confirmation`.
- `is_jj_mutating`.

**Risk**

- Key dispatch priority can regress, especially input mode → modal → active view.
- Over-generalizing command metadata may obscure view-specific behavior.
- README generation may be too much for one batch; start with code-level metadata and tests.

**Verification**

- Existing key priority tests in `src/app/tests.rs`.
- New tests asserting help overlay/action metadata contains expected key labels for core actions.
- `cargo test`, `cargo clippy --all-targets --all-features`, `cargo fmt --check`.

### P1 — Introduce a fakeable jj backend seam before command refactors

**Problem**

`App` command methods call concrete `JjRunner`/fetch functions directly, making mutation contracts hard to test without real jj behavior. AGENTS.md requires mutating jj commands to store command results and refresh the log, but future commands can bypass that pattern.

**Proposed refactor**

Add a narrow backend/runner trait seam around jj process execution and fetch operations. Start by making command-result handling testable with a fake backend rather than rewriting all command logic. Keep `JjRunner` as the production implementation.

Include both mutating commands and read/fetch paths currently reached from `App`:

- `src/app/mod.rs::open_detail()` → `fetch_show`.
- `src/app/loading.rs::load_more_entries()` → `fetch_graph_log_after`.
- `src/app/commands.rs::open_diff_view()` → direct `run_capture` for diff summary.
- `src/app/commands.rs::refresh_diff_text()` → `fetch_diff_file`.

**Risk**

- Trait boundaries can become too broad if they mirror every current function at once.
- Lifetime/generic complexity can make `App` harder to read.
- Incorrect abstraction could change command ordering: `handle_command_result(result)` must precede `refresh_log()` for mutating commands.

**Verification**

- Fake-backend tests for representative mutating commands confirming result storage and log refresh.
- Fake-backend tests for `open_detail`, `open_diff_view` / `refresh_diff_text`, and `load_more_entries`.
- Tests should preserve the ordering contract that `handle_command_result(result)` runs before `refresh_log()` for mutating commands.
- Existing app command/navigation tests.
- Full required checks after implementation.

### P1 — Tighten jj parser/template contract constants and fixtures

**Problem**

Parser contracts are documented and tested, but templates and parser field expectations remain parallel hand-maintained strings/indices. `src/jj/show.rs` uses an 8-field null-byte template; `src/jj/graph_log.rs` depends on 8-character change IDs and graph-only lines remaining visible but not selectable.

**Proposed refactor**

Add named constants or a small field-index enum for show metadata fields and expected field count. Add direct tests for shared bookmark parsing. Prefer fixture-based parser tests for representative jj output before changing graph regex/template behavior.

For bookmark parsing, treat the jj template output `bookmarks.join(",")` as the canonical contract. The parser should preserve field contents rather than silently trimming whitespace unless a later jj compatibility investigation justifies changing that behavior.

**Risk**

- Changing templates without parser updates breaks detail/show parsing.
- Broadening graph change-id parsing could violate documented 8-character contract.
- Rename/copy diff behavior may need integration validation before implementation changes.

**Verification**

- Existing parser tests in `src/jj/graph_log.rs` and `src/jj/show.rs`.
- New tests asserting show template field count/order documentation matches parser constants.
- Direct bookmark parser unit tests for empty fields, comma-separated bookmark names, and literal whitespace preservation.

### P2 — Extract shared viewport and scroll helpers

**Problem**

Viewport/scroll behavior is repeated across log/detail/diff rendering and navigation: clamping, selected-visible windows, scrollbar construction, and status bar shape.

**Proposed refactor**

Create small pure helpers for vertical/horizontal bounds and selected-visible calculations. Apply them gradually to `src/app/navigation.rs`, `src/ui/log.rs`, `src/ui/detail.rs`, and `src/ui/diff.rs`.

**Risk**

- TUI safety rules require scroll positions to be clamped when rendered content height changes.
- Small terminal edge cases can regress if helper assumptions differ between views.

**Verification**

- Unit tests for helper boundary cases: empty content, one-line viewport, content smaller than viewport, selection before/after visible window.
- Existing render smoke tests and navigation tests.

### P2 — Split `App` state into smaller domain/view state structs

**Problem**

`src/app/mod.rs` centralizes UI, backend/loading, input/modal, and view-specific state. As features grow, `App` risks becoming a god object.

**Proposed refactor**

Extract behavior-neutral modules such as `app/state.rs`, `app/view_state.rs`, and `app/actions.rs`, or smaller structs for detail/diff/log view state. Do this only after command/backend seams are clearer.

**Risk**

- Mechanical moves can obscure behavior changes.
- Borrowing between `App`, backend, and view state may complicate command code.

**Verification**

- Existing app tests should remain behavior-identical.
- `cargo test` after each small extraction batch.

### P3 — Rendering allocation/display-model cleanup

**Problem**

`src/ui/detail.rs`, `src/ui/diff.rs`, and `src/ui/log.rs` build many owned `String`/`Line` values during render. This is acceptable now but may stutter with large diffs/logs.

**Proposed refactor**

Only after P1/P2, consider non-functional display-model or visible-slice helpers for large content. Avoid optimizing before behavior seams and tests are stable.

**Risk**

- Premature display-model caching can introduce stale UI state.
- Performance changes are hard to validate without representative large diffs.

**Verification**

- Render tests for identical output.
- Manual or benchmark-like large diff/log smoke checks if performance becomes a goal.

## Suggested implementation batches

1. **Parser constants and bookmark tests**: lowest-risk P1, keeps jj parsing contracts explicit.
2. **Command/action metadata for help/key consistency**: code-only first; defer README generation. The first metadata batch should only drive help/status consistency tests and must not route command execution through metadata until the backend seam exists.
3. **jj backend fake seam and mutation-refresh tests**: enables safer future command refactors.
4. **Viewport/scroll helper extraction**: pure helpers and boundary tests.
5. **App state module extraction**: mechanical once seams are clearer.

## Global verification method

For any implementation batch, use `jj status`/`jj diff` for review and run the project-required checks before finalizing:

```bash
cargo test
cargo clippy --all-targets --all-features
cargo fmt --check
```

For parser/template changes, update parser tests in the same batch as template changes. For mutating jj command changes, ensure command result handling and log refresh remain paired.
