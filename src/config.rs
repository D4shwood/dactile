//! The settings: `~/.config/dactile/config.toml`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// How many recently opened plays are remembered.
const RECENT: usize = 12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Where plays live; the home screen lists the `.tex` files in it.
    pub plays_dir: String,
    /// The writer's name, for new plays.
    pub author: String,
    /// "auto" (from the play), "tr" or "en".
    pub lang: String,
    /// "auto": a backslash opens the suggestions; "manual": only ^K does.
    pub menu: String,
    /// Words per minute for running times, unless the play sets its own.
    pub wpm: u32,
    pub typewriter: bool,
    pub focus: bool,
    pub dim: bool,
    /// The play on a paper page; dark otherwise.
    pub paper: bool,
    /// Page view: A4-shaped sheets.
    pub pages: bool,
    /// The keyboard drawn on the help screen: "q" (Turkish Q), "f" (Turkish
    /// F) or "us".
    pub keyboard: String,
    /// Plays opened lately, newest first.
    pub recent: Vec<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            plays_dir: "~/Oyunlar".into(),
            author: String::new(),
            lang: "auto".into(),
            menu: "auto".into(),
            wpm: crate::timing::WPM,
            typewriter: false,
            focus: false,
            dim: false,
            paper: true,
            pages: false,
            keyboard: "q".into(),
            recent: Vec::new(),
        }
    }
}

pub fn dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dactile")
}

pub fn path() -> PathBuf {
    std::env::var_os("DACTILE_CONFIG").map_or_else(|| dir().join("config.toml"), PathBuf::from)
}

/// `~/x` → the home folder.
pub fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().unwrap_or_default().join(rest),
        None if p == "~" => dirs::home_dir().unwrap_or_default(),
        None => PathBuf::from(p),
    }
}

/// `/home/x/y` → `~/y`, for showing paths.
pub fn tilde(p: &Path) -> String {
    match dirs::home_dir().and_then(|h| p.strip_prefix(&h).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => p.display().to_string(),
    }
}

impl Config {
    pub fn exists() -> bool {
        path().is_file()
    }

    pub fn load() -> Result<Config> {
        let p = path();
        match std::fs::read_to_string(&p) {
            Ok(t) => toml::from_str(&t).with_context(|| format!("reading {}", p.display())),
            Err(_) => Ok(Config::default()),
        }
    }

    pub fn save(&self) -> Result<()> {
        let p = path();
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        crate::file::write_atomic(&p, &toml::to_string(self)?)
    }

    pub fn plays_dir(&self) -> PathBuf {
        expand(&self.plays_dir)
    }

    /// Puts a play at the top of the recent list.
    pub fn opened(&mut self, play: &Path) {
        let p = play
            .canonicalize()
            .unwrap_or_else(|_| play.to_path_buf())
            .display()
            .to_string();
        self.recent.retain(|r| *r != p);
        self.recent.insert(0, p);
        self.recent.truncate(RECENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_recent() {
        let c: Config = toml::from_str("author = \"X\"").unwrap();
        assert_eq!(c.author, "X");
        assert_eq!(c.menu, "auto");
        let mut c = Config::default();
        for i in 0..15 {
            c.opened(Path::new(&format!("/tmp/o{i}.tex")));
        }
        c.opened(Path::new("/tmp/o3.tex"));
        assert_eq!(c.recent.len(), RECENT);
        assert_eq!(c.recent[0], "/tmp/o3.tex");
        assert_eq!(expand("~/a"), dirs::home_dir().unwrap().join("a"));
    }
}
