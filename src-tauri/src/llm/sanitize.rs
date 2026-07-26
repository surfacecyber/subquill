use crate::redact::redact_secret_in_text;

pub const MAX_API_ERROR_MESSAGE_CHARS: usize = 256;

/// Sanitize upstream API error text for user-facing messages.
pub fn sanitize_api_error_message(raw: &str, api_key: &str) -> String {
    let redacted = redact_secret_in_text(raw, api_key);
    let cleaned: String = redacted
        .chars()
        .filter(|ch| !ch.is_control() || *ch == ' ')
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_message(&collapsed, MAX_API_ERROR_MESSAGE_CHARS)
}

fn truncate_message(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redact::REDACTED;

    #[test]
    fn strips_control_characters() {
        let msg = sanitize_api_error_message("model\u{0001}not\u{0007}found", "");
        assert!(!msg.chars().any(|c| c.is_control()));
        assert!(msg.contains("model"));
        assert!(msg.contains("found"));
    }

    #[test]
    fn truncates_long_messages() {
        let long = "x".repeat(400);
        let msg = sanitize_api_error_message(&long, "");
        assert!(msg.chars().count() <= MAX_API_ERROR_MESSAGE_CHARS + 1);
    }

    #[test]
    fn redacts_api_key_from_error() {
        let msg = sanitize_api_error_message(
            "Invalid key sk-leaked-secret-123 in request",
            "sk-leaked-secret-123",
        );
        assert!(!msg.contains("sk-leaked-secret-123"));
        assert!(msg.contains(REDACTED));
    }
}
