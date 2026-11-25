//! Challenge use cases: library queries, custom challenge CRUD, bundled-content seeding.

use sqlx::SqlitePool;

use super::validation::{custom_to_draft, is_valid_key, normalize, validate_draft};
use crate::content;
use crate::error::{AppError, AppResult};
use crate::models::challenge::*;
use crate::repositories::challenges as repo;

pub async fn list(pool: &SqlitePool, query: Option<&str>) -> AppResult<Vec<ChallengeSummary>> {
    let mut conn = pool.acquire().await?;
    repo::list(&mut conn, query).await
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<ChallengeDetail> {
    let mut conn = pool.acquire().await?;
    repo::get_detail(&mut conn, id)
        .await?
        .ok_or(AppError::NotFound("Challenge"))
}

pub async fn create_custom(
    pool: &SqlitePool,
    input: CustomChallengeInput,
) -> AppResult<ChallengeDetail> {
    let draft = custom_to_draft(input);
    validate_draft(&draft)?;

    let mut tx = pool.begin().await?;
    let slug = unique_slug(&mut tx, &draft.title).await?;
    let id = repo::insert(&mut tx, ChallengeSource::Custom, &slug, 1, &draft).await?;
    tx.commit().await?;
    get(pool, id).await
}

/// Replace a custom challenge's content. Only allowed while no attempt references it,
/// because attempts must keep seeing the definition they were started against.
pub async fn update_custom(
    pool: &SqlitePool,
    id: i64,
    input: CustomChallengeInput,
) -> AppResult<ChallengeDetail> {
    let draft = custom_to_draft(input);
    validate_draft(&draft)?;

    let mut tx = pool.begin().await?;
    ensure_editable(&mut tx, id).await?;
    repo::replace_content(&mut tx, id, &draft).await?;
    tx.commit().await?;
    get(pool, id).await
}

pub async fn delete_custom(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    ensure_editable(&mut tx, id).await?;
    repo::delete(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

async fn ensure_editable(conn: &mut sqlx::SqliteConnection, id: i64) -> AppResult<()> {
    match repo::source_of(conn, id).await? {
        None => Err(AppError::NotFound("Challenge")),
        Some(ChallengeSource::Bundled) => {
            Err(AppError::conflict("Bundled challenges are read-only."))
        }
        Some(ChallengeSource::Custom) => {
            if repo::attempt_count(conn, id).await? > 0 {
                Err(AppError::conflict(
                    "This challenge already has attempts, so it can't be changed or deleted. \
                     Create a new challenge instead.",
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed: String = out.trim_matches('-').chars().take(48).collect();
    let trimmed = trimmed.trim_end_matches('-');
    if trimmed.is_empty() {
        "challenge".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `custom-` prefix keeps custom slugs out of the namespace bundled content uses.
async fn unique_slug(conn: &mut sqlx::SqliteConnection, title: &str) -> AppResult<String> {
    let base = format!("custom-{}", slugify(title));
    let mut candidate = base.clone();
    let mut n = 2;
    while repo::slug_exists(conn, &candidate).await? {
        candidate = format!("{base}-{n}");
        n += 1;
    }
    Ok(candidate)
}

/// Insert any bundled `(slug, version)` that is not present yet. Returns how many
/// were inserted, so repeated launches report 0 and never duplicate records.
/// Older versions stay in place because saved attempts may reference them.
pub async fn seed_bundled(pool: &SqlitePool) -> AppResult<usize> {
    let mut inserted = 0;
    for bundled in content::bundled_challenges()? {
        if !is_valid_key(&bundled.slug) || bundled.version < 1 {
            return Err(AppError::Content(format!(
                "bad identity for '{}'",
                bundled.slug
            )));
        }
        let draft = normalize(bundled.draft);
        validate_draft(&draft).map_err(|e| AppError::Content(format!("{}: {e}", bundled.slug)))?;

        let mut tx = pool.begin().await?;
        if repo::find_id(&mut tx, &bundled.slug, bundled.version)
            .await?
            .is_some()
        {
            continue;
        }
        repo::insert(
            &mut tx,
            ChallengeSource::Bundled,
            &bundled.slug,
            bundled.version,
            &draft,
        )
        .await?;
        tx.commit().await?;
        inserted += 1;
    }
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_support::fresh;
    use crate::models::progress::StepStatus;
    use crate::services::attempts;

    fn custom_input(title: &str) -> CustomChallengeInput {
        CustomChallengeInput {
            title: title.into(),
            summary: "A summary".into(),
            difficulty: Difficulty::Beginner,
            objectives_md: "Learn".into(),
            steps: vec![CustomStepInput {
                title: "Step".into(),
                instructions_md: "Do".into(),
                completion_criteria: "Done".into(),
                hint: None,
            }],
        }
    }

    #[test]
    fn bundled_content_parses_and_validates() {
        let all = content::bundled_challenges().expect("bundled files parse");
        assert!(!all.is_empty());
        for b in all {
            assert!(is_valid_key(&b.slug), "{}", b.slug);
            validate_draft(&normalize(b.draft)).unwrap_or_else(|e| panic!("{}: {e}", b.slug));
        }
    }

    #[test]
    fn slugify_handles_punctuation_unicode_and_empty() {
        assert_eq!(slugify("  Read a DNS timeout!! "), "read-a-dns-timeout");
        assert_eq!(slugify("日本語"), "challenge");
        assert_eq!(slugify("a--b__c"), "a-b-c");
        assert!(slugify(&"x".repeat(200)).len() <= 48);
    }

    #[tokio::test]
    async fn seeding_twice_does_not_duplicate() {
        let db = fresh().await;
        assert_eq!(seed_bundled(&db.pool).await.unwrap(), 1);
        assert_eq!(seed_bundled(&db.pool).await.unwrap(), 0);
        let (challenges, steps, choices, evidence): (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM challenges), (SELECT COUNT(*) FROM challenge_steps), \
                    (SELECT COUNT(*) FROM step_choices), (SELECT COUNT(*) FROM challenge_evidence)",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!((challenges, steps, choices, evidence), (1, 4, 4, 1));
    }

    #[tokio::test]
    async fn challenge_detail_never_exposes_the_correct_choice() {
        let db = fresh().await;
        seed_bundled(&db.pool).await.unwrap();
        let id = list(&db.pool, None).await.unwrap()[0].id;
        let detail = get(&db.pool, id).await.unwrap();
        let json = serde_json::to_string(&detail).unwrap();
        assert!(!json.contains("isCorrect") && !json.contains("is_correct"));
        assert!(
            !json.contains("explanation"),
            "explanations are only revealed on submit"
        );
        assert_eq!(detail.evidence[0].packets.len(), 5);
    }

    #[tokio::test]
    async fn new_bundled_version_keeps_the_definition_saved_attempts_use() {
        let db = fresh().await;
        seed_bundled(&db.pool).await.unwrap();
        let v1 = list(&db.pool, None).await.unwrap()[0].id;
        let view = attempts::start(&db.pool, v1).await.unwrap();

        // Simulate shipping v2 of the same exercise with a different title.
        let bundled = content::bundled_challenges().unwrap().remove(0);
        let mut draft = normalize(bundled.draft);
        draft.title = "Read a DNS timeout and retry (revised)".into();
        let mut conn = db.pool.acquire().await.unwrap();
        repo::insert(
            &mut conn,
            ChallengeSource::Bundled,
            &bundled.slug,
            2,
            &draft,
        )
        .await
        .unwrap();
        drop(conn);

        // Library shows only the newest version and still reports the old attempt.
        let library = list(&db.pool, None).await.unwrap();
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].version, 2);
        assert_eq!(library[0].latest_attempt_id, Some(view.attempt.id));

        // The saved attempt still resolves to v1 content.
        let reopened = attempts::get(&db.pool, view.attempt.id).await.unwrap();
        assert_eq!(reopened.challenge.version, 1);
        assert_eq!(reopened.challenge.title, "Read a DNS timeout and retry");
        assert!(
            reopened
                .steps
                .iter()
                .all(|s| s.status == StepStatus::NotStarted)
        );
    }

    #[tokio::test]
    async fn search_matches_title_and_summary_and_escapes_wildcards() {
        let db = fresh().await;
        seed_bundled(&db.pool).await.unwrap();
        create_custom(&db.pool, custom_input("Port scanning basics"))
            .await
            .unwrap();

        assert_eq!(list(&db.pool, Some("dns")).await.unwrap().len(), 1);
        assert_eq!(list(&db.pool, Some("SCANNING")).await.unwrap().len(), 1);
        assert_eq!(
            list(&db.pool, Some("a summary")).await.unwrap().len(),
            1,
            "summary match"
        );
        assert_eq!(
            list(&db.pool, Some("   ")).await.unwrap().len(),
            2,
            "blank means all"
        );
        assert!(
            list(&db.pool, Some("%")).await.unwrap().is_empty(),
            "% is literal, not a wildcard"
        );
        assert!(
            list(&db.pool, Some("100%'; DROP TABLE challenges;--"))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            list(&db.pool, None).await.unwrap().len(),
            2,
            "tables intact"
        );
    }

    #[tokio::test]
    async fn custom_challenge_crud_round_trip() {
        let db = fresh().await;
        let created = create_custom(&db.pool, custom_input("My Lab"))
            .await
            .unwrap();
        assert_eq!(created.slug, "custom-my-lab");
        assert_eq!(created.source, ChallengeSource::Custom);
        assert_eq!(created.steps[0].step_key, "step-1");

        let same_title = create_custom(&db.pool, custom_input("My Lab"))
            .await
            .unwrap();
        assert_eq!(same_title.slug, "custom-my-lab-2");

        let mut edit = custom_input("My Lab, renamed");
        edit.steps.push(edit.steps[0].clone());
        let updated = update_custom(&db.pool, created.id, edit).await.unwrap();
        assert_eq!(updated.title, "My Lab, renamed");
        assert_eq!(updated.steps.len(), 2);
        assert_eq!(updated.slug, "custom-my-lab", "slug is stable across edits");

        delete_custom(&db.pool, created.id).await.unwrap();
        assert!(matches!(
            get(&db.pool, created.id).await,
            Err(AppError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn invalid_custom_input_is_rejected_before_touching_the_database() {
        let db = fresh().await;
        let mut bad = custom_input("Ok");
        bad.steps.clear();
        assert!(matches!(
            create_custom(&db.pool, bad).await,
            Err(AppError::Validation(_))
        ));
        let mut blank = custom_input("   ");
        blank.summary = "x".into();
        assert!(matches!(
            create_custom(&db.pool, blank).await,
            Err(AppError::Validation(_))
        ));
        assert!(list(&db.pool, None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn bundled_and_attempted_challenges_cannot_be_changed_or_deleted() {
        let db = fresh().await;
        seed_bundled(&db.pool).await.unwrap();
        let bundled_id = list(&db.pool, None).await.unwrap()[0].id;
        assert!(matches!(
            update_custom(&db.pool, bundled_id, custom_input("Hijack")).await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            delete_custom(&db.pool, bundled_id).await,
            Err(AppError::Conflict(_))
        ));

        let custom = create_custom(&db.pool, custom_input("Attempted"))
            .await
            .unwrap();
        attempts::start(&db.pool, custom.id).await.unwrap();
        assert!(matches!(
            update_custom(&db.pool, custom.id, custom_input("Changed")).await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            delete_custom(&db.pool, custom.id).await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(get(&db.pool, custom.id).await.unwrap().title, "Attempted");
        assert!(matches!(
            delete_custom(&db.pool, 9999).await,
            Err(AppError::NotFound(_))
        ));
    }
}
