//! Settings that outlive a restart, kept in `$XDG_CONFIG_HOME/fishpr/config`
//! (`~/.config/fishpr/config` usually). A small key=value file, so the
//! dictation process can read it without pulling in Qt.

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};

const KEY_SHORTCUT_MODE: &str = "shortcut-mode";

/// How the global shortcut starts and stops recording.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShortcutMode {
    /// Hold the keys while speaking; release stops and transcribes.
    #[default]
    Hold,
    /// Press once to start, and press again to stop and transcribe.
    Toggle,
}

impl ShortcutMode {
    pub fn is_toggle(self) -> bool {
        self == Self::Toggle
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Toggle => "toggle",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            value if value.eq_ignore_ascii_case("hold") => Some(Self::Hold),
            value if value.eq_ignore_ascii_case("toggle") => Some(Self::Toggle),
            _ => None,
        }
    }
}

/// `$XDG_CONFIG_HOME/fishpr`, where the settings file lives.
fn dir() -> Result<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .context("neither XDG_CONFIG_HOME nor HOME is set")?;
    Ok(base.join("fishpr"))
}

fn file() -> Result<PathBuf> {
    Ok(dir()?.join("config"))
}

/// The mode saved by a previous run, or `Hold` if there is none.
pub fn shortcut_mode() -> ShortcutMode {
    let text = file().ok().and_then(|path| fs::read_to_string(path).ok());
    text.as_deref().and_then(parse_shortcut_mode).unwrap_or_default()
}

/// Saves the shortcut mode, leaving any other settings in the file alone.
pub fn save_shortcut_mode(mode: ShortcutMode) -> Result<()> {
    let path = file()?;
    let old = fs::read_to_string(&path).unwrap_or_default();
    let text = with_setting(&old, KEY_SHORTCUT_MODE, mode.as_str());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

fn with_setting(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let line = format!("{key}={value}");
    match lines.iter().position(|line| setting_key(line) == Some(key)) {
        Some(index) => lines[index] = line,
        None => lines.push(line),
    }
    lines.join("\n") + "\n"
}

fn parse_shortcut_mode(text: &str) -> Option<ShortcutMode> {
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == KEY_SHORTCUT_MODE).then(|| ShortcutMode::parse(value)).flatten()
    })
}

fn setting_key(line: &str) -> Option<&str> {
    line.split_once('=').map(|(key, _)| key.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_saved_mode() {
        assert_eq!(parse_shortcut_mode("shortcut-mode=hold\n"), Some(ShortcutMode::Hold));
        assert_eq!(parse_shortcut_mode("shortcut-mode = Toggle\n"), Some(ShortcutMode::Toggle));
        assert_eq!(parse_shortcut_mode("other=1\nshortcut-mode=toggle"), Some(ShortcutMode::Toggle));
    }

    #[test]
    fn a_missing_or_broken_setting_means_hold() {
        assert_eq!(parse_shortcut_mode(""), None);
        assert_eq!(parse_shortcut_mode("shortcut-mode=sideways\n"), None);
        assert_eq!(parse_shortcut_mode("nonsense\n"), None);
        assert_eq!(ShortcutMode::default(), ShortcutMode::Hold);
    }

    #[test]
    fn names_the_modes() {
        assert_eq!(ShortcutMode::Hold.as_str(), "hold");
        assert_eq!(ShortcutMode::Toggle.as_str(), "toggle");
        assert!(ShortcutMode::Toggle.is_toggle());
        assert!(!ShortcutMode::Hold.is_toggle());
    }

    #[test]
    fn saving_a_setting_leaves_the_others_alone() {
        assert_eq!(with_setting("", "shortcut-mode", "toggle"), "shortcut-mode=toggle\n");
        assert_eq!(with_setting("other=1\n", "shortcut-mode", "toggle"), "other=1\nshortcut-mode=toggle\n");
        assert_eq!(with_setting("shortcut-mode=hold\nother=1\n", "shortcut-mode", "toggle"), "shortcut-mode=toggle\nother=1\n");
    }
}
