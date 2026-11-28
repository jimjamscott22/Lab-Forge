# Roadmap

Milestones follow the November 2026 design report. Each has an acceptance check; a milestone is
done when its check passes in the native app, not just in tests.

## Milestone 1: Desktop foundation (current)

Native Ubuntu app, typed IPC, SQLite migrations, one guided exercise.
**Acceptance:** finish the exercise and recover its progress and note after a restart.

Delivered: scaffold, schema and migration, bundled DNS exercise, challenge CRUD commands,
attempt/step/note commands, dashboard, challenge library with search, workspace with hints,
knowledge check, packet summary and notebook, tests, and docs.

Still open before calling it done:
- Run the restart check through the native UI (steps are in the README). Process-level and
  data-layer restarts are verified; UI-driven automation needs `webkit2gtk-driver` and `tauri-driver`.
- Verify the exercise end to end on a clean Ubuntu desktop.

## Milestone 2: Learning MVP

Searchable tool references, several exercises, ordered learning paths, attempt history.
**Acceptance:** complete a path and retry an exercise with its previous attempt intact.

Includes: `tools` and `learning_paths` (+ ordered steps) tables, an attempt history view, a UI to
create and edit custom challenges (the commands already exist), more bundled content, and a
SQLite online-backup snapshot with a test that restores it into a fresh data directory.

Delivered: attempt history (`list_attempts`, History view).

## Milestone 3: Local lab pilot

Simulated runtime first, then a controlled local HTTP fixture.
**Acceptance:** start, stop, reset and recover the fixture after restarting the app; verify
network isolation.

Includes: `lab_templates` and `lab_instances`, the `lab_runtime` adapter (candidate: Bollard),
states for unavailable, starting, ready, stopped and error, and every requirement in
[SECURITY.md](SECURITY.md#requirements-for-the-docker-runtime-milestone-3).

## Milestone 4: Portfolio release

Markdown evidence export, accessible UI audit, Ubuntu packaging, Windows verification.
**Acceptance:** install on supported desktops and open an exported report independently.

Includes: the `evidence` table, exportable reports, `.deb`/AppImage packaging, and a full pass on
Windows (WebView2, CSP behavior, paths, packaging).

## Backlog, not scheduled

- Automated dependency auditing in CI.
- A screen-reader pass beyond the keyboard and focus checks done in Milestone 1.
- Content-hash check so a bundled file edited without a version bump is detected.
