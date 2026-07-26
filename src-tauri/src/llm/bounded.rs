use super::error::{LlmError, Result};
use super::http::MAX_RESPONSE_BYTES;

/// Collect response chunks up to `max_bytes + 1`; error if exceeded.
pub fn collect_bounded_body(chunks: impl IntoIterator<Item = Result<Vec<u8>>>) -> Result<Vec<u8>> {
    let limit = MAX_RESPONSE_BYTES + 1;
    let mut body = Vec::new();

    for chunk in chunks {
        let chunk = chunk.map_err(|_| LlmError::network("LLM response read failed"))?;
        if body.len() + chunk.len() > limit {
            return Err(LlmError::invalid_response(
                "LLM response body exceeds size limit",
            ));
        }
        body.extend_from_slice(&chunk);
    }

    if body.len() > MAX_RESPONSE_BYTES {
        return Err(LlmError::invalid_response(
            "LLM response body exceeds size limit",
        ));
    }

    Ok(body)
}

/// Reject bodies whose declared Content-Length exceeds the limit before reading.
pub fn check_content_length(content_length: Option<u64>) -> Result<()> {
    if let Some(len) = content_length {
        if len as usize > MAX_RESPONSE_BYTES {
            return Err(LlmError::invalid_response(
                "LLM response Content-Length exceeds size limit",
            ));
        }
    }
    Ok(())
}

pub fn bytes_to_utf8_body(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes)
        .map_err(|_| LlmError::invalid_response("LLM response body is not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_chunks(sizes: &[usize]) -> Vec<Result<Vec<u8>>> {
        sizes.iter().map(|size| Ok(vec![b'a'; *size])).collect()
    }

    #[test]
    fn accepts_body_at_limit() {
        let body = collect_bounded_body(ok_chunks(&[MAX_RESPONSE_BYTES])).expect("ok");
        assert_eq!(body.len(), MAX_RESPONSE_BYTES);
    }

    #[test]
    fn rejects_body_one_byte_over_limit() {
        let err = collect_bounded_body(ok_chunks(&[MAX_RESPONSE_BYTES + 1])).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }

    #[test]
    fn rejects_cumulative_overflow() {
        let half = MAX_RESPONSE_BYTES / 2 + 1;
        let err = collect_bounded_body(ok_chunks(&[half, half])).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }

    #[test]
    fn check_content_length_rejects_huge_declared_size() {
        let err = check_content_length(Some((MAX_RESPONSE_BYTES + 1) as u64)).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }
}
