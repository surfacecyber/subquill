use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

const FILENAME_MAX_CHARS: usize = 120;

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

fn is_path_forbidden_char(c: char) -> bool {
    c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

fn trim_filename_edges(name: &str) -> String {
    name.trim()
        .trim_end_matches(|c: char| c.is_whitespace() || c == '.')
        .trim_start_matches(|c: char| c.is_whitespace())
        .to_string()
}

fn truncate_filename_chars(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        return name.to_string();
    }
    trim_filename_edges(&name.chars().take(max_chars).collect::<String>())
}

fn is_windows_reserved_filename(name: &str) -> bool {
    let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name);
    WINDOWS_RESERVED_NAMES
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// Build a safe `.md` filename from a note title.
pub fn sanitize_markdown_filename(title: &str) -> String {
    let mut base: String = title
        .chars()
        .map(|c| {
            if is_path_forbidden_char(c) {
                if c.is_whitespace() {
                    '-'
                } else {
                    '_'
                }
            } else {
                c
            }
        })
        .collect();

    base = trim_filename_edges(&base);
    base = truncate_filename_chars(&base, FILENAME_MAX_CHARS);

    if base.is_empty() {
        base = "opennote".to_string();
    }

    if is_windows_reserved_filename(&base) {
        base = format!("_{base}");
    }

    format!("{base}.md")
}

fn sanitize_id_fragment(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|c| {
            if is_path_forbidden_char(c) || c.is_whitespace() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = trim_filename_edges(&cleaned);
    truncate_filename_chars(&cleaned, 40)
}

fn unique_markdown_path(dir: &Path, title: &str, video_id: &str) -> PathBuf {
    let primary = dir.join(sanitize_markdown_filename(title));
    if !primary.exists() {
        return primary;
    }

    let stem = primary
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("opennote");
    let id = sanitize_id_fragment(video_id);
    let with_id = if id.is_empty() {
        None
    } else {
        let candidate = dir.join(format!("{stem}_{id}.md"));
        if !candidate.exists() {
            return candidate;
        }
        Some(candidate)
    };

    let mut counter = 2u32;
    loop {
        let candidate = match &with_id {
            Some(_) => dir.join(format!("{stem}_{id}_{counter}.md")),
            None => dir.join(format!("{stem}_{counter}.md")),
        };
        if !candidate.exists() {
            return candidate;
        }
        counter = counter.saturating_add(1);
        if counter == u32::MAX {
            return dir.join(format!("{stem}_{id}_{counter}.md"));
        }
    }
}

/// Write markdown into `dir`, choosing a non-colliding filename.
pub fn write_markdown_to_dir(
    dir: &Path,
    title: &str,
    video_id: &str,
    markdown: &str,
) -> Result<PathBuf> {
    if !dir.is_dir() {
        return Err(Error::storage(format!(
            "Notes save directory does not exist: {}",
            dir.display()
        )));
    }

    let path = unique_markdown_path(dir, title, video_id);
    std::fs::write(&path, markdown.as_bytes()).map_err(|err| {
        Error::storage(format!(
            "Failed to write markdown file {}: {err}",
            path.display()
        ))
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_empty_title_defaults_to_opennote() {
        assert_eq!(sanitize_markdown_filename(""), "opennote.md");
    }

    #[test]
    fn sanitize_replaces_unsafe_chars_and_adds_extension() {
        assert_eq!(
            sanitize_markdown_filename("Hello World!"),
            "Hello World!.md"
        );
    }

    #[test]
    fn sanitize_preserves_chinese_title() {
        assert_eq!(sanitize_markdown_filename("学习笔记"), "学习笔记.md");
    }

    #[test]
    fn sanitize_preserves_emoji() {
        assert_eq!(sanitize_markdown_filename("笔记🎬"), "笔记🎬.md");
    }

    #[test]
    fn sanitize_prefixes_windows_reserved_name() {
        assert_eq!(sanitize_markdown_filename("CON"), "_CON.md");
        assert_eq!(sanitize_markdown_filename("com1"), "_com1.md");
    }

    #[test]
    fn sanitize_replaces_path_traversal_chars() {
        assert_eq!(
            sanitize_markdown_filename("../../etc/passwd"),
            ".._.._etc_passwd.md"
        );
    }

    #[test]
    fn sanitize_truncates_long_titles_by_unicode_chars() {
        let long = "学".repeat(200);
        let name = sanitize_markdown_filename(&long);
        assert!(name.ends_with(".md"));
        assert_eq!(name.chars().count(), 123);
    }

    #[test]
    fn write_markdown_uses_title_when_available() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_markdown_to_dir(dir.path(), "Demo Note", "BVTEST", "# hi")
            .expect("write");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("Demo Note.md"));
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "# hi");
    }

    #[test]
    fn write_markdown_avoids_overwrite_with_video_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = write_markdown_to_dir(dir.path(), "Demo", "BV1", "# one").expect("first");
        let second = write_markdown_to_dir(dir.path(), "Demo", "BV2", "# two").expect("second");
        assert_eq!(first.file_name().and_then(|n| n.to_str()), Some("Demo.md"));
        assert_eq!(
            second.file_name().and_then(|n| n.to_str()),
            Some("Demo_BV2.md")
        );
        assert_eq!(std::fs::read_to_string(&second).expect("read"), "# two");
    }
}
