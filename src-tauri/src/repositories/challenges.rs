//! SQL for challenge content. Functions take a connection so callers decide
//! whether they run on the pool or inside a transaction.

use sqlx::SqliteConnection;

use crate::error::{AppError, AppResult};
use crate::models::challenge::*;

pub async fn list(
    conn: &mut SqliteConnection,
    query: Option<&str>,
) -> AppResult<Vec<ChallengeSummary>> {
    let pattern = query
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{}%", escape_like(q)));

    // Shows the newest version of each challenge. The latest-attempt join looks
    // across every version of the slug so a retry history survives a content update.
    let rows = sqlx::query_as::<_, ChallengeSummary>(
        r"SELECT c.id, c.slug, c.version, c.source, c.title, c.summary, c.difficulty,
                 (SELECT COUNT(*) FROM challenge_steps s WHERE s.challenge_id = c.id) AS step_count,
                 (SELECT COUNT(*) FROM attempts a JOIN challenges c2 ON c2.id = a.challenge_id
                   WHERE c2.slug = c.slug) AS attempt_count,
                 la.status AS latest_attempt_status,
                 la.id     AS latest_attempt_id
            FROM challenges c
            LEFT JOIN attempts la ON la.id = (
                 SELECT a.id FROM attempts a JOIN challenges c2 ON c2.id = a.challenge_id
                  WHERE c2.slug = c.slug
                  ORDER BY a.started_at DESC, a.id DESC LIMIT 1)
           WHERE c.version = (SELECT MAX(version) FROM challenges WHERE slug = c.slug)
             AND (?1 IS NULL OR c.title LIKE ?1 ESCAPE '\' OR c.summary LIKE ?1 ESCAPE '\')
           ORDER BY c.source ASC, c.title COLLATE NOCASE ASC",
    )
    .bind(pattern)
    .fetch_all(conn)
    .await?;
    Ok(rows)
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[derive(sqlx::FromRow)]
struct ChallengeRow {
    id: i64,
    slug: String,
    version: i64,
    source: ChallengeSource,
    title: String,
    summary: String,
    difficulty: Difficulty,
    objectives_md: String,
}

#[derive(sqlx::FromRow)]
struct StepRow {
    id: i64,
    step_key: String,
    position: i64,
    kind: StepKind,
    title: String,
    instructions_md: String,
    completion_criteria: String,
}

#[derive(sqlx::FromRow)]
struct HintRow {
    step_id: i64,
    position: i64,
    body_md: String,
}

#[derive(sqlx::FromRow)]
struct ChoiceRow {
    id: i64,
    step_id: i64,
    position: i64,
    label: String,
}

#[derive(sqlx::FromRow)]
struct EvidenceRow {
    evidence_key: String,
    title: String,
    content: String,
}

pub async fn get_detail(
    conn: &mut SqliteConnection,
    id: i64,
) -> AppResult<Option<ChallengeDetail>> {
    let Some(row) = sqlx::query_as::<_, ChallengeRow>(
        "SELECT id, slug, version, source, title, summary, difficulty, objectives_md \
         FROM challenges WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(None);
    };

    let step_rows = sqlx::query_as::<_, StepRow>(
        "SELECT id, step_key, position, kind, title, instructions_md, completion_criteria \
         FROM challenge_steps WHERE challenge_id = ?1 ORDER BY position",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;

    let hint_rows = sqlx::query_as::<_, HintRow>(
        "SELECT h.step_id, h.position, h.body_md FROM step_hints h \
         JOIN challenge_steps s ON s.id = h.step_id \
         WHERE s.challenge_id = ?1 ORDER BY h.step_id, h.position",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;

    // Note: `is_correct` is intentionally not selected.
    let choice_rows = sqlx::query_as::<_, ChoiceRow>(
        "SELECT c.id, c.step_id, c.position, c.label FROM step_choices c \
         JOIN challenge_steps s ON s.id = c.step_id \
         WHERE s.challenge_id = ?1 ORDER BY c.step_id, c.position",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;

    let evidence_rows = sqlx::query_as::<_, EvidenceRow>(
        "SELECT evidence_key, title, content FROM challenge_evidence \
         WHERE challenge_id = ?1 AND kind = 'packet_summary' ORDER BY position",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;

    let steps = step_rows
        .into_iter()
        .map(|s| Step {
            hints: hint_rows
                .iter()
                .filter(|h| h.step_id == s.id)
                .map(|h| Hint {
                    position: h.position,
                    body_md: h.body_md.clone(),
                })
                .collect(),
            choices: choice_rows
                .iter()
                .filter(|c| c.step_id == s.id)
                .map(|c| Choice {
                    id: c.id,
                    position: c.position,
                    label: c.label.clone(),
                })
                .collect(),
            id: s.id,
            step_key: s.step_key,
            position: s.position,
            kind: s.kind,
            title: s.title,
            instructions_md: s.instructions_md,
            completion_criteria: s.completion_criteria,
        })
        .collect();

    let evidence = evidence_rows
        .into_iter()
        .map(|e| {
            let summary: PacketSummary = serde_json::from_str(&e.content).map_err(|err| {
                AppError::Content(format!("evidence '{}': {err}", e.evidence_key))
            })?;
            Ok(Evidence {
                evidence_key: e.evidence_key,
                title: e.title,
                packets: summary.packets,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(Some(ChallengeDetail {
        id: row.id,
        slug: row.slug,
        version: row.version,
        source: row.source,
        title: row.title,
        summary: row.summary,
        difficulty: row.difficulty,
        objectives_md: row.objectives_md,
        steps,
        evidence,
    }))
}

pub async fn find_id(
    conn: &mut SqliteConnection,
    slug: &str,
    version: i64,
) -> AppResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM challenges WHERE slug = ?1 AND version = ?2")
            .bind(slug)
            .bind(version)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn slug_exists(conn: &mut SqliteConnection, slug: &str) -> AppResult<bool> {
    let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM challenges WHERE slug = ?1 LIMIT 1")
        .bind(slug)
        .fetch_optional(conn)
        .await?;
    Ok(found.is_some())
}

pub async fn source_of(conn: &mut SqliteConnection, id: i64) -> AppResult<Option<ChallengeSource>> {
    Ok(
        sqlx::query_scalar("SELECT source FROM challenges WHERE id = ?1")
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn attempt_count(conn: &mut SqliteConnection, challenge_id: i64) -> AppResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM attempts WHERE challenge_id = ?1")
            .bind(challenge_id)
            .fetch_one(conn)
            .await?,
    )
}

/// Insert a challenge row plus its steps, hints, choices and evidence.
pub async fn insert(
    conn: &mut SqliteConnection,
    source: ChallengeSource,
    slug: &str,
    version: i64,
    draft: &ChallengeDraft,
) -> AppResult<i64> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO challenges (slug, version, source, title, summary, difficulty, objectives_md) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
    )
    .bind(slug)
    .bind(version)
    .bind(source)
    .bind(&draft.title)
    .bind(&draft.summary)
    .bind(draft.difficulty)
    .bind(&draft.objectives_md)
    .fetch_one(&mut *conn)
    .await?;
    insert_children(conn, id, draft).await?;
    Ok(id)
}

/// Update a challenge's own columns and replace all of its steps and evidence.
/// Callers must have established that no attempt references this challenge.
pub async fn replace_content(
    conn: &mut SqliteConnection,
    id: i64,
    draft: &ChallengeDraft,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE challenges SET title = ?2, summary = ?3, difficulty = ?4, objectives_md = ?5, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
    )
    .bind(id)
    .bind(&draft.title)
    .bind(&draft.summary)
    .bind(draft.difficulty)
    .bind(&draft.objectives_md)
    .execute(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM challenge_steps WHERE challenge_id = ?1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM challenge_evidence WHERE challenge_id = ?1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    insert_children(conn, id, draft).await
}

async fn insert_children(
    conn: &mut SqliteConnection,
    id: i64,
    draft: &ChallengeDraft,
) -> AppResult<()> {
    for (i, step) in draft.steps.iter().enumerate() {
        let step_id: i64 = sqlx::query_scalar(
            "INSERT INTO challenge_steps \
             (challenge_id, step_key, position, kind, title, instructions_md, completion_criteria) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
        )
        .bind(id)
        .bind(&step.step_key)
        .bind(i as i64 + 1)
        .bind(step.kind)
        .bind(&step.title)
        .bind(&step.instructions_md)
        .bind(&step.completion_criteria)
        .fetch_one(&mut *conn)
        .await?;

        for (h, hint) in step.hints.iter().enumerate() {
            sqlx::query("INSERT INTO step_hints (step_id, position, body_md) VALUES (?1, ?2, ?3)")
                .bind(step_id)
                .bind(h as i64 + 1)
                .bind(hint)
                .execute(&mut *conn)
                .await?;
        }
        for (c, choice) in step.choices.iter().enumerate() {
            sqlx::query(
                "INSERT INTO step_choices (step_id, position, label, is_correct, explanation) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(step_id)
            .bind(c as i64 + 1)
            .bind(&choice.label)
            .bind(choice.is_correct)
            .bind(&choice.explanation)
            .execute(&mut *conn)
            .await?;
        }
    }

    for (e, evidence) in draft.evidence.iter().enumerate() {
        let content = serde_json::to_string(&PacketSummary {
            packets: evidence.packets.clone(),
        })
        .map_err(|err| AppError::Content(err.to_string()))?;
        sqlx::query(
            "INSERT INTO challenge_evidence (challenge_id, evidence_key, position, kind, title, content) \
             VALUES (?1, ?2, ?3, 'packet_summary', ?4, ?5)",
        )
        .bind(id)
        .bind(&evidence.evidence_key)
        .bind(e as i64 + 1)
        .bind(&evidence.title)
        .bind(content)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn delete(conn: &mut SqliteConnection, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM challenges WHERE id = ?1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
