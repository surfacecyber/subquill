use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{Error, Result};
use crate::paths::{atomic_write, read_to_string, StoragePaths};
use crate::redact::REDACTED;

pub const AUTH_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthSecrets {
    pub version: u32,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bilibili_cookie: Option<String>,
}

impl fmt::Debug for AuthSecrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthSecrets")
            .field("version", &self.version)
            .field("api_key", &REDACTED)
            .field(
                "bilibili_cookie",
                &self.bilibili_cookie.as_ref().map(|_| REDACTED),
            )
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct SaveAuthInput {
    pub api_key: Option<String>,
    pub bilibili_cookie: Option<String>,
    #[serde(default)]
    pub clear_bilibili_cookie: bool,
}

impl fmt::Debug for SaveAuthInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaveAuthInput")
            .field("api_key", &self.api_key.as_ref().map(|_| REDACTED))
            .field(
                "bilibili_cookie",
                &self.bilibili_cookie.as_ref().map(|_| REDACTED),
            )
            .field("clear_bilibili_cookie", &self.clear_bilibili_cookie)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuthStatus {
    pub has_api_key: bool,
    pub has_bilibili_cookie: bool,
}

pub fn validate_api_key(api_key: &str) -> Result<String> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(Error::validation("api_key is required"));
    }
    Ok(trimmed.to_string())
}

pub fn normalize_optional_secret(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Accept either `SESSDATA=…` or a raw SESSDATA value pasted from DevTools.
pub fn normalize_bilibili_cookie(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('"');
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.to_ascii_lowercase().contains("sessdata=") {
        return trimmed.to_string();
    }

    format!("SESSDATA={trimmed}")
}

pub fn resolve_bilibili_cookie(
    input: &SaveAuthInput,
    existing: Option<String>,
) -> Result<Option<String>> {
    if input.clear_bilibili_cookie {
        if input.bilibili_cookie.is_some() {
            return Err(Error::validation(
                "cannot set bilibili_cookie and clear_bilibili_cookie together",
            ));
        }
        return Ok(None);
    }

    if let Some(cookie) = normalize_optional_secret(input.bilibili_cookie.clone()) {
        let normalized = normalize_bilibili_cookie(&cookie);
        if normalized.is_empty() {
            return Ok(None);
        }
        return Ok(Some(normalized));
    }

    Ok(existing.map(|cookie| normalize_bilibili_cookie(&cookie)).filter(|c| !c.is_empty()))
}

pub fn load_auth(paths: &StoragePaths) -> Result<Option<AuthSecrets>> {
    let file = paths.auth_file();
    let Some(raw) = read_to_string(&file)? else {
        return Ok(None);
    };

    match serde_json::from_str::<AuthSecrets>(&raw) {
        Ok(auth) if auth.version == AUTH_VERSION => Ok(Some(auth)),
        Ok(_) | Err(_) => {
            // Quarantine corrupt / unsupported auth so save/status can recover.
            let bak = file.with_extension("json.bak");
            let _ = std::fs::rename(&file, &bak);
            Ok(None)
        }
    }
}

pub fn save_auth(paths: &StoragePaths, input: SaveAuthInput) -> Result<AuthStatus> {
    let existing = load_auth(paths)?;

    let bilibili_cookie = resolve_bilibili_cookie(
        &input,
        existing
            .as_ref()
            .and_then(|auth| auth.bilibili_cookie.clone()),
    )?;

    let api_key = match normalize_optional_secret(input.api_key) {
        Some(key) => validate_api_key(&key)?,
        None => existing
            .map(|auth| auth.api_key)
            .ok_or_else(|| Error::validation("api_key is required"))?,
    };

    let auth = AuthSecrets {
        version: AUTH_VERSION,
        api_key,
        bilibili_cookie,
    };

    let json = serde_json::to_vec_pretty(&auth)?;
    atomic_write(&paths.auth_file(), &json)?;

    Ok(auth_status_from(&auth))
}

pub fn auth_status(paths: &StoragePaths) -> Result<AuthStatus> {
    let auth = load_auth(paths)?;
    Ok(match auth {
        Some(secrets) => auth_status_from(&secrets),
        None => AuthStatus {
            has_api_key: false,
            has_bilibili_cookie: false,
        },
    })
}

fn auth_status_from(auth: &AuthSecrets) -> AuthStatus {
    AuthStatus {
        has_api_key: !auth.api_key.trim().is_empty(),
        has_bilibili_cookie: auth
            .bilibili_cookie
            .as_ref()
            .is_some_and(|cookie| !cookie.trim().is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_secrets_debug_redacts_sensitive_fields() {
        let secrets = AuthSecrets {
            version: AUTH_VERSION,
            api_key: "sk-secret-auth-key".to_string(),
            bilibili_cookie: Some("SESSDATA=abc".to_string()),
        };
        let debug = format!("{secrets:?}");
        assert!(!debug.contains("sk-secret-auth-key"));
        assert!(!debug.contains("SESSDATA=abc"));
        assert!(debug.contains(REDACTED));
    }

    #[test]
    fn save_auth_input_debug_redacts_sensitive_fields() {
        let input = SaveAuthInput {
            api_key: Some("sk-secret-input".to_string()),
            bilibili_cookie: Some("cookie-value".to_string()),
            clear_bilibili_cookie: false,
        };
        let debug = format!("{input:?}");
        assert!(!debug.contains("sk-secret-input"));
        assert!(!debug.contains("cookie-value"));
        assert!(debug.contains(REDACTED));
    }

    #[test]
    fn auth_status_never_serializes_secrets() {
        let status = AuthStatus {
            has_api_key: true,
            has_bilibili_cookie: true,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("has_api_key"));
        assert!(!json.contains("\"api_key\""));
        assert!(json.contains("has_bilibili_cookie"));
        assert!(!json.contains("\"bilibili_cookie\""));
    }

    #[test]
    fn save_auth_preserves_existing_key_when_omitted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path(), dir.path());

        save_auth(
            &paths,
            SaveAuthInput {
                api_key: Some("sk-test-key".to_string()),
                bilibili_cookie: None,
                clear_bilibili_cookie: false,
            },
        )
        .expect("initial save");

        let status = save_auth(
            &paths,
            SaveAuthInput {
                api_key: None,
                bilibili_cookie: Some("SESSDATA=abc".to_string()),
                clear_bilibili_cookie: false,
            },
        )
        .expect("update cookie");

        assert!(status.has_api_key);
        assert!(status.has_bilibili_cookie);

        let stored = load_auth(&paths).expect("load");
        assert_eq!(stored.expect("exists").api_key, "sk-test-key".to_string());
    }

    #[test]
    fn save_auth_clear_bilibili_cookie() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path(), dir.path());

        save_auth(
            &paths,
            SaveAuthInput {
                api_key: Some("sk-test-key".to_string()),
                bilibili_cookie: Some("SESSDATA=abc".to_string()),
                clear_bilibili_cookie: false,
            },
        )
        .expect("initial save");

        let status = save_auth(
            &paths,
            SaveAuthInput {
                api_key: None,
                bilibili_cookie: None,
                clear_bilibili_cookie: true,
            },
        )
        .expect("clear cookie");

        assert!(status.has_api_key);
        assert!(!status.has_bilibili_cookie);

        let stored = load_auth(&paths).expect("load").expect("exists");
        assert!(stored.bilibili_cookie.is_none());
    }

    #[test]
    fn save_auth_rejects_clear_and_set_together() {
        let input = SaveAuthInput {
            api_key: Some("sk-test".to_string()),
            bilibili_cookie: Some("SESSDATA=abc".to_string()),
            clear_bilibili_cookie: true,
        };

        let err = resolve_bilibili_cookie(&input, None).unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn normalize_bilibili_cookie_prefixes_raw_sessdata_value() {
        assert_eq!(
            normalize_bilibili_cookie("abc%2Cdef"),
            "SESSDATA=abc%2Cdef"
        );
        assert_eq!(
            normalize_bilibili_cookie("SESSDATA=abc%2Cdef"),
            "SESSDATA=abc%2Cdef"
        );
        assert_eq!(
            normalize_bilibili_cookie("sessdata=abc; bili_jct=1"),
            "sessdata=abc; bili_jct=1"
        );
    }

    #[test]
    fn auth_round_trip_via_storage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path(), dir.path().join("settings"));

        let status = save_auth(
            &paths,
            SaveAuthInput {
                api_key: Some("sk-secret".to_string()),
                bilibili_cookie: Some("SESSDATA=xyz".to_string()),
                clear_bilibili_cookie: false,
            },
        )
        .expect("save");

        assert!(status.has_api_key);
        assert!(status.has_bilibili_cookie);
        assert_eq!(auth_status(&paths).expect("status"), status);
    }

    #[test]
    fn load_auth_recovers_from_corrupt_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path(), dir.path());
        let file = paths.auth_file();
        std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&file, "{not-json").expect("write corrupt");

        let loaded = load_auth(&paths).expect("load");
        assert!(loaded.is_none());
        assert!(!file.exists());
        assert!(file.with_extension("json.bak").exists());

        // Save can proceed after quarantine.
        let status = save_auth(
            &paths,
            SaveAuthInput {
                api_key: Some("sk-new".to_string()),
                bilibili_cookie: None,
                clear_bilibili_cookie: false,
            },
        )
        .expect("save after quarantine");
        assert!(status.has_api_key);
    }
}
