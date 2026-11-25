//! Tauri commands: thin, typed adapters between the WebView and the services.
//!
//! Commands hold no business logic. They deserialize structured input, call one
//! service function, and return serializable results or an `AppError`.
//! Every command listed here must also be named in `build.rs` and granted in
//! `capabilities/default.json`; a command missing from either cannot be invoked.

use sqlx::SqlitePool;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::models::challenge::{ChallengeDetail, ChallengeSummary, CustomChallengeInput};
use crate::models::progress::{AttemptView, DashboardSummary, Note, StepOutcome};
use crate::services::{attempts, challenges, links};

pub struct AppState {
    pub pool: SqlitePool,
}

/// Names of every command, kept in one place so `build.rs` and tests can check them.
pub const COMMAND_NAMES: &[&str] = &[
    "list_challenges",
    "get_challenge",
    "create_challenge",
    "update_challenge",
    "delete_challenge",
    "start_attempt",
    "get_attempt",
    "complete_step",
    "save_note",
    "get_dashboard",
    "open_external_link",
];

#[tauri::command]
pub async fn list_challenges(
    state: State<'_, AppState>,
    query: Option<String>,
) -> AppResult<Vec<ChallengeSummary>> {
    challenges::list(&state.pool, query.as_deref()).await
}

#[tauri::command]
pub async fn get_challenge(
    state: State<'_, AppState>,
    challenge_id: i64,
) -> AppResult<ChallengeDetail> {
    challenges::get(&state.pool, challenge_id).await
}

#[tauri::command]
pub async fn create_challenge(
    state: State<'_, AppState>,
    input: CustomChallengeInput,
) -> AppResult<ChallengeDetail> {
    challenges::create_custom(&state.pool, input).await
}

#[tauri::command]
pub async fn update_challenge(
    state: State<'_, AppState>,
    challenge_id: i64,
    input: CustomChallengeInput,
) -> AppResult<ChallengeDetail> {
    challenges::update_custom(&state.pool, challenge_id, input).await
}

#[tauri::command]
pub async fn delete_challenge(state: State<'_, AppState>, challenge_id: i64) -> AppResult<()> {
    challenges::delete_custom(&state.pool, challenge_id).await
}

#[tauri::command]
pub async fn start_attempt(
    state: State<'_, AppState>,
    challenge_id: i64,
) -> AppResult<AttemptView> {
    attempts::start(&state.pool, challenge_id).await
}

#[tauri::command]
pub async fn get_attempt(state: State<'_, AppState>, attempt_id: i64) -> AppResult<AttemptView> {
    attempts::get(&state.pool, attempt_id).await
}

#[tauri::command]
pub async fn complete_step(
    state: State<'_, AppState>,
    attempt_id: i64,
    step_id: i64,
    choice_id: Option<i64>,
) -> AppResult<StepOutcome> {
    attempts::complete_step(&state.pool, attempt_id, step_id, choice_id).await
}

#[tauri::command]
pub async fn save_note(
    state: State<'_, AppState>,
    attempt_id: i64,
    body_md: String,
) -> AppResult<Note> {
    attempts::save_note(&state.pool, attempt_id, &body_md).await
}

#[tauri::command]
pub async fn get_dashboard(state: State<'_, AppState>) -> AppResult<DashboardSummary> {
    attempts::dashboard(&state.pool).await
}

/// Open a validated `https` link in the system browser. This is the only way the
/// frontend can open an external URL; the opener plugin itself is not exposed to it.
#[tauri::command]
pub async fn open_external_link(app: AppHandle, url: String) -> AppResult<()> {
    let url = links::validate_external_url(&url)?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))
}

#[cfg(test)]
mod tests {
    use super::COMMAND_NAMES;
    use std::collections::BTreeSet;

    const CAPABILITY: &str = include_str!("../../capabilities/default.json");
    const BUILD_RS: &str = include_str!("../../build.rs");
    const LIB_RS: &str = include_str!("../lib.rs");

    /// A command missing from any of these places fails only at runtime in the native
    /// app ("command not allowed"), so check that the three lists agree.
    #[test]
    fn commands_are_registered_manifested_and_granted_consistently() {
        let capability: serde_json::Value = serde_json::from_str(CAPABILITY).unwrap();
        let granted: BTreeSet<String> = capability["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_owned())
            .collect();
        let expected: BTreeSet<String> = COMMAND_NAMES
            .iter()
            .map(|n| format!("allow-{}", n.replace('_', "-")))
            .collect();
        assert_eq!(
            granted, expected,
            "capability must grant exactly the app commands and nothing else"
        );
        assert_eq!(capability["windows"], serde_json::json!(["main"]));

        for name in COMMAND_NAMES {
            assert!(
                BUILD_RS.contains(&format!("\"{name}\"")),
                "{name} missing from build.rs"
            );
            assert!(
                LIB_RS.contains(&format!("commands::{name},")),
                "{name} missing from invoke_handler"
            );
        }
    }
}
