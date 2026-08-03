use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Error, Result};
use crate::paths::{atomic_write, read_to_string, StoragePaths};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Zh,
    En,
    System,
}

impl Locale {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "zh" => Ok(Self::Zh),
            "en" => Ok(Self::En),
            "system" => Ok(Self::System),
            _ => Err(Error::validation("locale must be zh, en, or system")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub version: u32,
    pub base_url: String,
    pub model: String,
    pub locale: Locale,
    pub onboarding_completed: bool,
    /// Absolute path to auto-save generated notes. `None` disables auto-save.
    #[serde(default)]
    pub notes_save_dir: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "deepseek-v4-flash".to_string(),
            locale: Locale::System,
            onboarding_completed: false,
            notes_save_dir: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveSettingsInput {
    pub base_url: String,
    pub model: String,
    pub locale: String,
    pub onboarding_completed: bool,
    #[serde(default)]
    pub notes_save_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsView {
    pub version: u32,
    pub base_url: String,
    pub model: String,
    pub locale: String,
    pub onboarding_completed: bool,
    pub notes_save_dir: Option<String>,
}

impl From<Settings> for SettingsView {
    fn from(value: Settings) -> Self {
        Self {
            version: value.version,
            base_url: value.base_url,
            model: value.model,
            locale: locale_to_string(&value.locale),
            onboarding_completed: value.onboarding_completed,
            notes_save_dir: value.notes_save_dir,
        }
    }
}

fn locale_to_string(locale: &Locale) -> String {
    match locale {
        Locale::Zh => "zh".to_string(),
        Locale::En => "en".to_string(),
        Locale::System => "system".to_string(),
    }
}

/// Validates and normalizes an OpenAI-compatible API v1 root URL.
/// Subsequent requests append paths such as `chat/completions`.
pub fn validate_base_url(base_url: &str) -> Result<String> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(Error::validation("base_url is required"));
    }

    let parsed =
        Url::parse(trimmed).map_err(|_| Error::validation("base_url must be a valid URL"))?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(Error::validation("base_url must use http or https"));
        }
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(Error::validation("base_url must not contain credentials"));
    }

    if parsed.fragment().is_some() {
        return Err(Error::validation("base_url must not contain a fragment"));
    }

    if parsed.query().is_some() {
        return Err(Error::validation(
            "base_url must not contain a query string",
        ));
    }

    let host = parsed
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| Error::validation("base_url must include a host"))?;

    let mut normalized = format!("{}://{}", parsed.scheme(), host);

    if let Some(port) = parsed.port() {
        let is_default = (parsed.scheme() == "http" && port == 80)
            || (parsed.scheme() == "https" && port == 443);
        if !is_default {
            normalized.push(':');
            normalized.push_str(&port.to_string());
        }
    }

    let path = parsed.path().trim_end_matches('/');
    if !path.is_empty() {
        normalized.push_str(path);
    }

    Ok(normalized)
}

pub fn validate_model(model: &str) -> Result<String> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err(Error::validation("model is required"));
    }
    Ok(trimmed.to_string())
}

/// Validates an optional notes auto-save directory.
/// Empty / whitespace clears the setting. Otherwise the path must be absolute and exist as a directory.
pub fn validate_notes_save_dir(value: Option<String>) -> Result<Option<String>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(Error::validation(
            "notes_save_dir must be an absolute path",
        ));
    }
    if !Path::new(&path).is_dir() {
        return Err(Error::validation(
            "notes_save_dir must be an existing directory",
        ));
    }

    let normalized = path
        .canonicalize()
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    Ok(Some(normalized))
}

pub fn validate_settings_input(input: SaveSettingsInput) -> Result<Settings> {
    Ok(Settings {
        version: SETTINGS_VERSION,
        base_url: validate_base_url(&input.base_url)?,
        model: validate_model(&input.model)?,
        locale: Locale::parse(&input.locale)?,
        onboarding_completed: input.onboarding_completed,
        notes_save_dir: validate_notes_save_dir(input.notes_save_dir)?,
    })
}

pub fn load_settings(paths: &StoragePaths) -> Result<Settings> {
    let file = paths.settings_file();
    let Some(raw) = read_to_string(&file)? else {
        return Ok(Settings::default());
    };

    match serde_json::from_str::<Settings>(&raw) {
        Ok(settings) if settings.version == SETTINGS_VERSION => Ok(settings),
        Ok(_) | Err(_) => {
            // Quarantine corrupt / unsupported settings so the app can start again.
            let bak = file.with_extension("json.bak");
            let _ = std::fs::rename(&file, &bak);
            Ok(Settings::default())
        }
    }
}

pub fn save_settings(paths: &StoragePaths, settings: &Settings) -> Result<()> {
    let json = serde_json::to_vec_pretty(settings)?;
    atomic_write(&paths.settings_file(), &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_validation_accepts_localhost_with_v1_path() {
        let url = validate_base_url("http://localhost:11434/v1/").expect("valid");
        assert_eq!(url, "http://localhost:11434/v1");
    }

    #[test]
    fn base_url_validation_accepts_root_without_v1_suffix() {
        let url = validate_base_url("https://api.openai.com/").expect("valid");
        assert_eq!(url, "https://api.openai.com");
    }

    #[test]
    fn base_url_rejects_unsupported_scheme() {
        let err = validate_base_url("ftp://example.com").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn base_url_rejects_credentials() {
        let err = validate_base_url("https://user:pass@example.com/v1").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn base_url_rejects_query_string() {
        let err = validate_base_url("https://example.com/v1?key=1").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn base_url_rejects_fragment() {
        let err = validate_base_url("https://example.com/v1#secret").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn base_url_rejects_invalid_url() {
        let err = validate_base_url("not-a-url").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn base_url_rejects_scheme_only() {
        let err = validate_base_url("https://").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn settings_json_does_not_include_auth_fields() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).expect("serialize");
        assert!(!json.contains("api_key"));
        assert!(!json.contains("cookie"));
        assert!(!json.contains("bilibili"));
    }

    #[test]
    fn settings_round_trip_via_storage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path().join("auth"), dir.path());

        let notes_dir = dir.path().join("notes");
        std::fs::create_dir_all(&notes_dir).expect("mkdir notes");

        let input = SaveSettingsInput {
            base_url: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            locale: "zh".to_string(),
            onboarding_completed: true,
            notes_save_dir: Some(notes_dir.to_string_lossy().into_owned()),
        };

        let settings = validate_settings_input(input).expect("validate");
        save_settings(&paths, &settings).expect("save");
        let loaded = load_settings(&paths).expect("load");

        assert_eq!(loaded, settings);
        assert!(loaded.notes_save_dir.is_some());
    }

    #[test]
    fn notes_save_dir_empty_clears_setting() {
        let validated = validate_notes_save_dir(Some("   ".to_string())).expect("validate");
        assert_eq!(validated, None);
    }

    #[test]
    fn notes_save_dir_rejects_relative_path() {
        let err = validate_notes_save_dir(Some("relative/notes".to_string())).unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn notes_save_dir_rejects_missing_directory() {
        let missing = tempfile::tempdir()
            .expect("tempdir")
            .path()
            .join("does-not-exist");
        let err =
            validate_notes_save_dir(Some(missing.to_string_lossy().into_owned())).unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn load_settings_accepts_legacy_json_without_notes_save_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path().join("auth"), dir.path());
        let file = paths.settings_file();
        std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
        std::fs::write(
            &file,
            r#"{
              "version": 1,
              "base_url": "https://api.openai.com/v1",
              "model": "deepseek-v4-flash",
              "locale": "system",
              "onboarding_completed": true
            }"#,
        )
        .expect("write");

        let loaded = load_settings(&paths).expect("load");
        assert!(loaded.onboarding_completed);
        assert_eq!(loaded.notes_save_dir, None);
    }

    #[test]
    fn load_settings_recovers_from_corrupt_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path().join("auth"), dir.path());
        let file = paths.settings_file();
        std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&file, "{not-json").expect("write corrupt");

        let loaded = load_settings(&paths).expect("load");
        assert_eq!(loaded, Settings::default());
        assert!(!file.exists());
        assert!(file.with_extension("json.bak").exists());
    }
}
