//! Credential redaction for log-safe output.
//!
//! Xtream URLs carry the account `username` and `password` as query parameters, and
//! reqwest embeds the full URL in its error `Display`. Both are account PII that must
//! never reach an error message, a captured log line, or a copied bug report. This
//! masks those two parameter values wherever they appear in a string, leaving the host,
//! action, and surrounding text intact so the message is still useful for debugging.

/// Replace the values of `username=` and `password=` query parameters with `***`.
///
/// Idempotent: masking an already-masked string returns it unchanged.
pub fn secrets(s: &str) -> String {
    mask_param(&mask_param(s, "password"), "username")
}

/// Mask the value of every `<key>=…` occurrence, up to the next `&`, whitespace, or
/// closing paren (the `url (...)` wrapper reqwest uses), or the end of the string.
fn mask_param(s: &str, key: &str) -> String {
    let needle = format!("{key}=");
    let mut result = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find(&needle) {
        let value_start = pos + needle.len();
        result.push_str(&rest[..value_start]);
        let value = &rest[value_start..];
        let end = value.find(['&', ' ', ')']).unwrap_or(value.len());
        result.push_str("***");
        rest = &value[end..];
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_username_and_password_in_a_player_api_url() {
        let msg = "network error during xtream request: error sending request for url \
            (http://host:826/player_api.php?username=Kaiser&password=s3cr3t&action=get_live_categories)";
        let out = secrets(msg);
        assert!(out.contains("username=***"));
        assert!(out.contains("password=***"));
        assert!(!out.contains("Kaiser"));
        assert!(!out.contains("s3cr3t"));
        // Non-secret parts survive.
        assert!(out.contains("host:826"));
        assert!(out.contains("action=get_live_categories"));
    }

    #[test]
    fn leaves_clean_messages_untouched() {
        let msg = "storage error during open: no such file";
        assert_eq!(secrets(msg), msg);
    }

    #[test]
    fn masks_a_value_at_the_end_of_the_string() {
        assert_eq!(
            secrets("http://h/api?password=secret"),
            "http://h/api?password=***"
        );
    }

    #[test]
    fn is_idempotent() {
        let msg = "?username=Kaiser&password=s3cr3t";
        let once = secrets(msg);
        assert_eq!(secrets(&once), once);
        assert_eq!(once, "?username=***&password=***");
    }
}
