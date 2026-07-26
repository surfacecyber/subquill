use super::error::{BilibiliError, Result};

/// Map Bilibili JSON business codes shared by view/player APIs.
pub fn ensure_api_success(code: i32, message: &str, api_name: &str) -> Result<()> {
    match code {
        0 => Ok(()),
        -101 => Err(BilibiliError::auth_required(format!(
            "{api_name} requires login (code={code})"
        ))),
        -412 => Err(BilibiliError::rate_limited(format!(
            "{api_name} rate limited (code={code})"
        ))),
        _ => Err(BilibiliError::api_rejected(format!(
            "{api_name} rejected request: code={code} message={message}"
        ))),
    }
}

pub fn ensure_http_success(status: u16, api_name: &str) -> Result<()> {
    match status {
        401 | 403 => Err(BilibiliError::auth_required(format!(
            "{api_name} rejected request with HTTP {status}"
        ))),
        412 | 429 => Err(BilibiliError::rate_limited(format!(
            "{api_name} rate limited with HTTP {status}"
        ))),
        200..=299 => Ok(()),
        _ => Err(BilibiliError::network(format!(
            "{api_name} request failed with HTTP {status}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_login_required_business_code() {
        let err = ensure_api_success(-101, "账号未登录", "view API").unwrap_err();
        assert_eq!(err.code(), "AUTH_REQUIRED");
    }

    #[test]
    fn maps_rate_limit_business_code() {
        let err = ensure_api_success(-412, "request blocked", "player API").unwrap_err();
        assert_eq!(err.code(), "RATE_LIMITED");
    }

    #[test]
    fn maps_unknown_business_code_to_api_rejected() {
        let err = ensure_api_success(-999, "unknown", "view API").unwrap_err();
        assert_eq!(err.code(), "API_REJECTED");
    }

    #[test]
    fn maps_http_401_to_auth_required() {
        let err = ensure_http_success(401, "view API").unwrap_err();
        assert_eq!(err.code(), "AUTH_REQUIRED");
    }

    #[test]
    fn maps_http_429_to_rate_limited() {
        let err = ensure_http_success(429, "player API").unwrap_err();
        assert_eq!(err.code(), "RATE_LIMITED");
    }
}
