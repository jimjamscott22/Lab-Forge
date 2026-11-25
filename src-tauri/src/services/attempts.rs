//! Attempt use cases: start, resume, complete steps, save notes, dashboard.
//!
//! Rules enforced here (and nowhere in the UI):
//! - a challenge has at most one open attempt; retries are new attempts
//! - steps complete strictly in order
//! - a knowledge check only completes on the correct choice
//! - a note step only completes once the attempt's note has content
//! - completing the final step and completing the attempt commit together

use sqlx::SqlitePool;

use super::validation::validate_note;
use crate::error::{AppError, AppResult};
use crate::models::challenge::{AttemptStatus, StepKind};
use crate::models::progress::*;
use crate::repositories::{challenges as challenge_repo, progress as repo};

const RECENT_ATTEMPTS: i64 = 5;

pub async fn start(pool: &SqlitePool, challenge_id: i64) -> AppResult<AttemptView> {
    let mut tx = pool.begin().await?;
    if challenge_repo::source_of(&mut tx, challenge_id)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound("Challenge"));
    }
    let open_message = "This challenge already has an attempt in progress. Resume it instead.";
    if repo::open_attempt_id(&mut tx, challenge_id)
        .await?
        .is_some()
    {
        return Err(AppError::conflict(open_message));
    }
    let attempt_id = match repo::create_attempt(&mut tx, challenge_id).await {
        Ok(id) => id,
        // The partial unique index is the backstop if two starts race.
        Err(AppError::Database(e)) if AppError::is_unique_violation(&e) => {
            return Err(AppError::conflict(open_message));
        }
        Err(e) => return Err(e),
    };
    tx.commit().await?;
    get(pool, attempt_id).await
}

pub async fn get(pool: &SqlitePool, attempt_id: i64) -> AppResult<AttemptView> {
    let mut conn = pool.acquire().await?;
    let attempt = repo::find_attempt(&mut conn, attempt_id)
        .await?
        .ok_or(AppError::NotFound("Attempt"))?;
    let challenge = challenge_repo::get_detail(&mut conn, attempt.challenge_id)
        .await?
        .ok_or(AppError::NotFound("Challenge"))?;
    let steps = repo::attempt_steps(&mut conn, attempt_id).await?;
    let note = repo::get_note(&mut conn, attempt_id).await?;
    Ok(AttemptView {
        attempt,
        challenge,
        steps,
        note,
    })
}

pub async fn complete_step(
    pool: &SqlitePool,
    attempt_id: i64,
    step_id: i64,
    choice_id: Option<i64>,
) -> AppResult<StepOutcome> {
    let mut tx = pool.begin().await?;

    let attempt = repo::find_attempt(&mut tx, attempt_id)
        .await?
        .ok_or(AppError::NotFound("Attempt"))?;
    if attempt.status == AttemptStatus::Completed {
        return Err(AppError::conflict(
            "This attempt is already finished. Start a new attempt to try again.",
        ));
    }
    let kind = repo::step_kind_in_attempt(&mut tx, attempt_id, step_id)
        .await?
        .ok_or(AppError::NotFound("Step"))?;

    let steps = repo::attempt_steps(&mut tx, attempt_id).await?;
    if steps
        .iter()
        .any(|s| s.step_id == step_id && s.status == StepStatus::Completed)
    {
        return Err(AppError::conflict("This step is already complete."));
    }
    if repo::next_open_step(&mut tx, attempt_id).await? != Some(step_id) {
        return Err(AppError::conflict("Complete the earlier steps first."));
    }

    let mut explanation = None;
    match kind {
        StepKind::Task => {
            if choice_id.is_some() {
                return Err(AppError::validation("This step doesn't take an answer."));
            }
        }
        StepKind::Check => {
            let choice_id =
                choice_id.ok_or_else(|| AppError::validation("Choose an answer first."))?;
            let check = repo::choice_check(&mut tx, step_id, choice_id)
                .await?
                .ok_or_else(|| {
                    AppError::validation("That answer isn't one of the choices for this step.")
                })?;
            explanation = Some(check.explanation);
            if !check.is_correct {
                // Nothing is written for a wrong answer; the learner can try again.
                drop(tx);
                return Ok(StepOutcome {
                    correct: false,
                    explanation,
                    attempt: get(pool, attempt_id).await?,
                });
            }
        }
        StepKind::Note => {
            if choice_id.is_some() {
                return Err(AppError::validation("This step doesn't take an answer."));
            }
            let has_content = repo::get_note(&mut tx, attempt_id)
                .await?
                .is_some_and(|n| !n.body_md.trim().is_empty());
            if !has_content {
                return Err(AppError::validation(
                    "Write your finding in the notebook before completing this step.",
                ));
            }
        }
    }

    if repo::mark_step_completed(&mut tx, attempt_id, step_id, choice_id).await? != 1 {
        return Err(AppError::conflict("This step is already complete."));
    }
    if repo::next_open_step(&mut tx, attempt_id).await?.is_none() {
        repo::complete_attempt(&mut tx, attempt_id).await?;
    }
    tx.commit().await?;

    Ok(StepOutcome {
        correct: true,
        explanation,
        attempt: get(pool, attempt_id).await?,
    })
}

pub async fn save_note(pool: &SqlitePool, attempt_id: i64, body_md: &str) -> AppResult<Note> {
    validate_note(body_md)?;
    let mut tx = pool.begin().await?;
    if repo::find_attempt(&mut tx, attempt_id).await?.is_none() {
        return Err(AppError::NotFound("Attempt"));
    }
    let note = repo::upsert_note(&mut tx, attempt_id, body_md).await?;
    tx.commit().await?;
    Ok(note)
}

pub async fn dashboard(pool: &SqlitePool) -> AppResult<DashboardSummary> {
    let mut conn = pool.acquire().await?;
    Ok(DashboardSummary {
        current: repo::current_card(&mut conn).await?,
        challenges_available: repo::count_available_challenges(&mut conn).await?,
        challenges_completed: repo::count_completed_challenges(&mut conn).await?,
        attempts_completed: repo::count_attempts_with_status(&mut conn, AttemptStatus::Completed)
            .await?,
        recent: repo::recent_cards(&mut conn, RECENT_ATTEMPTS).await?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{open_in_dir, test_support::fresh};
    use crate::services::challenges::{list, seed_bundled};

    /// Seeds the bundled exercise and returns its challenge id.
    async fn setup(pool: &SqlitePool) -> i64 {
        seed_bundled(pool).await.unwrap();
        list(pool, None).await.unwrap()[0].id
    }

    fn step_ids(view: &AttemptView) -> Vec<i64> {
        view.challenge.steps.iter().map(|s| s.id).collect()
    }

    fn correct_choice(view: &AttemptView) -> i64 {
        // Position 1 is the correct choice in the bundled content.
        view.challenge.steps[2].choices[0].id
    }

    /// Completes every step that is still open, in order, and returns the final view.
    async fn finish(pool: &SqlitePool, attempt_id: i64) -> AttemptView {
        let view = get(pool, attempt_id).await.unwrap();
        for (step, progress) in view.challenge.steps.iter().zip(&view.steps) {
            if progress.status == StepStatus::Completed {
                continue;
            }
            match step.kind {
                StepKind::Task => {
                    complete_step(pool, attempt_id, step.id, None)
                        .await
                        .unwrap();
                }
                StepKind::Check => {
                    complete_step(pool, attempt_id, step.id, Some(correct_choice(&view)))
                        .await
                        .unwrap();
                }
                StepKind::Note => {
                    save_note(pool, attempt_id, "Packet 4 retried packet 3.")
                        .await
                        .unwrap();
                    complete_step(pool, attempt_id, step.id, None)
                        .await
                        .unwrap();
                }
            }
        }
        get(pool, attempt_id).await.unwrap()
    }

    #[tokio::test]
    async fn full_walkthrough_completes_the_attempt_in_one_transaction() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let started = start(&db.pool, cid).await.unwrap();
        assert_eq!(started.attempt.status, AttemptStatus::InProgress);
        assert_eq!(started.steps.len(), 4);

        let done = finish(&db.pool, started.attempt.id).await;
        assert_eq!(done.attempt.status, AttemptStatus::Completed);
        assert!(done.attempt.completed_at.is_some());
        assert!(done.steps.iter().all(|s| s.status == StepStatus::Completed));
        assert_eq!(done.note.unwrap().body_md, "Packet 4 retried packet 3.");
    }

    #[tokio::test]
    async fn steps_must_complete_in_order() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let view = start(&db.pool, cid).await.unwrap();
        let ids = step_ids(&view);

        let skipped = complete_step(&db.pool, view.attempt.id, ids[1], None).await;
        assert!(matches!(skipped, Err(AppError::Conflict(_))));

        complete_step(&db.pool, view.attempt.id, ids[0], None)
            .await
            .unwrap();
        let repeat = complete_step(&db.pool, view.attempt.id, ids[0], None).await;
        match repeat {
            Err(AppError::Conflict(m)) => assert!(m.contains("already complete"), "{m}"),
            other => panic!("expected conflict, got {other:?}"),
        }

        let after = get(&db.pool, view.attempt.id).await.unwrap();
        assert_eq!(
            after
                .steps
                .iter()
                .filter(|s| s.status == StepStatus::Completed)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn wrong_answer_writes_nothing_and_correct_answer_advances() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let view = start(&db.pool, cid).await.unwrap();
        let ids = step_ids(&view);
        complete_step(&db.pool, view.attempt.id, ids[0], None)
            .await
            .unwrap();
        complete_step(&db.pool, view.attempt.id, ids[1], None)
            .await
            .unwrap();

        let wrong_choice = view.challenge.steps[2].choices[1].id;
        let wrong = complete_step(&db.pool, view.attempt.id, ids[2], Some(wrong_choice))
            .await
            .unwrap();
        assert!(!wrong.correct);
        assert!(wrong.explanation.is_some());
        assert_eq!(
            wrong.attempt.steps[2].status,
            StepStatus::NotStarted,
            "a wrong answer leaves the step open"
        );

        let right = complete_step(
            &db.pool,
            view.attempt.id,
            ids[2],
            Some(correct_choice(&view)),
        )
        .await
        .unwrap();
        assert!(right.correct);
        assert_eq!(right.attempt.steps[2].status, StepStatus::Completed);
        assert_eq!(
            right.attempt.steps[2].selected_choice_id,
            Some(correct_choice(&view))
        );
    }

    #[tokio::test]
    async fn knowledge_check_rejects_missing_foreign_and_unexpected_choices() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let view = start(&db.pool, cid).await.unwrap();
        let ids = step_ids(&view);
        complete_step(&db.pool, view.attempt.id, ids[0], None)
            .await
            .unwrap();
        complete_step(&db.pool, view.attempt.id, ids[1], None)
            .await
            .unwrap();

        let none = complete_step(&db.pool, view.attempt.id, ids[2], None).await;
        assert!(matches!(none, Err(AppError::Validation(_))));
        let bogus = complete_step(&db.pool, view.attempt.id, ids[2], Some(987_654)).await;
        assert!(matches!(bogus, Err(AppError::Validation(_))));

        // A choice id that belongs to a different step is not accepted either.
        let foreign =
            sqlx::query_scalar::<_, i64>("SELECT id FROM step_choices WHERE step_id != ?1 LIMIT 1")
                .bind(ids[2])
                .fetch_optional(&db.pool)
                .await
                .unwrap();
        if let Some(foreign) = foreign {
            let r = complete_step(&db.pool, view.attempt.id, ids[2], Some(foreign)).await;
            assert!(matches!(r, Err(AppError::Validation(_))));
        }
    }

    #[tokio::test]
    async fn note_step_requires_a_non_blank_note() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let view = start(&db.pool, cid).await.unwrap();
        let id = view.attempt.id;
        let ids = step_ids(&view);
        complete_step(&db.pool, id, ids[0], None).await.unwrap();
        complete_step(&db.pool, id, ids[1], None).await.unwrap();
        complete_step(&db.pool, id, ids[2], Some(correct_choice(&view)))
            .await
            .unwrap();

        assert!(matches!(
            complete_step(&db.pool, id, ids[3], None).await,
            Err(AppError::Validation(_))
        ));
        save_note(&db.pool, id, "  \n\t ").await.unwrap();
        assert!(matches!(
            complete_step(&db.pool, id, ids[3], None).await,
            Err(AppError::Validation(_))
        ));
        assert_eq!(
            get(&db.pool, id).await.unwrap().attempt.status,
            AttemptStatus::InProgress,
            "failed completion leaves the attempt open"
        );
        save_note(&db.pool, id, "Real finding").await.unwrap();
        let done = complete_step(&db.pool, id, ids[3], None).await.unwrap();
        assert_eq!(done.attempt.attempt.status, AttemptStatus::Completed);
    }

    #[tokio::test]
    async fn finished_attempt_rejects_further_steps_and_a_retry_is_independent() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let first = start(&db.pool, cid).await.unwrap();
        let done = finish(&db.pool, first.attempt.id).await;
        let ids = step_ids(&done);

        let again = complete_step(&db.pool, first.attempt.id, ids[0], None).await;
        assert!(matches!(again, Err(AppError::Conflict(_))));

        let second = start(&db.pool, cid).await.unwrap();
        assert_ne!(second.attempt.id, first.attempt.id);
        assert_eq!(second.attempt.status, AttemptStatus::InProgress);
        assert!(
            second
                .steps
                .iter()
                .all(|s| s.status == StepStatus::NotStarted)
        );
        assert!(
            second.note.is_none(),
            "a retry starts with a fresh notebook"
        );

        // The first attempt's history is untouched.
        let first_again = get(&db.pool, first.attempt.id).await.unwrap();
        assert_eq!(first_again.attempt.status, AttemptStatus::Completed);
        assert_eq!(
            first_again.note.unwrap().body_md,
            "Packet 4 retried packet 3."
        );
    }

    #[tokio::test]
    async fn two_open_attempts_on_one_challenge_are_refused() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        start(&db.pool, cid).await.unwrap();
        assert!(matches!(
            start(&db.pool, cid).await,
            Err(AppError::Conflict(_))
        ));

        // The database refuses it too, independent of the service check.
        let raw =
            sqlx::query("INSERT INTO attempts (challenge_id, status) VALUES (?1, 'in_progress')")
                .bind(cid)
                .execute(&db.pool)
                .await;
        assert!(raw.is_err());
        assert!(matches!(
            start(&db.pool, 4242).await,
            Err(AppError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn notes_upsert_keeps_created_at_and_rejects_oversize() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let id = start(&db.pool, cid).await.unwrap().attempt.id;

        let first = save_note(&db.pool, id, "one").await.unwrap();
        let second = save_note(&db.pool, id, "two").await.unwrap();
        assert_eq!(first.id, second.id, "one note per attempt");
        assert_eq!(first.created_at, second.created_at);
        assert_eq!(second.body_md, "two");

        let too_big = "x".repeat(super::super::validation::MAX_NOTE_CHARS + 1);
        assert!(matches!(
            save_note(&db.pool, id, &too_big).await,
            Err(AppError::Validation(_))
        ));
        assert_eq!(
            get(&db.pool, id).await.unwrap().note.unwrap().body_md,
            "two"
        );
        assert!(matches!(
            save_note(&db.pool, 31337, "x").await,
            Err(AppError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn dashboard_reports_resume_target_and_progress() {
        let db = fresh().await;
        let empty = dashboard(&db.pool).await.unwrap();
        assert!(empty.current.is_none() && empty.recent.is_empty());

        let cid = setup(&db.pool).await;
        let view = start(&db.pool, cid).await.unwrap();
        complete_step(&db.pool, view.attempt.id, step_ids(&view)[0], None)
            .await
            .unwrap();

        let mid = dashboard(&db.pool).await.unwrap();
        let current = mid.current.expect("open attempt is offered for resume");
        assert_eq!((current.steps_completed, current.steps_total), (1, 4));
        assert_eq!(mid.challenges_completed, 0);

        finish(&db.pool, view.attempt.id).await;
        let end = dashboard(&db.pool).await.unwrap();
        assert!(end.current.is_none());
        assert_eq!(
            (
                end.challenges_completed,
                end.attempts_completed,
                end.challenges_available
            ),
            (1, 1, 1)
        );
        assert_eq!(end.recent[0].status, AttemptStatus::Completed);
    }

    /// The Milestone 1 acceptance check at the data layer: close the database as an app
    /// quit would, reopen the same file, and find progress and the note intact.
    #[tokio::test]
    async fn progress_and_note_survive_closing_and_reopening_the_database() {
        let db = fresh().await;
        let cid = setup(&db.pool).await;
        let view = start(&db.pool, cid).await.unwrap();
        let id = view.attempt.id;
        complete_step(&db.pool, id, step_ids(&view)[0], None)
            .await
            .unwrap();
        save_note(&db.pool, id, "10.20.0.15 is the client")
            .await
            .unwrap();
        db.pool.close().await;

        let reopened = open_in_dir(db.dir.path()).await.unwrap();
        assert_eq!(
            seed_bundled(&reopened).await.unwrap(),
            0,
            "relaunch does not reseed"
        );
        let restored = get(&reopened, id).await.unwrap();
        assert_eq!(restored.attempt.status, AttemptStatus::InProgress);
        assert_eq!(restored.steps[0].status, StepStatus::Completed);
        assert_eq!(restored.steps[1].status, StepStatus::NotStarted);
        assert_eq!(restored.note.unwrap().body_md, "10.20.0.15 is the client");

        let current = dashboard(&reopened).await.unwrap().current.unwrap();
        assert_eq!(current.attempt_id, id);
    }
}
