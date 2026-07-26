pub const REDACTED: &str = "[redacted]";

/// Truncate `text` to at most `max_chars` Unicode scalars without splitting UTF-8.
pub fn truncate_unicode_chars(text: &str, max_chars: usize) -> (String, bool) {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return (text.to_string(), false);
    }

    let truncated: String = text.chars().take(max_chars).collect();
    (truncated, true)
}

/// Append a truncation notice when `was_truncated` is true.
pub fn with_truncation_notice(text: String, was_truncated: bool, omitted_chars: usize) -> String {
    if was_truncated {
        format!("{text}… [truncated, {omitted_chars} chars omitted]")
    } else {
        text
    }
}

/// Remove substrings equal to `secret` (e.g. API key) from diagnostic text.
pub fn redact_secret_in_text(text: &str, secret: &str) -> String {
    let trimmed_secret = secret.trim();
    if trimmed_secret.is_empty() {
        return text.to_string();
    }
    text.replace(trimmed_secret, REDACTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_unicode_chars_does_not_split_utf8() {
        let text = "中文".repeat(10);
        let (truncated, was_truncated) = truncate_unicode_chars(&text, 5);
        assert!(was_truncated);
        assert_eq!(truncated.chars().count(), 5);
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn redact_secret_replaces_key() {
        let out = redact_secret_in_text("bad key sk-secret-value here", "sk-secret-value");
        assert!(!out.contains("sk-secret-value"));
        assert!(out.contains(REDACTED));
    }
}
