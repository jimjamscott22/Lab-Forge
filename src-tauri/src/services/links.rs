//! Validation for links that would be handed to the system browser.
//!
//! The WebView never opens external URLs itself. Markdown links are routed through
//! one command that only accepts plain `https` URLs.

use url::Url;

use crate::error::{AppError, AppResult};

const MAX_URL_LEN: usize = 2048;

pub fn validate_external_url(raw: &str) -> AppResult<Url> {
    let reject = || AppError::validation("Only https links can be opened.");
    if raw.len() > MAX_URL_LEN || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(reject());
    }
    let url = Url::parse(raw).map_err(|_| reject())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(reject());
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_https() {
        assert!(validate_external_url("https://tauri.app/security/").is_ok());
        assert!(validate_external_url("HTTPS://Example.test/a?b=c#d").is_ok());
    }

    #[test]
    fn opens_the_normalised_url_not_the_raw_input() {
        // The url crate treats extra slashes after a special scheme leniently.
        let url = validate_external_url("https:///docs.example.test/a").unwrap();
        assert_eq!(url.as_str(), "https://docs.example.test/a");
    }

    #[test]
    fn rejects_every_other_scheme_and_malformed_input() {
        for bad in [
            "http://example.test",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,<script>1</script>",
            "ftp://example.test",
            "mailto:a@example.test",
            "vbscript:x",
            "https://user:pw@example.test",
            "//example.test",
            "example.test",
            "https://exa mple.test",
            "https://example.test/\nHeader: x",
            "",
        ] {
            assert!(validate_external_url(bad).is_err(), "should reject {bad:?}");
        }
        assert!(
            validate_external_url(&format!("https://example.test/{}", "a".repeat(3000))).is_err()
        );
    }
}
