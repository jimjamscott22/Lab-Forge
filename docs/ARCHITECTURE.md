# Architecture

LabForge is a Tauri 2 desktop app. The React UI runs in the system WebView and talks to
Rust only through typed Tauri commands. There is no HTTP server and no network listener.

```
React (src/)  --invoke-->  commands  -->  services  -->  repositories  -->  SQLite
  views                    (adapters)     (rules,         (SQL only)         (WAL file in
  components                              validation)                         app data dir)
  api/client.ts  <- one typed wrapper
```

Dependencies point down only. `models` are shared data types, `content` holds bundled
definitions, `error` is the shared error type, and `lab_runtime` is an empty placeholder
reserved for Milestone 3.

## Rust modules (`src-tauri/src`)

| Module | Responsibility |
| --- | --- |
| `commands` | Thin `#[tauri::command]` adapters. Deserialize input, call one service function, return the result or an `AppError`. No logic. |
| `services` | Business rules: input validation, attempt state transitions, seeding, link checks. Independent of Tauri; tested directly. |
| `repositories` | SQL. Functions take `&mut SqliteConnection`, so callers choose pool or transaction. Queries are string literals (sqlx 0.9 refuses runtime-built SQL). |
| `database` | Pool options (WAL, foreign keys on, busy timeout) and the migration runner. |
| `models` | Serializable types. `challenge` is content, `progress` is user data. |
| `content` | Bundled challenge JSON embedded with `include_str!`. |
| `error` | `AppError` with a stable `code` and learner-safe message. |

## Data model

Content and progress are separate tables, joined only by foreign keys.

```
challenges (slug, version, source) 1--* challenge_steps (step_key, position, kind)
                                          1--* step_hints
                                          1--* step_choices (is_correct, explanation)
challenges 1--* challenge_evidence (kind, JSON content)

challenges 1--* attempts (status) 1--* attempt_steps (status, selected_choice_id)
                                  1--1 notes
```

- **Versioned definitions.** A challenge row is one `(slug, version)`. An attempt points at the
  exact version row it started on, with `ON DELETE RESTRICT`. Shipping version 2 adds a new row;
  version 1 and its attempts are untouched. Authors must bump `version` when content changes,
  because seeding skips an existing `(slug, version)`.
- **Stable step identity.** Steps have a `step_key` plus an explicit `position`.
- **Retries are new attempts.** A partial unique index allows one `in_progress` attempt per
  challenge. Each attempt has its own notebook. History (`list_attempts`) filters by `slug`, so
  attempts on every version of a challenge appear together; it returns at most 200, newest first.
- **Constraints in the schema.** Status sets are `CHECK`ed, `completed_at` must match
  `status`, a knowledge check has exactly one correct choice (partial unique index), and
  every reference has an explicit `ON DELETE`.
- **Timestamps** are RFC 3339 UTC text from SQLite. **Keys** are integer rowids because the
  database is local and single-user; stable identity for content is `slug` / `step_key`.
- **JSON** is used in exactly one place: evidence payloads, whose shape depends on `kind`
  and is validated in Rust. Everything else is columns.

Not built yet from the prompt's data model: tools, learning paths, lab templates, lab
instances, and evidence records for findings. They arrive with the milestones that use them.

## Rules the service layer owns

- Steps complete strictly in order; completing an already-completed step is a conflict.
- A knowledge check completes only on the correct choice. A wrong answer writes nothing and
  returns the explanation. Which choice is correct never leaves Rust.
- A note step completes only once the attempt's note has non-blank content.
- Completing the last step and completing the attempt are one transaction.
- Custom challenges can be edited or deleted only while no attempt references them. Bundled
  challenges are read-only.

## IPC contract

Commands: `list_challenges`, `get_challenge`, `create_challenge`, `update_challenge`,
`delete_challenge`, `start_attempt`, `get_attempt`, `complete_step`, `save_note`,
`get_dashboard`, `list_attempts`, `open_external_link`.

Adding one means touching four places, and a unit test fails if they disagree:
the Rust command, `invoke_handler` in `lib.rs`, the list in `build.rs`, and the
`allow-<name>` permission in `capabilities/default.json`. Then add it to `src/api/client.ts`.

Errors cross the boundary as `{ code, message }`. `validation`, `not_found` and `conflict`
messages are written for the learner. `internal` errors are logged in Rust and reach the UI
as one generic message, so SQL and file paths never appear in the interface.

## Frontend (`src`)

- `api/client.ts` is the only place that calls `invoke`. A `Commands` interface types every
  command's arguments and result.
- `App.tsx` is a four-view shell (Dashboard, Challenges, History, Workspace) with state-based
  routing; a router library is not worth a dependency for four views.
- The workspace puts the step list, active instructions, packet summary and notebook on one
  screen. The notebook draft autosaves, never drops text when a save fails, and is flushed
  before a note step is submitted.
- Tailwind v4 theme tokens live in `src/index.css`. Fonts are bundled (Atkinson Hyperlegible
  for UI, JetBrains Mono only for packet data) so the app works offline.
- Styling never uses inline `style` attributes, because the production CSP forbids them.

## Decisions worth knowing

- **Tauri over a local web server:** no listening port, and Rust owns validation and data.
- **SQLx with runtime-checked queries:** compile-time macros would need a live database at build
  time; tests against real SQLite files cover query correctness instead.
- **Repositories take a connection, not a pool:** services compose several repository calls in
  one transaction without repositories knowing about it.
- **Seeding by `(slug, version)`:** idempotent across launches and safe for saved attempts.
- **Custom challenges carry only plain task steps** and Rust generates their slug and step keys,
  so the frontend never chooses identifiers.
- **`LABFORGE_DATA_DIR`** overrides the data location for restart checks and scratch runs.

## Testing

Rust: unit tests for validation, link checks and error serialization; database tests against
real on-disk SQLite in temp directories for migrations, repeat seeding, invalid step
transitions, two attempts on one challenge, version preservation, and close-and-reopen
persistence. Frontend: Vitest with Testing Library for the IPC wrapper, Markdown safety, the
packet ledger, step flow, failed saves, and keyboard navigation.
