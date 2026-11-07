//! The settings: `~/.config/dactile/config.toml`.

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// How many recently opened plays are remembered.
const RECENT: usize = 12;

/// The accent colours, by the name the settings keep.
pub const ACCENTS: [&str; 5] = ["claret", "blue", "green", "plum", "ochre"];

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
    /// One of `ACCENTS`.
    pub accent: String,
    /// The desktop app's page: a font's name, empty for its own; its size in
    /// pixels, its line spacing in percent, its width in ems.
    pub page_font: String,
    pub page_size: u16,
    pub line_spacing: u16,
    pub page_width: u16,
    /// Dactile Light's page, `#rrggbb`; empty: the theme's.
    pub page_bg: String,
    pub page_fg: String,
    /// Typing, scrolling and windows move; off, everything appears at once.
    /// The cursor blinks either way.
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
    /// What `read` couldn't read; while it is here, `save` writes nothing.
    #[serde(skip)]
    pub unread: Option<Unread>,
}

/// A settings file that didn't read, or not all of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Unread {
    /// The file as it was, kept beside it; none if it couldn't be copied.
    pub copy: Option<PathBuf>,
    /// The settings that fell back to their defaults; empty: all of them.
    pub fields: Vec<String>,
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
            accent: "claret".into(),
            page_font: String::new(),
            page_size: 17,
            line_spacing: 155,
            page_width: 44,
            page_bg: String::new(),
            page_fg: String::new(),
            animations: true,
            pages: false,
            ribbon_folded: false,
            page_zoom: 100,
            keyboard: "q".into(),
            recent: Vec::new(),
            viewer: Viewer::default(),
            unread: None,
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
            Ok(t) => {
                let mut cfg: Config =
                    toml::from_str(&t).with_context(|| format!("reading {}", p.display()))?;
                cfg.normal();
                Ok(cfg)
            }
            Err(_) => Ok(Config::default()),
        }
    }

    /// Brings every number back to what the page can show, whatever the
    /// file says: a size of 0 is no page.
    pub fn normal(&mut self) {
        self.page_size = self.page_size.clamp(12, 28);
        self.line_spacing = self.line_spacing.clamp(120, 220);
        self.page_width = self.page_width.clamp(32, 64);
        self.page_zoom = self.page_zoom.clamp(50, 300);
        let v = &mut self.viewer;
        v.columns = v.columns.clamp(1, 9);
        v.rows = v.rows.clamp(1, 9);
        v.offset = v.offset.min(v.columns - 1);
        v.lens_size = v.lens_size.clamp(80, 400);
        // As `clampZoom` in the viewer.
        v.zoom = if v.zoom.is_finite() {
            v.zoom.clamp(12.5, 400.0)
        } else {
            100.0
        };
    }

    /// `load` for a window, which can say what went wrong and go on: a bad
    /// value falls back alone, the rest stays. Whatever didn't read is in
    /// `unread`, with a copy of the file beside it.
    pub fn read() -> Config {
        Config::read_at(&path())
    }

    fn read_at(p: &Path) -> Config {
        let unreadable = |copy| Config {
            unread: Some(Unread {
                copy,
                fields: Vec::new(),
            }),
            ..Config::default()
        };
        let bytes = match std::fs::read(p) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Config::default(),
            Err(_) => return unreadable(None),
        };
        let table = std::str::from_utf8(&bytes)
            .ok()
            .and_then(|t| t.parse::<toml::Table>().ok());
        let Some(mut table) = table else {
            return unreadable(aside(p, &bytes));
        };
        let mut fields = Vec::new();
        if let Some(toml::Value::Table(v)) = table.remove("viewer") {
            let v = readable::<Viewer>(v, "viewer.", &mut fields);
            table.insert("viewer".into(), toml::Value::Table(v));
        }
        let table = readable::<Config>(table, "", &mut fields);
        let mut cfg: Config = toml::Value::Table(table).try_into().unwrap_or_default();
        if !fields.is_empty() {
            cfg.unread = Some(Unread {
                copy: aside(p, &bytes),
                fields,
            });
        }
        cfg.normal();
        cfg
    }

    /// Writes the settings, unless they didn't read: what the file holds is
    /// not ours to replace until someone changes a setting.
    pub fn save(&self) -> Result<()> {
        self.save_at(&path())
    }

    fn save_at(&self, p: &Path) -> Result<()> {
        if self.unread.is_some() {
            return Ok(());
        }
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        crate::file::write_atomic(p, &toml::to_string(self)?)
    }

    /// Changes what the file holds now, not what this window read a while
    /// ago: another window's settings stay.
    pub fn update(change: impl FnOnce(&mut Config) -> Result<()>) -> Result<()> {
        Config::update_at(&path(), change)
    }

    fn update_at(p: &Path, change: impl FnOnce(&mut Config) -> Result<()>) -> Result<()> {
        let mut now = Config::read_at(p);
        change(&mut now)?;
        now.save_at(p)
    }

    /// Writes the recent plays and nothing else.
    pub fn save_recent(&self) -> Result<()> {
        if self.unread.is_some() {
            return Ok(());
        }
        Config::update(|now| {
            now.recent = self.recent.clone();
            Ok(())
        })
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

/// The settings of `file` that `T` can take, naming the others in `bad`.
fn readable<T: DeserializeOwned>(
    file: toml::Table,
    prefix: &str,
    bad: &mut Vec<String>,
) -> toml::Table {
    let takes = |t: &toml::Table| toml::Value::Table(t.clone()).try_into::<T>().is_ok();
    if takes(&file) {
        return file;
    }
    let mut good = toml::Table::new();
    for (key, value) in file {
        good.insert(key.clone(), value);
        if !takes(&good) {
            good.remove(&key);
            bad.push(format!("{prefix}{key}"));
        }
    }
    good
}

/// A copy of a file that didn't read, as `config.toml.bad`; an older copy
/// with something else in it stays, and this one gets the next number.
fn aside(p: &Path, bytes: &[u8]) -> Option<PathBuf> {
    let name = p.file_name()?.to_string_lossy().to_string();
    for n in 1..100 {
        let tail = if n == 1 {
            String::new()
        } else {
            format!("-{n}")
        };
        let copy = p.with_file_name(format!("{name}.bad{tail}"));
        match std::fs::read(&copy) {
            Ok(old) if old == bytes => return Some(copy),
            Ok(_) => {}
            Err(_) => return std::fs::write(&copy, bytes).ok().map(|_| copy),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(name: &str) -> (PathBuf, PathBuf) {
        let d = std::env::temp_dir().join(format!("dactile-cfg-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let p = d.join("config.toml");
        (d, p)
    }

    #[test]
    fn a_file_that_does_not_read_stays() {
        let (d, p) = tmp("broken");
        let broken = "plays_dir = \"~/Yazılar\"\nauthor = [oops\n";
        fs::write(&p, broken).unwrap();
        let c = Config::read_at(&p);
        assert_eq!(c.plays_dir, Config::default().plays_dir);
        let u = c.unread.clone().unwrap();
        assert!(u.fields.is_empty());
        let copy = d.join("config.toml.bad");
        assert_eq!(u.copy.as_ref(), Some(&copy));
        assert_eq!(fs::read_to_string(&copy).unwrap(), broken);
        // Saving changes nothing.
        c.save_at(&p).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), broken);
        // The same file again makes no second copy; another keeps the first.
        Config::read_at(&p);
        assert!(!d.join("config.toml.bad-2").exists());
        fs::write(&p, "= başka").unwrap();
        let c = Config::read_at(&p);
        assert_eq!(c.unread.unwrap().copy, Some(d.join("config.toml.bad-2")));
        assert_eq!(fs::read_to_string(&copy).unwrap(), broken);
        // Not even text.
        fs::write(&p, [0xff, 0xfe, 0x00]).unwrap();
        assert!(Config::read_at(&p).unread.is_some());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn a_bad_value_falls_back_alone() {
        let (d, p) = tmp("field");
        let file = "plays_dir = \"~/Yazılar\"\nauthor = \"Ayşe Yılmaz\"\npage_size = \"iri\"\n[viewer]\ncolumns = \"iki\"\nlens_size = 200\n";
        fs::write(&p, file).unwrap();
        let c = Config::read_at(&p);
        assert_eq!(
            (c.plays_dir.as_str(), c.author.as_str()),
            ("~/Yazılar", "Ayşe Yılmaz")
        );
        assert_eq!(c.page_size, 17);
        assert_eq!((c.viewer.columns, c.viewer.lens_size), (1, 200));
        let mut u = c.unread.clone().unwrap();
        u.fields.sort();
        assert_eq!(u.fields, ["page_size", "viewer.columns"]);
        assert_eq!(fs::read_to_string(u.copy.unwrap()).unwrap(), file);
        c.save_at(&p).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), file);
        // A setting changed on purpose writes what did read.
        let c = Config { unread: None, ..c };
        c.save_at(&p).unwrap();
        let c = Config::read_at(&p);
        assert_eq!((c.author.as_str(), c.unread), ("Ayşe Yılmaz", None));
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn numbers_come_back_to_what_the_page_can_show() {
        let (d, p) = tmp("numbers");
        let file = "page_size = 0\nline_spacing = 65535\npage_width = 0\npage_zoom = 65535\n[viewer]\ncolumns = 0\nrows = 200\noffset = 7\nlens_size = 1\nzoom = nan\n";
        fs::write(&p, file).unwrap();
        let c = Config::read_at(&p);
        assert_eq!(
            (c.page_size, c.line_spacing, c.page_width, c.page_zoom),
            (12, 220, 32, 300)
        );
        let v = &c.viewer;
        assert_eq!((v.columns, v.rows, v.offset, v.lens_size), (1, 9, 0, 80));
        assert_eq!(v.zoom, 100.0);
        // Numbers out of range read fine: nothing to warn about.
        assert_eq!(c.unread, None);
        fs::write(&p, "page_size = 65535\n[viewer]\nzoom = 9000.0\n").unwrap();
        let c = Config::read_at(&p);
        assert_eq!((c.page_size, c.viewer.zoom), (28, 400.0));
        let mut c = Config::default();
        c.normal();
        assert_eq!(c, Config::default());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn two_windows_keep_each_others_settings() {
        let (d, p) = tmp("windows");
        Config::default().save_at(&p).unwrap();
        // Both read the file, then each sets one thing.
        let (mut one, mut two) = (Config::read_at(&p), Config::read_at(&p));
        one.theme = "night".into();
        two.author = "Can Demir".into();
        Config::update_at(&p, |now| {
            now.theme = one.theme.clone();
            Ok(())
        })
        .unwrap();
        Config::update_at(&p, |now| {
            now.author = two.author.clone();
            Ok(())
        })
        .unwrap();
        let c = Config::read_at(&p);
        assert_eq!(
            (c.theme.as_str(), c.author.as_str()),
            ("night", "Can Demir")
        );
        // A file that broke meanwhile still stays.
        fs::write(&p, "= kırık").unwrap();
        Config::update_at(&p, |_| Ok(())).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "= kırık");
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn nothing_there_and_all_well() {
        let (d, p) = tmp("fine");
        assert_eq!(Config::read_at(&p), Config::default());
        assert!(!d.join("config.toml.bad").exists());
        fs::write(&p, "author = \"X\"\nunknown = 1\n").unwrap();
        let c = Config::read_at(&p);
        assert_eq!((c.author.as_str(), c.unread), ("X", None));
        assert!(!d.join("config.toml.bad").exists());
        // Dactile Light's `load` still refuses a bad value.
        assert!(toml::from_str::<Config>("page_size = \"iri\"").is_err());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn defaults_and_recent() {
        let c: Config = toml::from_str("author = \"X\"").unwrap();
        assert_eq!(c.author, "X");
        assert_eq!(c.menu, "auto");
        // Settings older than the page's look keep the look it had.
        assert_eq!(
            (c.accent.as_str(), c.page_size, c.line_spacing, c.page_width),
            ("claret", 17, 155, 44)
        );
        assert!(c.page_font.is_empty() && c.page_bg.is_empty() && c.page_fg.is_empty());
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
