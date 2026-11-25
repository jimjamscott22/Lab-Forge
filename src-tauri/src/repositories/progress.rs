//! SQL for attempts, step progress and notes.

use sqlx::SqliteConnection;

use crate::error::AppResult;
use crate::models::challenge::{AttemptStatus, StepKind};
use crate::models::progress::*;

pub async fn find_attempt(conn: &mut SqliteConnection, id: i64) -> AppResult<Option<Attempt>> {
    Ok(sqlx::query_as::<_, Attempt>(
        "SELECT id, challenge_id, status, started_at, completed_at FROM attempts WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

pub async fn open_attempt_id(
    conn: &mut SqliteConnection,
    challenge_id: i64,
) -> AppResult<Option<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM attempts WHERE challenge_id = ?1 AND status = 'in_progress'",
    )
    .bind(challenge_id)
    .fetch_optional(conn)
    .await?)
}

/// Create an attempt and one `not_started` row per step of that challenge version.
pub async fn create_attempt(conn: &mut SqliteConnection, challenge_id: i64) -> AppResult<i64> {
    let attempt_id: i64 = sqlx::query_scalar(
        "INSERT INTO attempts (challenge_id, status) VALUES (?1, 'in_progress') RETURNING id",
    )
    .bind(challenge_id)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO attempt_steps (attempt_id, step_id, status) \
         SELECT ?1, id, 'not_started' FROM challenge_steps WHERE challenge_id = ?2",
    )
    .bind(attempt_id)
    .bind(challenge_id)
    .execute(conn)
    .await?;
    Ok(attempt_id)
}

pub async fn attempt_steps(
    conn: &mut SqliteConnection,
    attempt_id: i64,
) -> AppResult<Vec<AttemptStep>> {
    Ok(sqlx::query_as::<_, AttemptStep>(
        "SELECT a.step_id, a.status, a.completed_at, a.selected_choice_id \
         FROM attempt_steps a JOIN challenge_steps s ON s.id = a.step_id \
         WHERE a.attempt_id = ?1 ORDER BY s.position",
    )
    .bind(attempt_id)
    .fetch_all(conn)
    .await?)
}

/// First step, in order, that has not been completed.
pub async fn next_open_step(
    conn: &mut SqliteConnection,
    attempt_id: i64,
) -> AppResult<Option<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT a.step_id FROM attempt_steps a JOIN challenge_steps s ON s.id = a.step_id \
         WHERE a.attempt_id = ?1 AND a.status = 'not_started' ORDER BY s.position LIMIT 1",
    )
    .bind(attempt_id)
    .fetch_optional(conn)
    .await?)
}

/// Kind of `step_id` if it belongs to the challenge version this attempt was started on.
pub async fn step_kind_in_attempt(
    conn: &mut SqliteConnection,
    attempt_id: i64,
    step_id: i64,
) -> AppResult<Option<StepKind>> {
    Ok(sqlx::query_scalar(
        "SELECT s.kind FROM attempt_steps a JOIN challenge_steps s ON s.id = a.step_id \
         WHERE a.attempt_id = ?1 AND a.step_id = ?2",
    )
    .bind(attempt_id)
    .bind(step_id)
    .fetch_optional(conn)
    .await?)
}

#[derive(sqlx::FromRow)]
pub struct ChoiceCheck {
    pub is_correct: bool,
    pub explanation: String,
}

pub async fn choice_check(
    conn: &mut SqliteConnection,
    step_id: i64,
    choice_id: i64,
) -> AppResult<Option<ChoiceCheck>> {
    Ok(sqlx::query_as::<_, ChoiceCheck>(
        "SELECT is_correct, explanation FROM step_choices WHERE id = ?1 AND step_id = ?2",
    )
    .bind(choice_id)
    .bind(step_id)
    .fetch_optional(conn)
    .await?)
}

/// Returns the number of rows changed (0 if the step was already completed).
pub async fn mark_step_completed(
    conn: &mut SqliteConnection,
    attempt_id: i64,
    step_id: i64,
    choice_id: Option<i64>,
) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE attempt_steps \
            SET status = 'completed', selected_choice_id = ?3, \
                completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
          WHERE attempt_id = ?1 AND step_id = ?2 AND status = 'not_started'",
    )
    .bind(attempt_id)
    .bind(step_id)
    .bind(choice_id)
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

pub async fn complete_attempt(conn: &mut SqliteConnection, attempt_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE attempts SET status = 'completed', \
                completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
          WHERE id = ?1 AND status = 'in_progress'",
    )
    .bind(attempt_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn get_note(conn: &mut SqliteConnection, attempt_id: i64) -> AppResult<Option<Note>> {
    Ok(sqlx::query_as::<_, Note>(
        "SELECT id, attempt_id, body_md, created_at, updated_at FROM notes WHERE attempt_id = ?1",
    )
    .bind(attempt_id)
    .fetch_optional(conn)
    .await?)
}

pub async fn upsert_note(
    conn: &mut SqliteConnection,
    attempt_id: i64,
    body_md: &str,
) -> AppResult<Note> {
    Ok(sqlx::query_as::<_, Note>(
        "INSERT INTO notes (attempt_id, body_md) VALUES (?1, ?2) \
         ON CONFLICT (attempt_id) DO UPDATE SET body_md = excluded.body_md, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         RETURNING id, attempt_id, body_md, created_at, updated_at",
    )
    .bind(attempt_id)
    .bind(body_md)
    .fetch_one(conn)
    .await?)
}

/// Shared column list for dashboard cards. A macro (not a `const`) so queries are
/// still assembled from string literals at compile time; sqlx 0.9 rejects runtime SQL.
macro_rules! card_select {
    () => {
        r"SELECT a.id AS attempt_id, a.challenge_id, c.title AS challenge_title,
        a.status, a.started_at, a.completed_at,
        (SELECT COUNT(*) FROM attempt_steps s WHERE s.attempt_id = a.id AND s.status = 'completed') AS steps_completed,
        (SELECT COUNT(*) FROM attempt_steps s WHERE s.attempt_id = a.id) AS steps_total
   FROM attempts a JOIN challenges c ON c.id = a.challenge_id"
    };
}

pub async fn current_card(conn: &mut SqliteConnection) -> AppResult<Option<AttemptCard>> {
    Ok(sqlx::query_as::<_, AttemptCard>(concat!(
        card_select!(),
        " WHERE a.status = 'in_progress' ORDER BY a.started_at DESC, a.id DESC LIMIT 1"
    ))
    .fetch_optional(conn)
    .await?)
}

pub async fn recent_cards(conn: &mut SqliteConnection, limit: i64) -> AppResult<Vec<AttemptCard>> {
    Ok(sqlx::query_as::<_, AttemptCard>(concat!(
        card_select!(),
        " ORDER BY a.started_at DESC, a.id DESC LIMIT ?1"
    ))
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn count_attempts_with_status(
    conn: &mut SqliteConnection,
    status: AttemptStatus,
) -> AppResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM attempts WHERE status = ?1")
            .bind(status)
            .fetch_one(conn)
            .await?,
    )
}

/// Distinct challenges (by slug) that have at least one completed attempt.
pub async fn count_completed_challenges(conn: &mut SqliteConnection) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(DISTINCT c.slug) FROM attempts a JOIN challenges c ON c.id = a.challenge_id \
         WHERE a.status = 'completed'",
    )
    .fetch_one(conn)
    .await?)
}

/// Distinct challenges (by slug) available in the library.
pub async fn count_available_challenges(conn: &mut SqliteConnection) -> AppResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(DISTINCT slug) FROM challenges")
            .fetch_one(conn)
            .await?,
    )
}
