//! F9: who plays whom. The play's castings ("A", "B") and their actors, the
//! scene/role table, the numbers (lines, words, time), and what goes out of
//! them: the table as CSV or PDF, and a copy of the play for each actor.

use crate::view::unit_at;
use dactile::casting::{self, CopyKind, Presence};
use dactile::render::Doc;
use dactile::sidecar::{Casting, Sidecar};
use dactile::{build, lang, outline, timing};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct Role {
    pub alias: String,
    pub name: String,
    pub actor: Option<String>,
    pub lines: usize,
    pub words: usize,
    /// "12 dk".
    pub time: String,
    pub scenes: usize,
    /// Its share of the words spoken, in percent.
    pub share: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Scene {
    /// "I-2".
    pub name: String,
    pub place: String,
    /// Where its heading starts, in UTF-16 units.
    pub at: usize,
    pub time: String,
    pub lines: usize,
    /// By role, in the cast's order: 0 absent, 1 on stage, 2 speaks.
    pub here: Vec<u8>,
    /// "I. Perde — Kar Yağarken ≈ 20 dk" on its act's first scene.
    pub act: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Roles {
    pub castings: Vec<String>,
    pub casting: usize,
    pub wpm: u32,
    pub roles: Vec<Role>,
    pub scenes: Vec<Scene>,
    /// "≈ 45 dk · 3 200 kelime konuşulur · 2 perde · 6 sahne".
    pub play: String,
    /// Everyone cast somewhere, for the actor field.
    pub known: Vec<String>,
}

pub fn wpm(side: &Sidecar, default: u32) -> u32 {
    side.wpm.unwrap_or(default)
}

fn data(text: &str, doc: &Doc, wpm: u32) -> (Vec<Vec<Presence>>, Vec<casting::Role>) {
    let stage = dactile::stage::read(text, doc, &[]);
    let table = casting::table(text, doc, &stage);
    let roles = casting::roles(text, doc, &table, wpm);
    (table, roles)
}

pub fn roles(text: &str, side: &Sidecar, wpm: u32) -> Roles {
    let doc = Doc::new(text);
    let (table, roles) = data(text, &doc, wpm);
    let total: usize = roles.iter().map(|r| r.words).sum();
    let o = &doc.outline;
    let secs: u32 = o.scenes.iter().map(|s| timing::scene_secs(s, wpm)).sum();
    let mut act = None;
    let scenes = o
        .scenes
        .iter()
        .enumerate()
        .map(|(s, scene)| {
            let first = scene.act != act;
            act = scene.act;
            Scene {
                name: casting::scene_name(&doc, s),
                place: scene.place.trim().to_string(),
                at: unit_at(text, doc.blocks[doc.body.start + scene.block].span.start),
                time: timing::format(timing::scene_secs(scene, wpm)),
                lines: scene.lines,
                here: table[s]
                    .iter()
                    .map(|p| match p {
                        Presence::Absent => 0,
                        Presence::On => 1,
                        Presence::Speaks => 2,
                    })
                    .collect(),
                act: scene.act.filter(|_| first).map(|a| {
                    let secs: u32 = o
                        .scenes
                        .iter()
                        .filter(|x| x.act == Some(a))
                        .map(|x| timing::scene_secs(x, wpm))
                        .sum();
                    let a = &o.acts[a];
                    let title = match a.title.trim() {
                        "" => String::new(),
                        t => format!(" — {t}"),
                    };
                    format!(
                        "{}. {}{title} ≈ {}",
                        outline::roman(a.number),
                        lang::l("Perde", "Act"),
                        timing::format(secs)
                    )
                }),
            }
        })
        .collect();
    let mut known: Vec<String> = side
        .castings
        .iter()
        .flat_map(|c| c.actors.values().cloned())
        .collect();
    known.sort();
    known.dedup();
    Roles {
        castings: side.castings.iter().map(|c| c.name.clone()).collect(),
        casting: side.casting.min(side.castings.len().saturating_sub(1)),
        wpm,
        roles: doc
            .cast
            .characters
            .iter()
            .zip(&roles)
            .map(|(c, r)| Role {
                alias: c.alias.clone(),
                name: c.script.clone(),
                actor: side.actor(&c.alias).map(str::to_string),
                lines: r.lines,
                words: r.words,
                time: timing::format(r.secs),
                scenes: r.scenes.len(),
                share: (r.words * 100).checked_div(total).unwrap_or(0),
            })
            .collect(),
        scenes,
        play: lang::f(
            "≈ {} · {} kelime konuşulur · {} perde · {} sahne",
            "≈ {} · {} words spoken · {} acts · {} scenes",
            &[
                &timing::format(secs),
                &total,
                &o.acts.len(),
                &o.scenes.len(),
            ],
        ),
        known,
    }
}

/// A change to the castings, kept beside the play.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Edit {
    /// Who plays `alias` in the casting in use; empty, nobody.
    Actor {
        alias: String,
        actor: String,
    },
    /// A new casting, a copy of the one in use.
    New {
        name: String,
    },
    Rename {
        name: String,
    },
    Delete,
    Pick {
        casting: usize,
    },
    Wpm {
        wpm: u32,
    },
}

pub fn edit(side: &mut Sidecar, e: Edit) -> Result<(), String> {
    let name_given = |n: &str| -> Result<String, String> {
        match n.trim() {
            "" => Err(lang::l(
                "Bir ad yaz: A, B, prömiyer…",
                "Give it a name: A, B, premiere…",
            )
            .to_string()),
            n => Ok(n.to_string()),
        }
    };
    let n = side.castings.len();
    match e {
        Edit::Actor { alias, actor } => {
            if side.castings.is_empty() {
                side.castings.push(Casting {
                    name: "A".into(),
                    ..Casting::default()
                });
                side.casting = 0;
            }
            let c = side.casting.min(side.castings.len() - 1);
            let actors = &mut side.castings[c].actors;
            match actor.trim() {
                "" => actors.remove(&alias),
                a => actors.insert(alias, a.to_string()),
            };
        }
        Edit::New { name } => {
            let name = name_given(&name)?;
            let actors = side.current().map(|c| c.actors.clone()).unwrap_or_default();
            side.castings.push(Casting { name, actors });
            side.casting = side.castings.len() - 1;
        }
        Edit::Rename { name } => {
            let name = name_given(&name)?;
            match side.castings.get_mut(side.casting.min(n.saturating_sub(1))) {
                Some(c) => c.name = name,
                None => side.castings.push(Casting {
                    name,
                    ..Casting::default()
                }),
            }
        }
        Edit::Delete if n > 0 => {
            let c = side.casting.min(n - 1);
            side.castings.remove(c);
            side.casting = c.saturating_sub(1);
        }
        Edit::Delete => {}
        Edit::Pick { casting } => side.casting = casting.min(n.saturating_sub(1)),
        Edit::Wpm { wpm } if (30..=400).contains(&wpm) => side.wpm = Some(wpm),
        Edit::Wpm { .. } => {
            return Err(lang::l("30 ile 400 arası bir sayı.", "A number from 30 to 400.").into());
        }
    }
    Ok(())
}

/// The table beside the play, for a spreadsheet: where it went.
pub fn table_csv(text: &str, side: &Sidecar, play: &Path) -> anyhow::Result<PathBuf> {
    let doc = Doc::new(text);
    let (table, _) = data(text, &doc, timing::WPM);
    let path = casting::table_path(play, "csv");
    dactile::file::write_atomic(&path, &casting::csv(&doc, side, &table))?;
    Ok(path)
}

/// The table as a PDF beside the play (LaTeX runs: slow).
pub fn table_pdf(text: &str, side: &Sidecar, play: &Path) -> Result<PathBuf, String> {
    let doc = Doc::new(text);
    let (table, _) = data(text, &doc, timing::WPM);
    let tex = casting::table_tex(&doc, text, side, &table);
    let out = casting::table_path(play, "pdf");
    build::build_aside(play, &tex, "", &out)
        .map(|b| b.pdf)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Full,
    Rehearsal,
    Light,
}

/// A copy for each actor of the casting in use, and for each role on stage
/// nobody is cast in (LaTeX runs for each: slow). The folder, and how many.
pub fn copies(
    text: &str,
    side: &Sidecar,
    play: &Path,
    wpm: u32,
    kind: Kind,
) -> Result<(PathBuf, usize), String> {
    let doc = Doc::new(text);
    let stage = dactile::stage::read(text, &doc, &[]);
    let table = casting::table(text, &doc, &stage);
    let mut all = casting::copies(&doc, side, &table);
    if all.is_empty() {
        return Err(lang::l("Sahnede kimse yok.", "Nobody is on stage.").into());
    }
    let kind = match kind {
        Kind::Full => CopyKind::Full,
        Kind::Rehearsal => CopyKind::Rehearsal,
        Kind::Light => CopyKind::Light,
    };
    for c in &mut all {
        c.kind = kind;
    }
    let aux = build::casts(&doc.outline);
    let dir = casting::copies_dir(play);
    for c in &all {
        let tex = casting::copy_tex(text, &doc, &stage, &table, wpm, c);
        let out = dir.join(c.file_name(&doc, side));
        build::build_aside(play, &tex, &aux, &out).map_err(|e| e.to_string())?;
    }
    Ok((dir, all.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    #[test]
    fn castings_take_actors_and_the_table_shows_who_is_where() {
        let mut side = Sidecar::default();
        let r = roles(PLAY, &side, timing::WPM);
        assert!(r.castings.is_empty());
        assert!(!r.roles.is_empty());
        assert!(r.scenes.iter().all(|s| s.here.len() == r.roles.len()));
        assert!(r.scenes.iter().any(|s| s.here.contains(&2)));
        let alias = r.roles[0].alias.clone();
        edit(
            &mut side,
            Edit::Actor {
                alias: alias.clone(),
                actor: "Deniz Kaya".into(),
            },
        )
        .unwrap();
        edit(&mut side, Edit::New { name: "B".into() }).unwrap();
        let r = roles(PLAY, &side, timing::WPM);
        assert_eq!(r.castings, ["A", "B"]);
        assert_eq!(r.casting, 1);
        // A new casting starts as a copy.
        assert_eq!(r.roles[0].actor.as_deref(), Some("Deniz Kaya"));
        assert_eq!(r.known, ["Deniz Kaya"]);
        edit(
            &mut side,
            Edit::Actor {
                alias,
                actor: " ".into(),
            },
        )
        .unwrap();
        assert_eq!(roles(PLAY, &side, 130).roles[0].actor, None);
        edit(&mut side, Edit::Delete).unwrap();
        assert_eq!(side.castings.len(), 1);
        assert!(edit(&mut side, Edit::Rename { name: " ".into() }).is_err());
        assert!(edit(&mut side, Edit::Wpm { wpm: 5 }).is_err());
        edit(&mut side, Edit::Wpm { wpm: 150 }).unwrap();
        assert_eq!(wpm(&side, 130), 150);
    }
}
