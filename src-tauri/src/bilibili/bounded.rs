use std::io::Read;

use super::error::{BilibiliError, Result};
use super::http::ResponseBodyLimit;

pub fn body_limit_exceeded(limit: ResponseBodyLimit) -> BilibiliError {
    match limit {
        ResponseBodyLimit::Subtitle => {
            BilibiliError::subtitle_corrupt("subtitle response exceeds size limit")
        }
        ResponseBodyLimit::Api => BilibiliError::api_rejected("API response exceeds size limit"),
        ResponseBodyLimit::Redirect => {
            BilibiliError::short_link_failed("short link response exceeds size limit")
        }
    }
}

/// Reject bodies whose declared Content-Length exceeds the limit before reading.
pub fn check_content_length(content_length: Option<u64>, limit: ResponseBodyLimit) -> Result<()> {
    if let Some(len) = content_length {
        if len > limit.max_bytes() {
            return Err(body_limit_exceeded(limit));
        }
    }
    Ok(())
}

/// Stream-read up to `limit + 1` bytes; error if the body exceeds `limit`.
pub fn read_bounded_body(reader: impl Read, limit: ResponseBodyLimit) -> Result<String> {
    let max = limit.max_bytes();
    let mut limited = reader.take(max.saturating_add(1));
    let mut buf = Vec::new();
    limited
        .read_to_end(&mut buf)
        .map_err(|err| BilibiliError::network(err.to_string()))?;

    if buf.len() as u64 > max {
        return Err(body_limit_exceeded(limit));
    }

    bytes_to_utf8_body(buf, limit)
}

pub fn bytes_to_utf8_body(bytes: Vec<u8>, limit: ResponseBodyLimit) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| match limit {
        ResponseBodyLimit::Subtitle => {
            BilibiliError::subtitle_corrupt("subtitle response is not valid UTF-8")
        }
        ResponseBodyLimit::Api => BilibiliError::api_rejected("API response is not valid UTF-8"),
        ResponseBodyLimit::Redirect => {
            BilibiliError::short_link_failed("short link response is not valid UTF-8")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn accepts_body_at_api_limit() {
        let data = vec![b'a'; ResponseBodyLimit::Api.max_bytes() as usize];
        let body = read_bounded_body(Cursor::new(data), ResponseBodyLimit::Api).expect("ok");
        assert_eq!(body.len(), ResponseBodyLimit::Api.max_bytes() as usize);
    }

    #[test]
    fn rejects_body_one_byte_over_api_limit() {
        let data = vec![b'a'; ResponseBodyLimit::Api.max_bytes() as usize + 1];
        let err = read_bounded_body(Cursor::new(data), ResponseBodyLimit::Api).unwrap_err();
        assert_eq!(err.code(), "API_REJECTED");
    }

    #[test]
    fn subtitle_limit_accepts_larger_body_than_api() {
        let size = ResponseBodyLimit::Api.max_bytes() as usize + 1;
        let data = vec![b'b'; size];

        let api_err = read_bounded_body(Cursor::new(&data), ResponseBodyLimit::Api).unwrap_err();
        assert_eq!(api_err.code(), "API_REJECTED");

        let subtitle =
            read_bounded_body(Cursor::new(data), ResponseBodyLimit::Subtitle).expect("subtitle");
        assert_eq!(subtitle.len(), size);
    }

    #[test]
    fn check_content_length_rejects_before_read() {
        let err = check_content_length(
            Some(ResponseBodyLimit::Api.max_bytes() + 1),
            ResponseBodyLimit::Api,
        )
        .unwrap_err();
        assert_eq!(err.code(), "API_REJECTED");
    }

    #[test]
    fn subtitle_over_limit_maps_to_subtitle_corrupt() {
        let data = vec![b'x'; ResponseBodyLimit::Subtitle.max_bytes() as usize + 1];
        let err = read_bounded_body(Cursor::new(data), ResponseBodyLimit::Subtitle).unwrap_err();
        assert_eq!(err.code(), "SUBTITLE_CORRUPT");
    }
}
