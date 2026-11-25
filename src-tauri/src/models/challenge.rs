//! Challenge *content*: definitions that ship with the app or that the learner authors.
//! Nothing in this file records user progress.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum ChallengeSource {
    Bundled,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum StepKind {
    /// Learner confirms they did the work.
    Task,
    /// Knowledge check answered from a fixed set of choices.
    Check,
    /// Completes only once the attempt's note has content.
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum AttemptStatus {
    InProgress,
    Completed,
}

// ---------------------------------------------------------------------------
// Read models (sent to the frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeSummary {
    pub id: i64,
    pub slug: String,
    pub version: i64,
    pub source: ChallengeSource,
    pub title: String,
    pub summary: String,
    pub difficulty: Difficulty,
    pub step_count: i64,
    pub attempt_count: i64,
    /// Status of the most recent attempt on any version of this challenge.
    pub latest_attempt_status: Option<AttemptStatus>,
    pub latest_attempt_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeDetail {
    pub id: i64,
    pub slug: String,
    pub version: i64,
    pub source: ChallengeSource,
    pub title: String,
    pub summary: String,
    pub difficulty: Difficulty,
    pub objectives_md: String,
    pub steps: Vec<Step>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: i64,
    pub step_key: String,
    pub position: i64,
    pub kind: StepKind,
    pub title: String,
    pub instructions_md: String,
    pub completion_criteria: String,
    pub hints: Vec<Hint>,
    /// Never includes which choice is correct; that stays in Rust.
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Hint {
    pub position: i64,
    pub body_md: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: i64,
    pub position: i64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub evidence_key: String,
    pub title: String,
    pub packets: Vec<Packet>,
}

/// One row of a packet summary. Hosts and addresses in bundled content are fictional.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Packet {
    pub no: u32,
    pub offset_ms: u32,
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub length: u32,
    pub info: String,
}

/// JSON payload stored in `challenge_evidence.content` for kind `packet_summary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PacketSummary {
    pub packets: Vec<Packet>,
}

// ---------------------------------------------------------------------------
// Write models (definitions that get inserted)
// ---------------------------------------------------------------------------

/// Everything needed to insert a challenge version, shared by the bundled seeder
/// and by custom challenges so both pass through one validator and one insert path.
// No `deny_unknown_fields` here: serde cannot combine it with the `flatten` used by
// `BundledChallenge`. A unit test parses every bundled file instead.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeDraft {
    pub title: String,
    pub summary: String,
    pub difficulty: Difficulty,
    pub objectives_md: String,
    pub steps: Vec<StepDraft>,
    #[serde(default)]
    pub evidence: Vec<EvidenceDraft>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepDraft {
    pub step_key: String,
    pub kind: StepKind,
    pub title: String,
    pub instructions_md: String,
    pub completion_criteria: String,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub choices: Vec<ChoiceDraft>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChoiceDraft {
    pub label: String,
    pub is_correct: bool,
    pub explanation: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceDraft {
    pub evidence_key: String,
    pub title: String,
    pub packets: Vec<Packet>,
}

/// A bundled challenge file: identity plus draft content.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledChallenge {
    pub slug: String,
    pub version: i64,
    #[serde(flatten)]
    pub draft: ChallengeDraft,
}

/// Input the frontend sends to create or replace a custom challenge.
/// Deliberately smaller than `ChallengeDraft`: custom challenges are plain
/// task lists, and slug/version/source are decided in Rust.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomChallengeInput {
    pub title: String,
    pub summary: String,
    pub difficulty: Difficulty,
    pub objectives_md: String,
    pub steps: Vec<CustomStepInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomStepInput {
    pub title: String,
    pub instructions_md: String,
    pub completion_criteria: String,
    #[serde(default)]
    pub hint: Option<String>,
}
