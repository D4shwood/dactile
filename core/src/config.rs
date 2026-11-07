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
    /// The play on a paper page; dark otherwise. Kept for older settings:
    /// `theme` says more.
    pub paper: bool,
    /// paper, sepia, night, contrast or dark; empty in older settings.
    pub theme: String,
    /// Typing, scrolling and windows move; off, everything appears at once.
    pub animations: bool,
    /// Page view: A4-shaped sheets.
    pub pages: bool,
    /// The writer's ribbon folded to its tabs.
    pub ribbon_folded: bool,
    /// The desktop app's page, in percent: the text grows, the window doesn't.
    pub page_zoom: u16,
    /// The keyboard drawn on the help screen: "q" (Turkish Q), "f" (Turkish
    /// F) or "us".
    pub keyboard: String,
    /// Plays opened lately, newest first.
    pub recent: Vec<String>,
    /// The desktop app's PDF viewer.
    pub viewer: Viewer,
}

/// What the PDF viewer remembers between plays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Viewer {
    /// "width", "text", "window", or "" for `zoom`.
    pub fit: String,
    /// In percent, when `fit` is empty.
    pub zoom: f32,
    pub continuous: bool,
    pub columns: u8,
    pub rows: u8,
    /// Next and previous go one page at a time, whatever the grid.
    pub single_step: bool,
    /// Empty places before the first page.
    pub offset: u8,
    /// The open tab of the left panel, and of the right one; empty: closed.
    pub left: String,
    pub right: String,
    pub toolbar: bool,
    pub invert: bool,
    pub gray: bool,
    pub lens_round: bool,
    /// Across, in pixels.
    pub lens_size: u16,
    pub lens_shadow: bool,
    pub lens_border: bool,
    /// The PDF goes where the writer's cursor goes.
    pub follow_cursor: bool,
    /// The writer's cursor goes where the PDF scrolls.
    pub follow_scroll: bool,
}

impl Default for Viewer {
    fn default() -> Viewer {
        Viewer {
            fit: "width".into(),
            zoom: 100.0,
            continuous: true,
            columns: 1,
            rows: 1,
            single_step: false,
            offset: 0,
            left: String::new(),
            right: String::new(),
            toolbar: true,
            invert: false,
            gray: false,
            lens_round: true,
            lens_size: 170,
            lens_shadow: true,
            lens_border: false,
            follow_cursor: false,
            follow_scroll: false,
        }
    }
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
            theme: String::new(),
            animations: true,
            pages: false,
            ribbon_folded: false,
            page_zoom: 100,
            keyboard: "q".into(),
            recent: Vec::new(),
            viewer: Viewer::default(),
        }
    }
}

impl Config {
    /// The theme, from `paper` when the settings are older than themes.
    pub fn theme(&self) -> &str {
        match self.theme.as_str() {
            "" if self.paper => "paper",
            "" => "dark",
            t => t,
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

    #[test]
    fn older_settings_have_a_viewer_and_it_comes_back() {
        let c: Config = toml::from_str("author = \"X\"").unwrap();
        assert_eq!(c.viewer, Viewer::default());
        let c: Config = toml::from_str("[viewer]\ncolumns = 2").unwrap();
        assert_eq!(c.viewer.columns, 2);
        assert_eq!(c.viewer.fit, "width");
        let mut c = Config::default();
        c.recent.push("/tmp/o.tex".into());
        c.viewer.zoom = 12.5;
        let back: Config = toml::from_str(&toml::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }
}
