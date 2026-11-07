//! What the app does with plays, without the window: the home screen's
//! lists, opening a play under its lock, saving, building, and moving acts
//! and scenes on the board. The commands in `lib.rs` only pass these on.

use crate::view;
use dactile::build;
use dactile::config::{self, Config};
use dactile::file::{self, Lock, LockState};
use dactile::lang::{self, Lang};
use dactile::library;
use dactile::render::Doc;
use dactile::shelf::{self, UnitKind};
use dactile::structure;
use dactile::templates::{self, From, Template};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct PlayCard {
    pub path: String,
    pub title: String,
    pub author: String,
    pub acts: usize,
    pub scenes: usize,
    pub minutes: u32,
    pub characters: usize,
    pub words: usize,
    pub draft: Option<String>,
    /// Seconds since 1970.
    pub modified: Option<u64>,
    pub elsewhere: bool,
    /// How many plays deep in its family: a draft of a draft is 2.
    pub depth: usize,
    /// A book of plays: those under it are its parts.
    pub book: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateCard {
    pub index: usize,
    pub name: String,
    pub about: String,
    /// "blank", "built-in" or "own".
    pub kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Settings {
    pub plays_dir: String,
    pub author: String,
    pub lang: &'static str,
    pub theme: String,
    pub surprise: String,
    pub accent: String,
    pub page_font: String,
    pub page_size: u16,
    pub line_spacing: u16,
    pub page_width: u16,
    pub animations: bool,
    pub page_zoom: u16,
    pub ribbon_folded: bool,
    pub ribbon_height: u16,
    pub scene_list: bool,
    pub pdf_width: u16,
    pub cover_last: std::collections::BTreeMap<String, String>,
    pub viewer: config::Viewer,
}

#[derive(Debug, Clone, Serialize)]
pub struct Home {
    pub plays: Vec<PlayCard>,
    pub templates: Vec<TemplateCard>,
    pub settings: Settings,
    pub folder: crate::plays::Folder,
    /// What went wrong with the settings file.
    pub notice: Option<String>,
}

pub fn lang_of(cfg: &Config) -> Lang {
    lang::resolve(&cfg.lang, None)
}

pub fn secs(t: Option<SystemTime>) -> Option<u64> {
    t?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

pub fn settings(cfg: &Config) -> Settings {
    Settings {
        plays_dir: cfg.plays_dir.clone(),
        author: cfg.author.clone(),
        lang: if lang::is_tr() { "tr" } else { "en" },
        theme: cfg.theme().to_string(),
        surprise: cfg.surprise.clone(),
        accent: cfg.accent.clone(),
        page_font: cfg.page_font.clone(),
        page_size: cfg.page_size,
        line_spacing: cfg.line_spacing,
        page_width: cfg.page_width,
        animations: cfg.animations,
        page_zoom: cfg.page_zoom,
        ribbon_folded: cfg.ribbon_folded,
        ribbon_height: cfg.ribbon_height,
        scene_list: cfg.scene_list,
        pdf_width: cfg.pdf_width,
        cover_last: cfg.cover_last.clone(),
        viewer: cfg.viewer.clone(),
    }
}

pub fn home(cfg: &Config) -> Home {
    lang::set(lang_of(cfg));
    let plays = library::tree(library::list(cfg))
        .into_iter()
        .map(|(p, up)| PlayCard {
            path: p.path.display().to_string(),
            title: p.title,
            author: p.author,
            acts: p.acts,
            scenes: p.scenes,
            minutes: p.secs.div_ceil(60),
            characters: p.characters,
            words: p.words,
            draft: p.draft,
            modified: secs(p.modified),
            elsewhere: p.elsewhere,
            depth: up.len(),
            book: p.book,
        })
        .collect();
    let templates = templates::all(lang_of(cfg))
        .into_iter()
        .enumerate()
        .map(|(index, t)| TemplateCard {
            index,
            kind: match t.from {
                From::Blank => "blank",
                From::BuiltIn(_) => "built-in",
                From::Own(_) => "own",
            },
            name: t.name,
            about: t.about,
        })
        .collect();
    Home {
        plays,
        templates,
        settings: settings(cfg),
        folder: crate::plays::folder(cfg),
        notice: notice(cfg),
    }
}

/// A settings file that didn't read, in words.
fn notice(cfg: &Config) -> Option<String> {
    let u = cfg.unread.as_ref()?;
    let copy = u.copy.as_deref().map_or_else(
        || lang::l("kopyalanamadı", "couldn't be copied").to_string(),
        config::tilde,
    );
    Some(if u.fields.is_empty() {
        lang::f(
            "Ayarlar dosyası okunamadı: varsayılanlarla çalışılıyor. Bir ayar değiştirmedikçe dosyaya dokunulmaz. Kopyası: {}",
            "The settings file can't be read: running on defaults. It stays as it is until you change a setting. Its copy: {}",
            &[&copy],
        )
    } else {
        lang::f(
            "Bazı ayarlar okunamadı ({}): yerlerine varsayılanlar kondu. Bir ayar değiştirmedikçe dosyaya dokunulmaz. Kopyası: {}",
            "Some settings can't be read ({}): defaults stand in for them. The file stays as it is until you change a setting. Its copy: {}",
            &[&u.fields.join(", "), &copy],
        )
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub title: String,
    pub author: String,
    pub cast: Vec<String>,
    /// Each act's heading and its scenes'.
    pub acts: Vec<(String, Vec<String>)>,
}

/// A play's cover and outline, for the home screen.
pub fn preview(path: &Path) -> anyhow::Result<Preview> {
    let text = file::read(path)?;
    let v = view::view(&text);
    let doc = Doc::new(&text);
    let mut acts: Vec<(String, Vec<String>)> = Vec::new();
    for b in &v.blocks {
        let Some(label) = b.marks.first().and_then(|m| m.label.clone()) else {
            continue;
        };
        let head = label.lines().next().unwrap_or("").to_string();
        match b.cls {
            "act" => acts.push((label.replace('\n', " · "), Vec::new())),
            "scene" => match acts.last_mut() {
                Some(a) => a.1.push(head),
                None => acts.push((String::new(), vec![head])),
            },
            _ => {}
        }
    }
    let cover = v.cover.unwrap_or(view::Cover {
        from: 0,
        to: 0,
        title: String::new(),
        author: String::new(),
        cast: Vec::new(),
    });
    Ok(Preview {
        title: doc.title(&text).unwrap_or(cover.title),
        author: cover.author,
        cast: cover.cast.iter().map(view::CastRow::line).collect(),
        acts,
    })
}

/// A new play from template `index`, in the plays folder: its path.
pub fn create(
    cfg: &Config,
    index: usize,
    title: &str,
    details: &std::collections::BTreeMap<String, String>,
) -> anyhow::Result<PathBuf> {
    let title = title.trim();
    anyhow::ensure!(
        !title.is_empty(),
        "{}",
        lang::l("Oyunun bir adı olmalı.", "The play needs a title.")
    );
    let all: Vec<Template> = templates::all(lang_of(cfg));
    let t = all
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("no template {index}"))?;
    let mut text = templates::make(t, title, &cfg.author, lang_of(cfg))?;
    for (name, value) in details {
        let value = dactile::tex::escape_text(value.trim());
        if let Some(e) = dactile::info::fill(&text, name, &value) {
            text = dactile::tex::apply(&text, &[e]);
        }
    }
    let dir = cfg.plays_dir();
    std::fs::create_dir_all(&dir)?;
    let path = library::free_path(&dir, &library::file_name(title));
    file::write_atomic(&path, &text)?;
    Ok(path)
}

/// The example play, copied into the plays folder.
pub fn example(cfg: &Config) -> anyhow::Result<PathBuf> {
    let dir = cfg.plays_dir();
    std::fs::create_dir_all(&dir)?;
    templates::copy_example(&dir)
}

/// The play being written.
pub struct Open {
    pub path: PathBuf,
    /// None: another Dactile has it, and this one only reads.
    lock: Option<Lock>,
    history: PathBuf,
    disk_time: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Opened {
    pub path: String,
    pub name: String,
    pub text: String,
    pub read_only: bool,
    /// Who holds the play when it is read only.
    pub holder: Option<String>,
}

pub fn open(cfg: &mut Config, path: &Path) -> anyhow::Result<(Open, Opened)> {
    let opened = open_at(path, &cfg.lang, file::history_dir(path))?;
    cfg.opened(path);
    let _ = cfg.save_recent();
    Ok(opened)
}

/// Opens a play under its lock, keeping its versions in `history`.
fn open_at(path: &Path, lang_setting: &str, history: PathBuf) -> anyhow::Result<(Open, Opened)> {
    let text = file::read(path)?;
    lang::set(lang::resolve(lang_setting, Some(&text)));
    let (lock, holder) = match file::lock(path)? {
        LockState::Ours(l) => (Some(l), None),
        LockState::Taken(who) => (None, Some(who)),
    };
    let _ = file::snapshot(&history, &text, true);
    let opened = Opened {
        path: path.display().to_string(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        read_only: lock.is_none(),
        holder,
        text,
    };
    let open = Open {
        path: path.to_path_buf(),
        lock,
        history,
        disk_time: file::modified(path),
    };
    Ok((open, opened))
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Saved {
    Saved,
    ReadOnly,
    /// Changed on disk since it was opened or saved: not written over.
    ChangedElsewhere {
        text: String,
    },
}

impl Open {
    pub fn save(&mut self, text: &str, force: bool) -> anyhow::Result<Saved> {
        if self.lock.is_none() {
            return Ok(Saved::ReadOnly);
        }
        let now = file::modified(&self.path);
        if !force && now.is_some() && now != self.disk_time {
            let theirs = file::read(&self.path)?;
            if theirs != text {
                return Ok(Saved::ChangedElsewhere { text: theirs });
            }
        }
        if force {
            // Theirs is kept in the history before it goes.
            if let Ok(theirs) = file::read(&self.path) {
                let _ = file::snapshot(&self.history, &theirs, true);
            }
        }
        file::write_atomic(&self.path, text)?;
        self.disk_time = file::modified(&self.path);
        let _ = file::snapshot(&self.history, text, false);
        let _ = build::seed(&self.path, &build::casts(&build::outline_of(text)), false);
        Ok(Saved::Saved)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BuildResult {
    Built {
        pdf: String,
        pages: Option<usize>,
        seconds: f32,
    },
    Failed {
        line: Option<usize>,
        message: String,
    },
}

/// Builds the PDF from the play as it is on disk.
pub fn build_pdf(path: &Path) -> BuildResult {
    match build::build(path) {
        Ok(b) => BuildResult::Built {
            pdf: b.pdf.display().to_string(),
            pages: b.pages,
            seconds: b.took.as_secs_f32(),
        },
        Err(f) => BuildResult::Failed {
            line: f.line,
            message: f.message,
        },
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Card {
    pub act: bool,
    pub hidden: bool,
    pub label: String,
    pub title: String,
    pub lines: usize,
    /// Where it starts, in UTF-16 units: the editor goes there.
    pub at: usize,
    /// Where its last line with something on it ends.
    pub end: usize,
    /// The places it can be moved to (before card `p`; the count means
    /// after the last).
    pub places: Vec<usize>,
}

/// The acts and scenes, in order, for the board.
pub fn board(text: &str) -> Vec<Card> {
    let doc = Doc::new(text);
    let units = shelf::units(text, &doc);
    units
        .iter()
        .enumerate()
        .map(|(i, u)| Card {
            act: u.kind == UnitKind::Act,
            hidden: u.hidden,
            label: u.label.clone(),
            title: u.title.trim().to_string(),
            lines: u.lines,
            at: view::unit_at(text, u.span.start),
            end: view::unit_at(text, u.span.end),
            places: structure::places(&units, i),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Change {
    pub from: usize,
    pub to: usize,
    pub insert: String,
    /// Where the moved card starts afterwards.
    pub at: usize,
}

/// Card `i` moved to place `to`: the one change to make, in UTF-16 units.
pub fn relocate(text: &str, i: usize, to: usize) -> Option<Change> {
    let doc = Doc::new(text);
    let units = shelf::units(text, &doc);
    let (edit, start) = structure::relocate(text, &units, i, to)?;
    let mut after = text.to_string();
    after.replace_range(edit.range.clone(), &edit.text);
    Some(Change {
        from: view::unit_at(text, edit.range.start),
        to: view::unit_at(text, edit.range.end),
        at: view::unit_at(&after, start),
        insert: edit.text,
    })
}

/// The settings the app changes.
pub fn set(cfg: &mut Config, key: &str, value: &serde_json::Value) -> anyhow::Result<()> {
    let apply = |c: &mut Config| -> anyhow::Result<()> {
        change(c, key, value)?;
        // The viewer keeps its layout by itself: nobody chose to write the file.
        // And a file with no copy beside it stays, whatever is chosen.
        c.unread.take_if(|u| key != "viewer" && u.copy.is_some());
        Ok(())
    };
    apply(cfg)?;
    if cfg.unread.is_some() {
        return Ok(());
    }
    Config::update(apply)
}

fn change(cfg: &mut Config, key: &str, value: &serde_json::Value) -> anyhow::Result<()> {
    let number = |default: u16, least: u16, most: u16| {
        value.as_u64().map_or(default, |n| {
            n.clamp(u64::from(least), u64::from(most)) as u16
        })
    };
    match key {
        "theme" => cfg.theme = value.as_str().unwrap_or("paper").to_string(),
        "surprise" => cfg.surprise = value.as_str().unwrap_or("").to_string(),
        "page_zoom" => cfg.page_zoom = value.as_u64().unwrap_or(100).clamp(50, 300) as u16,
        "ribbon_folded" => cfg.ribbon_folded = value.as_bool().unwrap_or(false),
        "scene_list" => cfg.scene_list = value.as_bool().unwrap_or(true),
        "ribbon_height" => cfg.ribbon_height = value.as_u64().unwrap_or(0).min(400) as u16,
        "pdf_width" => cfg.pdf_width = value.as_u64().unwrap_or(0).min(4000) as u16,
        "cover_last" => {
            cfg.cover_last = serde_json::from_value(value.clone()).unwrap_or_default();
        }
        "author" => cfg.author = value.as_str().unwrap_or("").to_string(),
        "plays_dir" => cfg.plays_dir = value.as_str().unwrap_or("~/Oyunlar").to_string(),
        "lang" => cfg.lang = value.as_str().unwrap_or("auto").to_string(),
        // Only what the window changed: another window's choices stay.
        "viewer" => {
            let mut now = serde_json::to_value(&cfg.viewer)?;
            if let (Some(now), Some(new)) = (now.as_object_mut(), value.as_object()) {
                now.extend(new.clone());
            }
            cfg.viewer = serde_json::from_value(now)?;
        }
        "accent" => {
            let a = value.as_str().filter(|a| config::ACCENTS.contains(a));
            cfg.accent = a.unwrap_or(config::ACCENTS[0]).to_string();
        }
        // It goes into a style between quotes.
        "page_font" => {
            let font = value.as_str().unwrap_or("");
            if font.contains(['"', '\\', ';', '\n']) {
                anyhow::bail!("no such font");
            }
            cfg.page_font = font.to_string();
        }
        "page_size" => cfg.page_size = number(17, 12, 28),
        "line_spacing" => cfg.line_spacing = number(155, 120, 220),
        "page_width" => cfg.page_width = number(44, 32, 64),
        "animations" => cfg.animations = value.as_bool().unwrap_or(true),
        _ => anyhow::bail!("no setting {key}"),
    }
    cfg.normal();
    Ok(())
}

pub fn load_config() -> Config {
    Config::read()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    fn apply(text: &str, c: &Change) -> String {
        let mut u: Vec<u16> = text.encode_utf16().collect();
        u.splice(c.from..c.to, c.insert.encode_utf16());
        String::from_utf16(&u).unwrap()
    }

    #[test]
    fn a_viewer_change_leaves_the_other_viewer_choices() {
        let mut c = Config::default();
        c.viewer.columns = 2;
        change(&mut c, "viewer", &serde_json::json!({ "invert": true })).unwrap();
        assert_eq!((c.viewer.columns, c.viewer.invert), (2, true));
    }

    #[test]
    fn a_settings_file_that_did_not_read_is_told_and_kept() {
        let mut c = Config {
            unread: Some(config::Unread {
                copy: Some(PathBuf::from("/ev/config.toml.bad")),
                fields: vec!["page_size".into()],
            }),
            ..Config::default()
        };
        lang::set(Lang::En);
        let told = notice(&c).unwrap();
        assert!(told.contains("page_size") && told.contains("config.toml.bad"));
        // The viewer saving its layout is no one's choice: still nothing written.
        let viewer = serde_json::to_value(config::Viewer::default()).unwrap();
        set(&mut c, "viewer", &viewer).unwrap();
        assert!(c.unread.is_some());
        assert_eq!(notice(&Config::default()), None);
    }

    #[test]
    fn the_look_stays_in_range() {
        use serde_json::json;
        let mut c = Config::default();
        change(&mut c, "accent", &json!("blue")).unwrap();
        assert_eq!(c.accent, "blue");
        change(&mut c, "accent", &json!("pembe")).unwrap();
        assert_eq!(c.accent, "claret");
        change(&mut c, "page_size", &json!(99)).unwrap();
        change(&mut c, "line_spacing", &json!(10)).unwrap();
        change(&mut c, "page_width", &json!(52)).unwrap();
        assert_eq!((c.page_size, c.line_spacing, c.page_width), (28, 120, 52));
        change(&mut c, "page_font", &json!("Noto Serif")).unwrap();
        assert!(change(&mut c, "page_font", &json!("x\"; color: red")).is_err());
        assert_eq!(c.page_font, "Noto Serif");
        change(&mut c, "animations", &json!(false)).unwrap();
        assert!(!settings(&c).animations);
        let mut v = serde_json::to_value(config::Viewer::default()).unwrap();
        v["columns"] = json!(0);
        v["lens_size"] = json!(5000);
        change(&mut c, "viewer", &v).unwrap();
        assert_eq!((c.viewer.columns, c.viewer.lens_size), (1, 400));
    }

    #[test]
    fn board_moves_a_scene() {
        lang::set(Lang::Tr);
        let cards = board(PLAY);
        assert!(cards.iter().any(|c| c.act));
        let i = cards.iter().position(|c| !c.act).unwrap();
        let to = *cards[i].places.iter().rfind(|&&p| p != i).unwrap();
        let c = relocate(PLAY, i, to).unwrap();
        let after = apply(PLAY, &c);
        assert_ne!(after, PLAY);
        // The same scenes, in another order: nothing lost, nothing added.
        let count = |t: &str| board(t).iter().filter(|c| !c.act).count();
        assert_eq!(count(&after), count(PLAY));
        assert_eq!(after.len(), PLAY.len());
        // The editor goes to the scene where it landed.
        let back = board(&after);
        let landed = back.iter().find(|b| b.at == c.at).unwrap();
        assert_eq!(landed.title, cards[i].title);
    }

    #[test]
    fn staying_put_is_no_change() {
        let cards = board(PLAY);
        let i = cards.iter().position(|c| !c.act).unwrap();
        assert!(relocate(PLAY, i, i).is_none());
    }

    #[test]
    fn open_save_and_lock() {
        let dir = std::env::temp_dir().join(format!("dactile-app-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("oyun.tex");
        std::fs::write(&path, PLAY).unwrap();
        let history = dir.join("history");
        let (mut open, opened) = open_at(&path, "tr", history.clone()).unwrap();
        assert!(!opened.read_only);
        assert_eq!(opened.text, PLAY);
        let edited = PLAY.replacen("\\begin{document}", "\\begin{document}\n% bir not", 1);
        assert!(matches!(open.save(&edited, false).unwrap(), Saved::Saved));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
        // Written elsewhere meanwhile: not written over.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, PLAY).unwrap();
        assert!(matches!(
            open.save(&edited, false).unwrap(),
            Saved::ChangedElsewhere { .. }
        ));
        assert!(matches!(open.save(&edited, true).unwrap(), Saved::Saved));
        assert!(!file::versions(&history).is_empty());
        drop(open);
        assert!(!file::lock_path(&path).exists());
        // Open on another machine (a shared folder): this one only reads.
        std::fs::write(file::lock_path(&path), "1 elsewhere 2026-01-01T00:00 0\n").unwrap();
        let (mut second, opened) = open_at(&path, "tr", history.clone()).unwrap();
        assert!(opened.read_only && opened.holder.is_some());
        assert!(matches!(second.save(PLAY, true).unwrap(), Saved::ReadOnly));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
