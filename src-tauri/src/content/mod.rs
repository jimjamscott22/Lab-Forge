//! Challenge definitions that ship inside the binary.
//!
//! Files live in `src-tauri/content/` under version control and are embedded at
//! compile time. They are seeded by `(slug, version)`: shipping changed content
//! under an existing version has no effect, so authors must bump `version`.

use crate::error::{AppError, AppResult};
use crate::models::challenge::BundledChallenge;

const BUNDLED_FILES: &[(&str, &str)] = &[(
    "dns-timeout-retry.json",
    include_str!("../../content/dns-timeout-retry.json"),
)];

pub fn bundled_challenges() -> AppResult<Vec<BundledChallenge>> {
    BUNDLED_FILES
        .iter()
        .map(|(name, raw)| {
            serde_json::from_str::<BundledChallenge>(raw)
                .map_err(|e| AppError::Content(format!("{name}: {e}")))
        })
        .collect()
}
