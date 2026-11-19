// SPDX-License-Identifier: BSD-2-Clause
//! Persistent configuration for AmeDE.
//!
//! Theme preference is stored as plain text in
//! `$XDG_CONFIG_HOME/amede/theme` (or `~/.config/amede/theme`).

use crate::scene::Theme;
use std::{fs, path::PathBuf};

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn config_dir() -> Option<PathBuf> {
    let base = if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else {
        home_dir()?.join(".config")
    };
    Some(base.join("amede"))
}

/// Reads the saved theme. Returns `None` if the file is missing or unreadable.
pub fn load_theme() -> Option<Theme> {
    load_theme_from(&config_dir()?)
}

fn load_theme_from(dir: &std::path::Path) -> Option<Theme> {
    let content = fs::read_to_string(dir.join("theme")).ok()?;
    Theme::parse(content.trim())
}

/// Saves the current theme selection. Silently ignores failures (best-effort).
pub fn save_theme(theme: Theme) {
    let Some(dir) = config_dir() else { return };
    let _ = save_theme_to(&dir, theme);
}

fn save_theme_to(dir: &std::path::Path, theme: Theme) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("theme"), theme.name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "amede-theme-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn round_trips_all_themes_and_creates_config_directory() {
        let temp = TestDir::new();
        let dir = temp.0.join("amede");
        for theme in [Theme::Amaterasu, Theme::Sakura, Theme::Oled] {
            save_theme_to(&dir, theme).unwrap();
            assert_eq!(load_theme_from(&dir), Some(theme));
        }
    }

    #[test]
    fn missing_or_invalid_theme_falls_back() {
        let temp = TestDir::new();
        assert_eq!(load_theme_from(&temp.0), None);
        for value in ["", "unknown-theme", "\u{fffd}"] {
            fs::write(temp.0.join("theme"), value).unwrap();
            assert_eq!(load_theme_from(&temp.0), None);
        }
    }

    #[test]
    fn accepts_trailing_newline() {
        let temp = TestDir::new();
        fs::write(temp.0.join("theme"), "sakura\n").unwrap();
        assert_eq!(load_theme_from(&temp.0), Some(Theme::Sakura));
    }

    #[test]
    fn save_reports_unusable_config_directory() {
        let temp = TestDir::new();
        let dir = temp.0.join("not-a-directory");
        fs::write(&dir, "keep").unwrap();
        assert!(save_theme_to(&dir, Theme::Sakura).is_err());
        assert_eq!(fs::read_to_string(dir).unwrap(), "keep");
    }
}
