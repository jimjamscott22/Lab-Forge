//! User progress: attempts, per-step state, notes, and the dashboard roll-up.
//! Rows here reference challenge content by id but never copy or modify it.

use serde::{Deserialize, Serialize};

use super::challenge::{AttemptStatus, ChallengeDetail};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub id: i64,
    pub challenge_id: i64,
    pub status: AttemptStatus,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum StepStatus {
    NotStarted,
    Completed,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AttemptStep {
    pub step_id: i64,
    pub status: StepStatus,
    pub completed_at: Option<String>,
    pub selected_choice_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: i64,
    pub attempt_id: i64,
    pub body_md: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Everything the exercise workspace needs in one round trip.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptView {
    pub attempt: Attempt,
    pub challenge: ChallengeDetail,
    pub steps: Vec<AttemptStep>,
    pub note: Option<Note>,
}

/// Result of submitting a step. `correct` is false only for a wrong knowledge-check
/// answer, in which case nothing was written.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutcome {
    pub correct: bool,
    pub explanation: Option<String>,
    pub attempt: AttemptView,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AttemptCard {
    pub attempt_id: i64,
    pub challenge_id: i64,
    pub challenge_title: String,
    pub status: AttemptStatus,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub steps_completed: i64,
    pub steps_total: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    /// Most recently started attempt that is still open, offered as "Resume".
    pub current: Option<AttemptCard>,
    pub challenges_available: i64,
    pub challenges_completed: i64,
    pub attempts_completed: i64,
    pub recent: Vec<AttemptCard>,
}
