use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Every failure a command can return.
///
/// The frontend receives only `{ code, message }`. Validation, not-found and
/// conflict messages are written for the learner and are safe to show. Internal
/// failures (database, migration, I/O, bad bundled content) are logged here and
/// reach the UI as a generic message so paths and SQL never cross the IPC boundary.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0} was not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("storage error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid bundled content: {0}")]
    Content(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    /// Stable machine-readable code the UI can branch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Database(_) | Self::Migration(_) | Self::Io(_) | Self::Content(_) => "internal",
        }
    }

    /// Text that is safe to show in the UI.
    pub fn public_message(&self) -> String {
        match self {
            Self::Validation(_) | Self::NotFound(_) | Self::Conflict(_) => self.to_string(),
            _ => "LabForge could not read or save its data. Nothing was changed. \
                  Try again, and restart the app if it keeps happening."
                .to_string(),
        }
    }

    /// Whether a SQLx error is a UNIQUE/PRIMARY KEY violation.
    pub fn is_unique_violation(error: &sqlx::Error) -> bool {
        error
            .as_database_error()
            .is_some_and(|e| e.is_unique_violation())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.code() == "internal" {
            eprintln!("labforge: internal error: {self}");
        }
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.public_message())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_errors_do_not_leak_details() {
        let error = AppError::Content("secret/path/file.json line 3".into());
        assert_eq!(error.code(), "internal");
        assert!(!error.public_message().contains("secret"));
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "internal");
        assert!(!json["message"].as_str().unwrap().contains("secret"));
    }

    #[test]
    fn learner_facing_errors_keep_their_message() {
        let json = serde_json::to_value(AppError::conflict("Already open")).unwrap();
        assert_eq!(json["code"], "conflict");
        assert_eq!(json["message"], "Already open");
    }
}
