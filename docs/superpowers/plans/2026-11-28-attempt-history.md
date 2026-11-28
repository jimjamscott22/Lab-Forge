# Attempt History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a learner see every attempt they have made, across all versions of a challenge, and open any of them, so that "retry an exercise with its previous attempt intact" is something they can check in the UI.

**Architecture:** One new read-only command, `list_attempts`, that reuses the existing `AttemptCard` query (`repositories/progress.rs` `card_select!`), optionally filtered to one challenge *slug* (every version). A new `History` view lists the cards; completed attempts open in the existing read-only Workspace summary. No migration: attempts, steps and notes already store everything history needs.

**Tech Stack:** Tauri 2, Rust + SQLx (SQLite), React 19 + TypeScript, Vitest + Testing Library.

**Spec:** `docs/ROADMAP.md` (Milestone 2: "attempt history view"; acceptance "complete a path and retry an exercise with its previous attempt intact") and `docs/LabForge-Desktop-Design-Report.docx` ("Step state and timestamps support resuming, completion, and retries with history"). Rules that must keep holding are in `docs/ARCHITECTURE.md` and `docs/SECURITY.md`.

**Why this is M2's first task:** it needs no schema change and no new content, it closes the "retry with previous attempt intact" half of the M2 acceptance check, and the other M2 items (tools, learning paths, more exercises, authoring UI, backup) do not depend on it or block it. Learning paths come next and need more bundled exercises first.

## Global Constraints

- Commands stay thin; logic lives in `services`, SQL in `repositories` as string literals (sqlx 0.9 refuses runtime-built SQL; use `concat!` with `card_select!()` like `recent_cards`).
- A new command touches five places: `commands/mod.rs` (`COMMAND_NAMES` + fn), `lib.rs` `invoke_handler`, `build.rs` `COMMANDS`, `capabilities/default.json` (`allow-list-attempts`), and `src/api/client.ts` `Commands`. The existing test `commands_are_registered_manifested_and_granted_consistently` must pass.
- No list is unbounded: history returns at most `HISTORY_LIMIT = 200` cards, newest first (`started_at DESC, id DESC`).
- Errors cross IPC as `{ code, message }`; an unknown challenge is `AppError::NotFound("Challenge")`.
- No inline `style` attributes (production CSP). Every list row's button has an `aria-label` that names the attempt, as the Dashboard does.
- Frontend never calls `invoke` outside `src/api/client.ts`.

## Review Focus

1. **Retry across a content version bump.** Filtering by the v1 challenge id must still show attempts made on v2 (and vice versa): filter by slug, not id. — Task 1 test `history_for_a_challenge_includes_every_version_of_its_slug`.
2. **Opening an old completed attempt shows *its* note, not the newest one's.** — Task 3 test `finished attempt links to its challenge history` uses a fixture note and asserts that note's text.
3. **Unknown challenge id** (stale route after a custom challenge is deleted) returns `not_found`, and the view shows the message with **Try again** and **Show all attempts**, not a blank page. — Task 1 `history_for_an_unknown_challenge_is_not_found`, Task 2 `shows the error and a way back to all attempts`.
4. **Very long history** stays bounded and says so. — Task 1 `history_is_capped_at_the_limit`, Task 2 `says when only the most recent attempts are shown`.
5. **An in-progress attempt in the list** shows step progress and **Open** resumes it; it never starts a new attempt. — Task 2 `lists attempts newest first and opens one`.

---

### Task 1: `list_attempts` command (Rust)

**Files:**
- Modify: `src-tauri/src/models/progress.rs` (`AttemptCard`)
- Modify: `src-tauri/src/repositories/progress.rs` (`card_select!`, new `history_cards`)
- Modify: `src-tauri/src/services/attempts.rs` (new `history`, `HISTORY_LIMIT`, tests)
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`

**Interfaces:**
- Produces:
  - `AttemptCard` gains `pub challenge_version: i64` (serialized `challengeVersion`). Selected in `card_select!` as `c.version AS challenge_version`, so `current_card` and `recent_cards` carry it too.
  - `repositories::progress::history_cards(conn: &mut SqliteConnection, slug: Option<&str>, limit: i64) -> AppResult<Vec<AttemptCard>>` — `WHERE (?1 IS NULL OR c.slug = ?1) ORDER BY a.started_at DESC, a.id DESC LIMIT ?2`.
  - `services::attempts::HISTORY_LIMIT: i64 = 200` (pub).
  - `services::attempts::history(pool: &SqlitePool, challenge_id: Option<i64>) -> AppResult<Vec<AttemptCard>>` — with `Some(id)`, resolve the slug (add `repositories::challenges::slug_of(conn, id) -> AppResult<Option<String>>` next to `source_of`); `None` slug from a `Some` id → `NotFound("Challenge")`.
  - Command `list_attempts(state, challenge_id: Option<i64>) -> AppResult<Vec<AttemptCard>>`.

- [ ] **Step 1: Write the failing tests** in the `tests` module of `services/attempts.rs` (reuse `setup`, `finish`, `fresh`):

```rust
#[tokio::test]
async fn history_keeps_the_previous_attempt_intact_after_a_retry() {
    let db = fresh().await;
    let cid = setup(&db.pool).await;
    let first = start(&db.pool, cid).await.unwrap().attempt.id;
    finish(&db.pool, first).await;
    let retry = start(&db.pool, cid).await.unwrap();
    complete_step(&db.pool, retry.attempt.id, step_ids(&retry)[0], None).await.unwrap();

    let cards = history(&db.pool, None).await.unwrap();
    assert_eq!(cards.iter().map(|c| c.attempt_id).collect::<Vec<_>>(), vec![retry.attempt.id, first]);
    assert_eq!((cards[0].status, cards[0].steps_completed), (AttemptStatus::InProgress, 1));
    assert_eq!((cards[1].status, cards[1].steps_completed, cards[1].steps_total), (AttemptStatus::Completed, 4, 4));
    assert_eq!(cards[1].challenge_version, 1);
    assert_eq!(get(&db.pool, first).await.unwrap().note.unwrap().body_md, "Packet 4 retried packet 3.");
}

#[tokio::test]
async fn history_for_a_challenge_includes_every_version_of_its_slug() {
    // Start + finish on v1, insert v2 exactly as
    // services::challenges::tests::new_bundled_version_keeps_the_definition_saved_attempts_use does
    // (repo::insert with version 2), start an attempt on v2.
    // assert: history(Some(v1_id)) and history(Some(v2_id)) both return 2 cards,
    //         with challenge_version [2, 1].
}

#[tokio::test]
async fn history_for_a_challenge_excludes_other_challenges() {
    // Create a custom challenge with services::challenges::create_custom, start one attempt
    // on it and one on the bundled challenge.
    // assert: history(Some(bundled_id)) has exactly the bundled attempt; history(None) has both.
}

#[tokio::test]
async fn history_for_an_unknown_challenge_is_not_found() {
    let db = fresh().await;
    setup(&db.pool).await;
    assert!(matches!(history(&db.pool, Some(9_999)).await, Err(AppError::NotFound("Challenge"))));
}

#[tokio::test]
async fn history_is_capped_at_the_limit() {
    // Insert HISTORY_LIMIT + 1 completed attempts directly:
    // WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < ?2)
    // INSERT INTO attempts (challenge_id, status, completed_at)
    // SELECT ?1, 'completed', strftime('%Y-%m-%dT%H:%M:%fZ','now') FROM n
    // assert: history(None).len() == HISTORY_LIMIT as usize, and cards[0].attempt_id is the max id.
}
```

The two comment-bodied tests are specified by their comments; write them in the same style as the first.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test history_`
Expected: compile error, `history` / `challenge_version` not found.

- [ ] **Step 3: Implement** the model field, `card_select!` column, `slug_of`, `history_cards`, `HISTORY_LIMIT` and `history` per the Interfaces block. `AttemptCard` is only built by `FromRow`, so the new column is the only change the existing queries need.

- [ ] **Step 4: Run the history tests**

Run: `cd src-tauri && cargo test history_`
Expected: 5 passed.

- [ ] **Step 5: Register the command** — add `list_attempts` to `COMMAND_NAMES`, a thin `#[tauri::command] pub async fn list_attempts` calling `attempts::history`, `commands::list_attempts,` in `lib.rs`, `"list_attempts"` in `build.rs`, and `"allow-list-attempts"` in `capabilities/default.json`.

- [ ] **Step 6: Run the full Rust suite and lints**

Run: `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: all pass, including `commands_are_registered_manifested_and_granted_consistently`.

- [ ] **Step 7: Commit**

```bash
git add src-tauri
git commit -m "feat: add list_attempts command for attempt history"
```

---

### Task 2: History view (frontend)

**Files:**
- Modify: `src/api/types.ts` (`AttemptCard.challengeVersion: number`)
- Modify: `src/api/client.ts` (`Commands.list_attempts`, `api.listAttempts`)
- Modify: `src/api/client.test.ts`
- Modify: `src/test/fixtures.ts` (add `attemptCard(over?: Partial<AttemptCard>): AttemptCard`; default `challengeVersion: 1`)
- Create: `src/views/History.tsx`, `src/views/History.test.tsx`
- Modify: `src/App.tsx` (route + nav)

**Interfaces:**
- Consumes: command `list_attempts` with args `{ challengeId: number | null }`, result `AttemptCard[]` (Task 1).
- Produces:
  - `api.listAttempts(challengeId?: number): Promise<AttemptCard[]>` (sends `challengeId ?? null`).
  - `export const HISTORY_LIMIT = 200` in `src/views/History.tsx` (must equal the Rust constant).
  - `Route` gains `{ name: "history"; challengeId?: number }`.
  - `export function History({ challengeId, navigate }: { challengeId?: number; navigate: (route: Route) => void })`.

- [ ] **Step 1: Write the failing tests**

In `client.test.ts`: `listAttempts sends a null challengeId when unfiltered` — assert `invoke` called with `("list_attempts", { challengeId: null })`, and with `{ challengeId: 3 }` for `listAttempts(3)`.

In `History.test.tsx` (mock `api.listAttempts` the way `Dashboard.test.tsx` mocks `getDashboard`):

- `lists attempts newest first and opens one` — two cards (in-progress `attemptId: 9`, 1 of 4; completed `attemptId: 7`, `challengeVersion: 2`). Assert rows in that order, text `In progress, 1 of 4 steps done`, text `Version 2`, and clicking the button labelled `Open attempt: Read a DNS timeout and retry, In progress, 1 of 4 steps done` calls `navigate({ name: "attempt", attemptId: 9 })`.
- `filters to one challenge and offers all attempts` — render with `challengeId={1}`; assert `listAttempts` called with `1`, heading `Attempt history: Read a DNS timeout and retry` (title from the first card), and **Show all attempts** calls `navigate({ name: "history" })`.
- `explains an empty history` — `[]` → text `You have not started any attempts yet.` and a **Browse challenges** button navigating to `{ name: "challenges" }`.
- `shows the error and a way back to all attempts` — rejects with `new AppError("not_found", "Challenge not found.")`, `challengeId={5}`; `role="alert"` contains the message; **Try again** calls `listAttempts` again; **Show all attempts** navigates to `{ name: "history" }`.
- `says when only the most recent attempts are shown` — 200 cards → text `Showing the 200 most recent attempts.`; 199 cards → that text absent.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm test src/views/History.test.tsx src/api/client.test.ts`
Expected: FAIL, `./History` not found / `listAttempts` not a function.

- [ ] **Step 3: Implement** the type field, client entry, fixture, and `History.tsx`. Load with `useLoad(() => api.listAttempts(challengeId), [challengeId])`. Heading is `Attempt history` unfiltered, `Attempt history: <title>` filtered. Reuse the row markup, `describe()` wording and `aria-label` pattern from `Dashboard.tsx`; move `describe` to `src/lib/format.ts` as `export function describeAttempt(card: AttemptCard): string` and use it from both views. Each row also shows `Version N` and `Started <formatDateTime>`.

- [ ] **Step 4: Wire the route** — add `{ route: { name: "history" }, label: "History" }` to `NAV` and render `<History challengeId={route.challengeId} navigate={setRoute} />`. `activeNav` for `"history"` is `"history"` (attempts keep `"challenges"`).

- [ ] **Step 5: Run the frontend checks**

Run: `pnpm typecheck && pnpm test`
Expected: all pass (existing Dashboard tests still pass after the `describeAttempt` move).

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "feat: add attempt history view"
```

---

### Task 3: Entry points and docs

**Files:**
- Modify: `src/views/Workspace.tsx` (finished summary), `src/views/Workspace.test.tsx`
- Modify: `src/views/Dashboard.tsx` (recent section), `src/views/Dashboard.test.tsx`
- Modify: `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `README.md`

**Interfaces:**
- Consumes: `Route` `{ name: "history"; challengeId?: number }` (Task 2); `AttemptView.attempt.challengeId` (existing).

- [ ] **Step 1: Write the failing tests**

- `Workspace.test.tsx` → `finished attempt links to its challenge history`: render `attemptView({ status: "completed", done: 4, note: "Old attempt note" })`; assert text `Old attempt note` is shown, then clicking **View attempt history** calls `navigate({ name: "history", challengeId: <fixture challengeId> })`.
- `Dashboard.test.tsx` → `links to the full history`: with one recent card, clicking **See all attempts** calls `navigate({ name: "history" })`; with `recent: []`, the button is absent.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm test src/views/Workspace.test.tsx src/views/Dashboard.test.tsx`
Expected: FAIL, buttons not found.

- [ ] **Step 3: Implement** — in Workspace's "Attempt complete" button row, add a secondary **View attempt history** button after **Start another attempt**. In Dashboard, add a secondary **See all attempts** button under the recent list when `recent.length > 0`. Both use the existing secondary button classes (`rounded border border-ink-600 …`).

- [ ] **Step 4: Run all checks**

Run: `pnpm typecheck && pnpm test && pnpm build && (cd src-tauri && cargo test)`
Expected: all pass.

- [ ] **Step 5: Update docs**
  - `ARCHITECTURE.md`: add `list_attempts` to the IPC command list; add `History` to the frontend views sentence ("four-view shell").
  - `ROADMAP.md`: under Milestone 2 add `Delivered: attempt history (list_attempts, History view).` Leave Milestone 1's status alone.
  - `README.md`, "Try the restart check yourself": add step 6 — open **History**, confirm both attempts are listed and the first one still opens with its notes.

- [ ] **Step 6: Manual acceptance in the native app**

Run: `LABFORGE_DATA_DIR=/tmp/labforge-history pnpm tauri dev`, finish the DNS exercise with a note, choose **Start another attempt**, complete one step, quit and relaunch.
Expected: **History** lists two attempts (in progress 1 of 4, completed); opening the completed one shows its original note; the Workspace's **View attempt history** shows the same two.

- [ ] **Step 7: Commit**

```bash
git add src docs README.md
git commit -m "feat: link attempt history from dashboard and finished attempts"
```
