use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Injectable environment snapshot for pure path resolution (testable without mutating process env).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathEnv {
    pub home: Option<String>,
    pub xdg_data_home: Option<String>,
    pub xdg_config_home: Option<String>,
    pub local_app_data: Option<String>,
    pub app_data: Option<String>,
}

impl PathEnv {
    pub fn from_process_env() -> Self {
        Self {
            home: home_from_process_env(),
            xdg_data_home: std::env::var("XDG_DATA_HOME").ok(),
            xdg_config_home: std::env::var("XDG_CONFIG_HOME").ok(),
            local_app_data: std::env::var("LOCALAPPDATA").ok(),
            app_data: std::env::var("APPDATA").ok(),
        }
    }
}

fn home_from_process_env() -> Option<String> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok()
    }

    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok()
    }
}

/// Resolves platform-specific storage locations for settings and auth files.
#[derive(Debug, Clone)]
pub struct StoragePaths {
    auth_dir: PathBuf,
    settings_dir: PathBuf,
}

impl StoragePaths {
    pub fn from_env() -> Result<Self> {
        let env = PathEnv::from_process_env();
        Ok(Self {
            auth_dir: resolve_auth_dir(&env)?,
            settings_dir: resolve_settings_dir(&env)?,
        })
    }

    #[cfg(test)]
    pub fn from_dirs(auth_dir: impl Into<PathBuf>, settings_dir: impl Into<PathBuf>) -> Self {
        Self {
            auth_dir: auth_dir.into(),
            settings_dir: settings_dir.into(),
        }
    }

    pub fn auth_file(&self) -> PathBuf {
        self.auth_dir.join("auth.json")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.settings_dir.join("settings.json")
    }

    #[allow(dead_code)]
    pub fn auth_dir(&self) -> &Path {
        &self.auth_dir
    }

    #[allow(dead_code)]
    pub fn settings_dir(&self) -> &Path {
        &self.settings_dir
    }
}

pub fn resolve_auth_dir(env: &PathEnv) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = env.local_app_data.as_deref() {
            return Ok(PathBuf::from(local_app_data).join("subquill"));
        }

        let home = env
            .home
            .as_deref()
            .ok_or_else(|| Error::storage("USERPROFILE is not set"))?;
        return Ok(PathBuf::from(home)
            .join("AppData")
            .join("Local")
            .join("subquill"));
    }

    #[cfg(not(windows))]
    {
        if let Some(xdg_data_home) = env.xdg_data_home.as_deref() {
            return Ok(PathBuf::from(xdg_data_home).join("subquill"));
        }

        let home = env
            .home
            .as_deref()
            .ok_or_else(|| Error::storage("HOME is not set"))?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("subquill"))
    }
}

pub fn resolve_settings_dir(env: &PathEnv) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(app_data) = env.app_data.as_deref() {
            return Ok(PathBuf::from(app_data).join("subquill"));
        }

        let home = env
            .home
            .as_deref()
            .ok_or_else(|| Error::storage("USERPROFILE is not set"))?;
        return Ok(PathBuf::from(home)
            .join("AppData")
            .join("Roaming")
            .join("subquill"));
    }

    #[cfg(not(windows))]
    {
        if let Some(xdg_config_home) = env.xdg_config_home.as_deref() {
            return Ok(PathBuf::from(xdg_config_home).join("subquill"));
        }

        let home = env
            .home
            .as_deref()
            .ok_or_else(|| Error::storage("HOME is not set"))?;
        Ok(PathBuf::from(home).join(".config").join("subquill"))
    }
}

pub fn ensure_private_dir(path: &Path) -> Result<()> {
    if path.exists() {
        if !path.is_dir() {
            return Err(Error::storage(
                "configuration directory path is not a directory",
            ));
        }
    } else {
        std::fs::create_dir_all(path)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

fn set_private_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

fn sync_parent_dir(path: &Path) -> Result<()> {
    // Directory fsync is a Unix durability aid. On Windows, opening a directory
    // handle and calling sync_all fails (ERROR_ACCESS_DENIED / not supported).
    #[cfg(unix)]
    {
        let Some(parent) = path.parent() else {
            return Ok(());
        };

        if !parent.as_os_str().is_empty() {
            let dir = std::fs::File::open(parent)?;
            dir.sync_all()?;
        }
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

/// Atomically persist `contents` at `path` using a random same-directory temp file.
pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::storage("configuration path has no parent directory"))?;

    ensure_private_dir(parent)?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| Error::storage("failed to create temporary configuration file"))?;

    tmp.write_all(contents)
        .map_err(|_| Error::storage("failed to write temporary configuration file"))?;
    tmp.as_file()
        .sync_all()
        .map_err(|_| Error::storage("failed to sync temporary configuration file"))?;

    set_private_file_permissions(tmp.path())?;

    tmp.persist(path)
        .map_err(|_| Error::storage("failed to persist configuration file"))?;

    sync_parent_dir(path).map_err(|_| Error::storage("failed to sync configuration directory"))?;

    Ok(())
}

pub fn read_to_string(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(path)?;
    Ok(Some(contents))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_and_settings_paths_are_separate() {
        let root = std::env::temp_dir().join("subquill-path-test");
        let paths = StoragePaths::from_dirs(root.join("auth"), root.join("settings"));

        assert_eq!(paths.auth_file(), root.join("auth").join("auth.json"));
        assert_eq!(
            paths.settings_file(),
            root.join("settings").join("settings.json")
        );
        assert_ne!(paths.auth_file(), paths.settings_file());
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_auth_dir_uses_xdg_data_home_when_set() {
        let env = PathEnv {
            xdg_data_home: Some("/custom/data".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_auth_dir(&env).expect("auth dir"),
            PathBuf::from("/custom/data/subquill")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_settings_dir_uses_xdg_config_home_when_set() {
        let env = PathEnv {
            xdg_config_home: Some("/custom/config".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_settings_dir(&env).expect("settings dir"),
            PathBuf::from("/custom/config/subquill")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_auth_dir_fails_without_home_when_xdg_unset() {
        let env = PathEnv::default();
        let err = resolve_auth_dir(&env).unwrap_err();
        assert_eq!(err.code(), "STORAGE_ERROR");
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_auth_dir_falls_back_to_home() {
        let env = PathEnv {
            home: Some("/home/test".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_auth_dir(&env).expect("auth dir"),
            PathBuf::from("/home/test/.local/share/subquill")
        );
    }

    #[cfg(windows)]
    #[test]
    fn resolve_auth_dir_uses_local_app_data_when_set() {
        let env = PathEnv {
            local_app_data: Some(r"C:\Users\test\AppData\Local".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_auth_dir(&env).expect("auth dir"),
            PathBuf::from(r"C:\Users\test\AppData\Local\subquill")
        );
    }

    #[cfg(windows)]
    #[test]
    fn resolve_settings_dir_uses_app_data_when_set() {
        let env = PathEnv {
            app_data: Some(r"C:\Users\test\AppData\Roaming".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_settings_dir(&env).expect("settings dir"),
            PathBuf::from(r"C:\Users\test\AppData\Roaming\subquill")
        );
    }

    #[cfg(windows)]
    #[test]
    fn resolve_auth_dir_fails_without_userprofile_when_appdata_unset() {
        let env = PathEnv::default();
        let err = resolve_auth_dir(&env).unwrap_err();
        assert_eq!(err.code(), "STORAGE_ERROR");
    }

    #[test]
    fn atomic_write_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("settings.json");
        let payload = br#"{"version":1}"#;

        atomic_write(&file, payload).expect("write");
        let read = std::fs::read(&file).expect("read");

        assert_eq!(read, payload);
    }

    #[test]
    fn atomic_write_consecutive_overwrites() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("settings.json");

        atomic_write(&file, b"version-1").expect("first write");
        atomic_write(&file, b"version-2").expect("second write");
        atomic_write(&file, b"version-3").expect("third write");

        assert_eq!(std::fs::read(&file).expect("read"), b"version-3");
    }
}
