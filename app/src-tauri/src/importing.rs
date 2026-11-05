//! Importing a play from a document: read and shaped in the background,
//! then a preview to put the names right in before the play is written, as
//! the terminal app does. The draft stays here while the preview is open.

use dactile::config::Config;
use dactile::import::Run;
use dactile::import::pages::Page;
use dactile::import::shape::{Counts, Draft, Item, shape};
use dactile::lang::{self, Lang};
use dactile::{cast, file, library};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
}

pub fn read(path: &Path, lang: Lang) -> Result<Draft, String> {
    dactile::import::read(path)
        .map(|src| shape(&src, lang))
        .map_err(|e| format!("{e:#}"))
}

fn joined(runs: &[Run]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
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
    }
}

/// A correction in the preview.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Edit {
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
}

pub fn edit(d: &mut Draft, e: Edit) -> Result<(), String> {
    let n = d.people.len();
    let known = |i: usize| (i < n).then_some(()).ok_or("no such person".to_string());
    match e {
        Edit::Info { title, author } => {
            d.title = title.trim().to_string();
            d.author = author.trim().to_string();
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
        let mut d = read(&src, Lang::Tr).unwrap();
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
