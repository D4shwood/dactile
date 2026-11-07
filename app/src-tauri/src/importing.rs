//! Importing a play from a document: read and shaped in the background,
//! then a preview to put the names right in before the play is written, as
//! the terminal app does. The draft stays here while the preview is open.

use dactile::config::Config;
use dactile::import::Run;
use dactile::import::ocr::{self, Job};
use dactile::import::pages::Page;
use dactile::import::pdf::NoText;
use dactile::import::shape::{Counts, Draft, Item, shape};
use dactile::lang::{self, Lang};
use dactile::{cast, file, library};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// As many of the play's lines as the preview shows.
const SHOWN: usize = 400;

#[derive(Debug, Clone, Serialize)]
pub struct Person {
    pub name: String,
    pub alias: String,
    pub lines: usize,
    pub sample: String,
    pub not_character: bool,
    /// Who speaks their lines now, merged.
    pub merged: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Line {
    /// "act", "scene", "section", "speech", "direction" or "plain".
    pub kind: &'static str,
    pub who: Option<String>,
    pub text: String,
}

/// A paragraph of the document, and the page printed on it.
#[derive(Debug, Clone, Serialize)]
pub struct Para {
    pub text: String,
    pub page: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shown {
    pub file: String,
    pub title: String,
    pub author: String,
    pub people: Vec<Person>,
    /// What each of the document's paragraphs is taken for.
    pub labels: Vec<&'static str>,
    pub lines: usize,
    pub directions: usize,
    pub acts: usize,
    pub scenes: usize,
    pub plain: usize,
    pub play: Vec<Line>,
    /// How many more lines the play has than `play` shows.
    pub more: usize,
    /// A PDF that can be read again from its page images.
    pub ocr: bool,
    /// The cast a person a line, and what to look at in it: on the list and
    /// never speaking, speaking and not on the list.
    pub cast_text: String,
    pub silent: Vec<String>,
    pub missing: Vec<String>,
    /// The plays of a volume of several, and which this is.
    pub plays: Vec<String>,
    pub picked: Option<usize>,
}

/// A play of a volume, in a line of the volume's list.
#[derive(Debug, Clone, Serialize)]
pub struct PlayLine {
    pub title: String,
    pub pages: Option<(String, String)>,
    pub lines: usize,
    pub people: usize,
    /// Found by the pages alone, the contents not listing it.
    pub guess: bool,
}

/// What became of one play of those written together.
#[derive(Debug, Clone, Serialize)]
pub struct Written {
    pub title: String,
    pub path: Option<String>,
    pub error: Option<String>,
}

/// The chosen plays as parts of one book.
#[derive(Debug, Clone, Deserialize)]
pub struct Book {
    pub title: String,
    /// What stands before the plays comes in too.
    pub front: bool,
}

/// Stops the reading under way.
pub static STOP: AtomicBool = AtomicBool::new(false);

/// Reads a document: a PDF of pictures, or any PDF asked to be, from its
/// page images, saying how far it has got.
pub fn read(
    path: &Path,
    lang: Lang,
    ocr: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<Draft, String> {
    STOP.store(false, Ordering::Relaxed);
    let job = Job {
        progress,
        stop: &STOP,
    };
    let src = if ocr {
        dactile::import::read_ocr(path, &job)
    } else {
        match dactile::import::read(path) {
            Err(e) if e.downcast_ref::<NoText>().is_some() && ocr::ready() => {
                dactile::import::read_ocr(path, &job)
            }
            other => other,
        }
    };
    if STOP.load(Ordering::Relaxed) {
        return Err(lang::l("Okuma durduruldu.", "The reading was stopped.").into());
    }
    src.map(|src| shape(&src, lang))
        .map_err(|e| format!("{e:#}"))
}

fn joined(runs: &[Run]) -> String {
    runs.iter()
        .map(|r| {
            if r.note {
                format!(" [*{}]", r.text)
            } else {
                r.text.clone()
            }
        })
        .collect()
}

fn lines(d: &Draft) -> Vec<Line> {
    let line = |kind, who, text| Line { kind, who, text };
    d.all_items()
        .iter()
        .map(|it| match it {
            Item::Act(t) => line("act", None, t.clone()),
            Item::Scene(t) => line("scene", None, t.clone()),
            Item::Preface => line("section", None, lang::l("Önsöz", "Preface").into()),
            Item::Notes => line("section", None, lang::l("Notlar", "Notes").into()),
            Item::Speech { who, said, raw } => {
                let p = &d.people[d.owner(*who)];
                if p.not_character {
                    line("plain", None, joined(raw))
                } else {
                    line("speech", Some(p.name.clone()), joined(said))
                }
            }
            Item::Direction(r) => line("direction", None, joined(r)),
            Item::Plain(r) => line("plain", None, joined(r)),
        })
        .collect()
}

/// The whole document, for the preview to mark up.
pub fn document(d: &Draft) -> Vec<Para> {
    d.source
        .paras
        .iter()
        .map(|p| Para {
            text: p.text(),
            page: p.page.and_then(|i| d.source.pages.get(i).cloned()),
        })
        .collect()
}

pub fn shown(file: &Path, d: &Draft) -> Shown {
    let Counts {
        lines: said,
        directions,
        acts,
        scenes,
        plain,
    } = d.counts();
    let (silent, missing) = d.cast_check();
    let mut play = lines(d);
    let more = play.len().saturating_sub(SHOWN);
    play.truncate(SHOWN);
    Shown {
        file: file.display().to_string(),
        title: d.title.clone(),
        author: d.author.clone(),
        people: d
            .people
            .iter()
            .enumerate()
            .map(|(i, p)| Person {
                name: p.name.clone(),
                alias: p.alias.clone(),
                lines: if p.merged_into.is_some() {
                    p.lines
                } else {
                    d.lines_of(i)
                },
                sample: p.sample.clone(),
                not_character: p.not_character,
                merged: p.merged_into.map(|_| d.people[d.owner(i)].name.clone()),
            })
            .collect(),
        labels: d.labels.iter().map(|l| l.name()).collect(),
        lines: said,
        directions,
        acts,
        scenes,
        plain,
        play,
        more,
        cast_text: d.cast_text(),
        silent,
        missing,
        plays: d.source.plays.iter().map(|p| p.title.clone()).collect(),
        picked: d.play,
        ocr: file
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
            && ocr::ready(),
    }
}

/// A correction in the preview.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Edit {
    /// Another play of the volume.
    Play {
        i: usize,
    },
    Info {
        title: String,
        author: String,
    },
    Person {
        i: usize,
        name: String,
        alias: String,
    },
    Merge {
        from: usize,
        into: usize,
    },
    Unmerge {
        i: usize,
    },
    /// Caught as a name, but isn't one; again, it is.
    NotCharacter {
        i: usize,
    },
    /// Paragraphs `from` to `to` taken for a page of the book, or the play.
    Mark {
        from: usize,
        to: usize,
        page: String,
    },
    /// The play starts at this paragraph.
    Start {
        i: usize,
    },
    /// The play ends at this paragraph.
    End {
        i: usize,
    },
    /// Paragraphs `from` to `to` are the cast page: a first list from them.
    CastPage {
        from: usize,
        to: usize,
    },
    /// The cast by hand, a person a line.
    CastList {
        text: String,
    },
    /// A new play of the volume starts at this paragraph.
    AddPlay {
        i: usize,
        title: String,
    },
    /// This one of the volume is no play: it goes with the one before.
    DropPlay {
        i: usize,
    },
}

pub fn edit(d: &mut Draft, e: Edit) -> Result<(), String> {
    let n = d.people.len();
    let known = |i: usize| (i < n).then_some(()).ok_or("no such person".to_string());
    match e {
        Edit::Play { i } => {
            if i >= d.source.plays.len() {
                return Err("no such play".to_string());
            }
            d.go_to_play(i);
        }
        Edit::Info { title, author } => {
            d.title = title.trim().to_string();
            d.author = author.trim().to_string();
            if let Some(at) = d.play {
                d.source.plays[at].title = d.title.clone();
            }
        }
        Edit::Person { i, name, alias } => {
            known(i)?;
            let (name, alias) = (name.trim(), alias.trim());
            if name.is_empty() {
                return Err(lang::l("Ad boş olamaz.", "The name can't be empty.").into());
            }
            if alias != d.people[i].alias
                && let Some(why) = d.alias_problem(i, alias)
            {
                return Err(why.into());
            }
            d.people[i].name = cast::upper_tr(name);
            d.people[i].alias = alias.to_string();
        }
        Edit::Merge { from, into } => {
            known(from)?;
            known(into)?;
            d.merge(from, into);
        }
        Edit::Unmerge { i } => {
            known(i)?;
            d.unmerge(i);
        }
        Edit::NotCharacter { i } => {
            known(i)?;
            d.people[i].not_character = !d.people[i].not_character;
        }
        Edit::Mark { from, to, page } => {
            let page = Page::named(&page).ok_or("no such page")?;
            d.mark(from, to, page);
        }
        Edit::Start { i } => d.play_from(i),
        Edit::End { i } => d.play_to(i),
        Edit::AddPlay { i, title } => {
            d.add_play(i, &title)?;
        }
        Edit::DropPlay { i } => d.drop_play(i),
        Edit::CastPage { from, to } => d.mark_cast(from, to),
        Edit::CastList { text } => d.set_cast(&text),
    }
    Ok(())
}

/// The play, in the plays folder under its title's file name.
pub fn write(file_from: &Path, d: &Draft, cfg: &Config) -> anyhow::Result<PathBuf> {
    let title = match d.title.trim() {
        "" => file_from
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        t => t.to_string(),
    };
    let dir = cfg.plays_dir();
    std::fs::create_dir_all(&dir)?;
    let path = library::free_path(&dir, &library::file_name(&title));
    file::write_atomic(&path, &d.tex())?;
    Ok(path)
}

/// The volume's plays, a line each.
pub fn volume(d: &Draft) -> Vec<PlayLine> {
    d.summaries()
        .into_iter()
        .map(|s| PlayLine {
            title: s.title,
            pages: s.pages,
            lines: s.lines,
            people: s.people,
            guess: !s.sure,
        })
        .collect()
}

/// These plays of the volume, each under its own name: one that can't be
/// written doesn't stop the others, and nothing there is written over.
pub fn write_all(
    file_from: &Path,
    d: &Draft,
    picked: &[usize],
    book: Option<&Book>,
    cfg: &Config,
) -> Vec<Written> {
    let root = book.and_then(|b| {
        let front = if b.front {
            d.source.front_matter()
        } else {
            String::new()
        };
        dactile::book::make(&cfg.plays_dir(), &b.title, &d.author, &front, d.lang).ok()
    });
    // The list shows the newest on top: the first play is written last.
    let plays = d.plays(picked);
    let mut out: Vec<Written> = plays
        .iter()
        .enumerate()
        .rev()
        .map(|(at, play)| {
            let made = write(file_from, play, cfg).and_then(|path| {
                if let Some(root) = &root {
                    dactile::book::join(root, &path, at)?;
                }
                Ok(path)
            });
            match made {
                Ok(path) => Written {
                    title: play.title.clone(),
                    path: Some(path.display().to_string()),
                    error: None,
                },
                Err(e) => Written {
                    title: play.title.clone(),
                    path: None,
                    error: Some(format!("{e:#}")),
                },
            }
        })
        .collect();
    out.reverse();
    if let (Some(root), Some(b)) = (&root, book) {
        out.insert(
            0,
            Written {
                title: b.title.clone(),
                path: Some(root.display().to_string()),
                error: None,
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "Kar Masalı\n\nSELİN: Amca, yine mi taş durdu?\n\nOZAN: Durdu.\n\nSELIN: Peki.\n\nNOT: Bu bir not.\n\nOZAN: Gidelim.\n";

    #[test]
    fn a_preview_takes_corrections_and_the_play_comes_out() {
        let dir = std::env::temp_dir().join(format!("dactile-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("oyun.txt");
        std::fs::write(&src, TEXT).unwrap();
        let mut d = read(&src, Lang::Tr, false, &|_, _| {}).unwrap();
        let s = shown(&src, &d);
        let at = |n: &str| s.people.iter().position(|p| p.name == n);
        let (selin, ozan) = (at("SELİN").unwrap(), at("OZAN").unwrap());
        assert!(s.play.iter().any(|l| l.kind == "speech"));
        assert!(
            edit(
                &mut d,
                Edit::Person {
                    i: ozan,
                    name: " ".into(),
                    alias: "oz".into()
                }
            )
            .is_err()
        );
        edit(
            &mut d,
            Edit::Person {
                i: ozan,
                name: "ozan bey".into(),
                alias: "ob".into(),
            },
        )
        .unwrap();
        assert_eq!(d.people[ozan].name, "OZAN BEY");
        if let Some(twin) = at("SELIN") {
            edit(
                &mut d,
                Edit::Merge {
                    from: twin,
                    into: selin,
                },
            )
            .unwrap();
            let s = shown(&src, &d);
            assert_eq!(s.people[twin].merged.as_deref(), Some("SELİN"));
            assert_eq!(s.people[selin].lines, 2);
        }
        edit(
            &mut d,
            Edit::Info {
                title: "Kar".into(),
                author: "Deniz".into(),
            },
        )
        .unwrap();
        assert!(d.tex().contains("OZAN BEY"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
