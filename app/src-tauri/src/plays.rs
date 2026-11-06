//! What the start screen does to a play: a new draft of it, a copy, a new
//! name, the trash and back.

use dactile::config::{Config, tilde};
use dactile::lang;
use dactile::library::{self, Play};
use dactile::outputs;
use dactile::trash::{self, Moved};
use dactile::{file, info, sidecar::Sidecar, tex};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file or a folder, as the window shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Spot {
    pub path: String,
    /// `~/…`.
    pub shown: String,
    /// Seconds since 1970.
    pub modified: Option<u64>,
}

fn spot(path: &Path, modified: Option<SystemTime>) -> Spot {
    Spot {
        path: path.display().to_string(),
        shown: tilde(path),
        modified: crate::play::secs(modified),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CopiesSpot {
    #[serde(flatten)]
    pub at: Spot,
    pub pdfs: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftSpot {
    #[serde(flatten)]
    pub at: Spot,
    pub title: String,
    pub draft: Option<String>,
}

/// Where a play's things are, and whether it is somewhere temporary.
#[derive(Debug, Clone, Serialize)]
pub struct Files {
    pub folder: Spot,
    pub pdf: Option<Spot>,
    pub copies: Option<CopiesSpot>,
    pub table: Option<Spot>,
    pub drafts: Vec<DraftSpot>,
    pub temporary: bool,
}

pub fn files(play: &Path) -> Files {
    let o = outputs::of(play);
    let made = |m: outputs::Made| spot(&m.path, m.modified);
    Files {
        folder: spot(&o.folder, None),
        pdf: o.pdf.map(made),
        copies: o.copies.map(|c| CopiesSpot {
            at: spot(&c.path, c.modified),
            pdfs: c.pdfs,
        }),
        table: o.table.map(made),
        drafts: o
            .drafts
            .into_iter()
            .map(|d| DraftSpot {
                at: spot(&d.path, d.modified),
                title: d.title,
                draft: d.draft,
            })
            .collect(),
        temporary: outputs::is_temporary(play),
    }
}

/// The plays folder, as the start screen shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Folder {
    pub path: String,
    pub shown: String,
    pub exists: bool,
    pub temporary: bool,
}

pub fn folder(cfg: &Config) -> Folder {
    let dir = cfg.plays_dir();
    Folder {
        path: dir.display().to_string(),
        shown: tilde(&dir),
        exists: dir.is_dir(),
        temporary: outputs::is_temporary(&dir),
    }
}

/// What a form starts from.
#[derive(Debug, Clone, Serialize)]
pub struct Start {
    pub title: String,
    pub draft: String,
    pub file: String,
}

pub fn fork_start(path: &Path, draft: Option<&str>, today: &str) -> Start {
    let draft = library::next_draft(draft.unwrap_or(""), today);
    Start {
        title: String::new(),
        file: library::fork_name(path, &draft),
        draft,
    }
}

pub fn copy_start(title: &str) -> Start {
    let title = lang::f("{} (kopya)", "{} (copy)", &[&title]);
    Start {
        file: library::file_name(&title),
        title,
        draft: String::new(),
    }
}

pub fn rename_start(path: &Path) -> Start {
    let raw = file::read(path)
        .ok()
        .and_then(|t| info::get(&t, "title"))
        .unwrap_or_default();
    Start {
        title: tex::unescape_text(&raw),
        draft: String::new(),
        file: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
    }
}

/// `name` as a file next to `src`, with why it can't be.
fn beside(src: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(lang::l("Dosya adı boş olamaz.", "The file name can't be empty.").into());
    }
    if name.contains('/') {
        return Err(lang::l(
            "Dosya adında / olamaz: yeni oyun öbürünün yanına yazılır.",
            "No / in the file name: the new play goes next to the other.",
        )
        .into());
    }
    let name = match name.ends_with(".tex") {
        true => name.to_string(),
        false => format!("{name}.tex"),
    };
    Ok(src.with_file_name(name))
}

fn taken(path: &Path) -> String {
    lang::f("{} zaten var.", "{} already exists.", &[&tilde(path)])
}

fn held(path: &Path) -> Result<(), String> {
    match file::holder(path) {
        Some(who) => Err(lang::f(
            "Başka bir Dactile'da açık ({}).",
            "Open in another Dactile ({}).",
            &[&who],
        )),
        None => Ok(()),
    }
}

fn drafts_of(path: &Path, plays: &[Play]) -> Vec<PathBuf> {
    plays
        .iter()
        .filter(|p| p.parent.as_deref() == Some(path))
        .map(|p| p.path.clone())
        .collect()
}

/// Keeps a play outside the plays folder on the list.
fn listed(path: &Path, cfg: &mut Config) {
    if !library::list(cfg).iter().any(|p| p.path == path) {
        cfg.opened(path);
        keep(cfg);
    }
}

/// A draft (`draft` given: it remembers what it comes from) or a copy
/// (`title` given) of `src`, next to it. The character cards and the play's
/// own words go along.
pub fn make(
    src: &Path,
    title: Option<&str>,
    draft: Option<&str>,
    name: &str,
    cfg: &mut Config,
) -> Result<PathBuf, String> {
    let title = title.map(str::trim);
    if title == Some("") {
        return Err(lang::l("Başlık boş olamaz.", "The title can't be empty.").into());
    }
    let path = beside(src, name)?;
    if path.exists() {
        return Err(taken(&path));
    }
    let text = file::read(src).map_err(|e| format!("{e:#}"))?;
    let text = library::derive(&text, title, draft.map(str::trim));
    file::write_atomic(&path, &text).map_err(|e| format!("{e:#}"))?;
    let mut side = Sidecar::load(src).unwrap_or_default();
    side.parent = draft
        .and(src.file_name())
        .map(|n| n.to_string_lossy().to_string());
    let _ = side.save(&path);
    listed(&path, cfg);
    Ok(path)
}

/// A new title and file name; its history and its drafts' links follow.
pub fn rename(from: &Path, title: &str, name: &str, cfg: &mut Config) -> Result<PathBuf, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(lang::l("Başlık boş olamaz.", "The title can't be empty.").into());
    }
    held(from)?;
    let to = beside(from, name)?;
    if to != from {
        if to.exists() {
            return Err(taken(&to));
        }
        let drafts = drafts_of(from, &library::list(cfg));
        library::rename(from, &to, &drafts).map_err(|e| format!("{e:#}"))?;
        let (old, new) = (from.to_string_lossy(), to.to_string_lossy());
        for r in cfg.recent.iter_mut().filter(|r| **r == old) {
            *r = new.to_string();
        }
        keep(cfg);
    }
    let text = file::read(&to).map_err(|e| format!("{e:#}"))?;
    let old = info::get(&text, "title").unwrap_or_default();
    if tex::unescape_text(&old) != title {
        file::write_atomic(&to, &library::derive(&text, Some(title), None))
            .map_err(|e| format!("{e:#}"))?;
    }
    Ok(to)
}

/// How many drafts stay when the play goes.
pub fn drafts(path: &Path, cfg: &Config) -> usize {
    drafts_of(path, &library::list(cfg)).len()
}

/// The play and what goes with it to the desktop's trash.
pub fn throw(path: &Path, cfg: &mut Config) -> Result<Vec<Moved>, String> {
    held(path)?;
    let moved = library::throw_away(path, &trash::bin()).map_err(|e| format!("{e:#}"))?;
    let gone = path.to_string_lossy();
    cfg.recent.retain(|r| *r != gone);
    keep(cfg);
    Ok(moved)
}

/// Back from the trash.
pub fn restore(moved: &[Moved], cfg: &mut Config) -> Result<(), String> {
    trash::restore(moved).map_err(|e| format!("{e:#}"))?;
    if let Some(m) = moved.first() {
        listed(&m.from, cfg);
    }
    Ok(())
}

/// The settings to disk; a test's stay in the test, off the real ones.
fn keep(cfg: &Config) {
    if !cfg!(test) {
        let _ = cfg.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    #[test]
    fn where_things_are() {
        let dir = std::env::temp_dir().join(format!("dactile-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("oyun.tex");
        std::fs::write(&src, PLAY).unwrap();
        let mut cfg = Config {
            plays_dir: dir.display().to_string(),
            ..Config::default()
        };
        make(&src, None, Some("0.2"), "oyun-0.2.tex", &mut cfg).unwrap();
        std::fs::write(dir.join("oyun.pdf"), "x").unwrap();
        let f = files(&src);
        assert_eq!(f.folder.path, dir.display().to_string());
        assert_eq!(
            f.pdf.unwrap().path,
            dir.join("oyun.pdf").display().to_string()
        );
        assert!(f.copies.is_none());
        assert_eq!(f.drafts.len(), 1);
        assert_eq!(f.drafts[0].draft.as_deref(), Some("0.2"));
        assert!(f.temporary);
        let shown = folder(&cfg);
        assert!(shown.exists && shown.temporary);
        // The window reads a draft's path at the top of it.
        let json = serde_json::to_value(&f.drafts[0]).unwrap();
        assert!(json["path"].is_string() && json["modified"].is_number());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_draft_a_copy_and_a_new_name() {
        let dir = std::env::temp_dir().join(format!("dactile-plays-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("oyun.tex");
        std::fs::write(&src, PLAY).unwrap();
        let mut cfg = Config {
            plays_dir: dir.display().to_string(),
            ..Config::default()
        };
        let s = fork_start(&src, None, "05.11.2026");
        assert!(!s.draft.is_empty());
        let draft = make(&src, None, Some(&s.draft), &s.file, &mut cfg).unwrap();
        assert_eq!(
            Sidecar::load(&draft).unwrap().parent.as_deref(),
            Some("oyun.tex")
        );
        assert_eq!(drafts(&src, &cfg), 1);
        // Taken, and no way out of the folder.
        assert!(make(&src, None, Some("x"), &s.file, &mut cfg).is_err());
        assert!(make(&src, Some("X"), None, "a/b", &mut cfg).is_err());
        let copy = make(&src, Some("Öbür Oyun"), None, "obur", &mut cfg).unwrap();
        assert!(Sidecar::load(&copy).unwrap().parent.is_none());
        assert!(file::read(&copy).unwrap().contains("Öbür Oyun"));
        // A new name: the draft's link follows.
        let to = rename(&src, "Yeni Ad", "yeni.tex", &mut cfg).unwrap();
        assert!(!src.exists());
        assert!(file::read(&to).unwrap().contains("Yeni Ad"));
        assert_eq!(
            Sidecar::load(&draft).unwrap().parent.as_deref(),
            Some("yeni.tex")
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
