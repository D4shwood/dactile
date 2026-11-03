//! What a new play starts from: an empty one, a few skeletons Dactile
//! brings, the writer's own templates, or the example play.

use crate::cast::Cast;
use crate::info;
use crate::lang::Lang;
use crate::lex::{self, Kind};
use crate::library;
use crate::source;
use crate::tex::{self, Edit};
use std::path::{Path, PathBuf};

/// The example play, fictional, that shows what the class does.
pub const EXAMPLE: &str = include_str!("../examples/ornek-oyun.tex");

/// The first line of a template of one's own, before its name.
const MARK: &str = "% Dactile şablonu: ";

#[derive(Debug, Clone, PartialEq)]
pub enum From {
    /// The one act and one scene of `library::template`.
    Blank,
    /// One of `BUILT_IN`, by its id.
    BuiltIn(&'static str),
    /// A file in the templates folder.
    Own(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub from: From,
    pub name: String,
    /// One line on what is in it.
    pub about: String,
}

/// A skeleton Dactile brings: its cast, and its acts with their scenes'
/// places (no act: scenes only).
struct Skeleton {
    id: &'static str,
    tr: (&'static str, &'static str),
    en: (&'static str, &'static str),
    /// `(name, alias)`, the same in both languages.
    cast: &'static [(&'static str, &'static str)],
    /// Scenes per act; `None` for a play without acts.
    acts: Option<&'static [usize]>,
    scenes: usize,
}

const BUILT_IN: [Skeleton; 4] = [
    Skeleton {
        id: "one-act",
        tr: ("Tek perdelik", "bir perde, üç sahne, iki kişi"),
        en: ("One act", "one act, three scenes, two people"),
        cast: &[("ADA", "ad"), ("DENİZ", "de")],
        acts: Some(&[3]),
        scenes: 0,
    },
    Skeleton {
        id: "three-acts",
        tr: ("Üç perdelik", "üç perde, her birinde iki sahne"),
        en: ("Three acts", "three acts of two scenes"),
        cast: &[("ADA", "ad"), ("DENİZ", "de"), ("ARDA", "ar")],
        acts: Some(&[2, 2, 2]),
        scenes: 0,
    },
    Skeleton {
        id: "monologue",
        tr: ("Monolog", "tek kişi, tek sahne"),
        en: ("Monologue", "one person, one scene"),
        cast: &[("ANLATICI", "an")],
        acts: None,
        scenes: 1,
    },
    Skeleton {
        id: "sketch",
        tr: ("Kısa skeç", "tek sahne, üç kişi, on dakika"),
        en: ("Short sketch", "one scene, three people, ten minutes"),
        cast: &[("ADA", "ad"), ("DENİZ", "de"), ("ARDA", "ar")],
        acts: None,
        scenes: 1,
    },
];

/// Where one's own templates live.
pub fn dir() -> PathBuf {
    crate::config::dir().join("templates")
}

/// Everything a new play can start from but the example: the empty play,
/// the skeletons, then one's own templates by name.
pub fn all(lang: Lang) -> Vec<Template> {
    let tr = lang == Lang::Tr;
    let mut out = vec![Template {
        from: From::Blank,
        name: (if tr { "Boş oyun" } else { "Blank play" }).into(),
        about: (if tr {
            "bir perde, bir sahne"
        } else {
            "one act, one scene"
        })
        .into(),
    }];
    for s in &BUILT_IN {
        let (name, about) = if tr { s.tr } else { s.en };
        out.push(Template {
            from: From::BuiltIn(s.id),
            name: name.into(),
            about: about.into(),
        });
    }
    out.extend(own(&dir(), lang));
    out
}

/// The templates in `dir`, by name.
pub fn own(dir: &Path, lang: Lang) -> Vec<Template> {
    let mut out: Vec<Template> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tex"))
        .filter_map(|p| {
            let text = std::fs::read_to_string(&p).ok()?;
            let name = text
                .lines()
                .next()
                .and_then(|l| l.strip_prefix(MARK))
                .map(|n| n.trim().to_string())
                .unwrap_or_else(|| {
                    p.file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                });
            let cast = Cast::of_play(&text);
            let blocks = lex::blocks(&text, source::split(&text).body, &cast);
            let acts = blocks
                .iter()
                .filter(|b| matches!(b.kind, Kind::Act { .. }))
                .count();
            let scenes = blocks
                .iter()
                .filter(|b| matches!(b.kind, Kind::Scene { .. }))
                .count();
            let about = if lang == Lang::Tr {
                format!(
                    "senin · {acts} perde, {scenes} sahne, {} kişi",
                    cast.characters.len()
                )
            } else {
                format!(
                    "yours · {acts} acts, {scenes} scenes, {} people",
                    cast.characters.len()
                )
            };
            Some(Template {
                from: From::Own(p),
                name,
                about,
            })
        })
        .collect();
    out.sort_by_key(|t| t.name.to_lowercase());
    out
}

/// A new play from `t`, with its title and author.
pub fn make(t: &Template, title: &str, author: &str, lang: Lang) -> anyhow::Result<String> {
    match &t.from {
        From::Blank => Ok(library::template(title, author, lang)),
        From::BuiltIn(id) => {
            let s = BUILT_IN
                .iter()
                .find(|s| s.id == *id)
                .ok_or_else(|| anyhow::anyhow!("no template {id}"))?;
            Ok(skeleton(s, title, author, lang))
        }
        From::Own(p) => {
            let text = crate::file::read(p)?;
            let text = text
                .strip_prefix(MARK)
                .map(|rest| rest.split_once('\n').map_or("", |(_, r)| r))
                .unwrap_or(&text)
                .to_string();
            Ok(with_cover(&text, title, author))
        }
    }
}

/// `text` with its title and author set, and its draft back at 0.1.
fn with_cover(text: &str, title: &str, author: &str) -> String {
    let mut text = text.to_string();
    for (name, value) in [
        ("title", tex::escape_text(title)),
        ("author", tex::escape_text(author)),
        ("draft", "0.1".to_string()),
    ] {
        if let Some(e) = info::set(&text, name, &value) {
            text = tex::apply(&text, &[e]);
        }
    }
    text
}

/// A skeleton written out: the empty play's cover and help, its cast, and
/// in each scene a direction and a line for each person.
fn skeleton(s: &Skeleton, title: &str, author: &str, lang: Lang) -> String {
    let tr = lang == Lang::Tr;
    let base = library::template(title, author, lang);
    let cast: String = s
        .cast
        .iter()
        .map(|(name, alias)| {
            let who = if tr {
                "kim olduğu, bir cümleyle"
            } else {
                "who, in a sentence"
            };
            format!("\\karakter[{who}]{{{name}}}{{{alias}}}\n")
        })
        .collect();
    let (act_kw, scene_kw) = if tr {
        ("perde", "sahne")
    } else {
        ("act", "scene")
    };
    let ords_tr = ["Birinci", "İkinci", "Üçüncü", "Dördüncü", "Beşinci"];
    let ords_en = ["One", "Two", "Three", "Four", "Five"];
    let scene = |n: usize| {
        let place = if tr {
            format!("Yer {n} — Zaman")
        } else {
            format!("Place {n} — Time")
        };
        let mut out = format!("#{scene_kw} {place}\n\n");
        out.push_str(if tr {
            "\\action{Işıklar açılır. Burada ne görüldüğünü yaz.}\n\n"
        } else {
            "\\action{The lights come up. Write what is seen.}\n\n"
        });
        for (_, alias) in s.cast {
            out.push_str(&format!(
                "\\{alias} {}\n",
                if tr { "Bir replik." } else { "A line." }
            ));
        }
        out.push('\n');
        out
    };
    let mut body = String::new();
    let mut n = 0;
    match s.acts {
        Some(acts) => {
            for (i, &scenes) in acts.iter().enumerate() {
                let name = if tr {
                    format!("{} Perde", ords_tr[i])
                } else {
                    format!("Act {}", ords_en[i])
                };
                body.push_str(&format!("#{act_kw} {name}\n\n"));
                for _ in 0..scenes {
                    n += 1;
                    body.push_str(&scene(n));
                }
            }
        }
        None => {
            for _ in 0..s.scenes {
                n += 1;
                body.push_str(&scene(n));
            }
        }
    }
    let parts = source::split(&base);
    // Through `\begin{document}`.
    let pre = &base[..parts.body.start];
    // The cast goes before `\begin{document}`, after the cover.
    let pre = pre.replacen("\\draft{0.1}\n", &format!("\\draft{{0.1}}\n\n{cast}"), 1);
    let body_text = &base[parts.body.clone()];
    let start = body_text
        .find("\\kisiler")
        .map_or(0, |i| i + "\\kisiler\n\n".len());
    format!(
        "{pre}{}{body}{}",
        &body_text[..start],
        &base[parts.body.end..]
    )
    .replace("\n\n\n", "\n\n")
}

/// `text` as a template of one's own named `name`: its class, cover and
/// cast, its acts and scenes, and none of its words.
pub fn from_play(text: &str, name: &str) -> String {
    let parts = source::split(text);
    let cast = Cast::of_play(text);
    let blocks = lex::blocks(text, parts.body.clone(), &cast);
    let mut cut: Vec<Edit> = Vec::new();
    for b in &blocks {
        let keep = match &b.kind {
            Kind::Act { .. } | Kind::Scene { .. } | Kind::Section { .. } => true,
            Kind::Blank | Kind::Comment => true,
            // `\maketitle`, `\kisiler`, `\newpage`: a command alone.
            Kind::Other => {
                let t = text[b.span.clone()].trim();
                t.starts_with('\\')
                    && t[1..]
                        .trim_end_matches('*')
                        .chars()
                        .all(|c| c.is_alphabetic())
            }
            _ => false,
        };
        if !keep {
            let end = (b.span.end + 1).min(text.len());
            cut.push(Edit::new(b.span.start..end, ""));
        }
    }
    let mut out = tex::apply(text, &cut);
    // What blocks leave behind: blank lines in a row.
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out = with_cover(&out, "", "");
    format!("{MARK}{}\n{out}", name.trim())
}

/// Writes `text` as a template named `name`; where it went.
pub fn save(text: &str, name: &str) -> anyhow::Result<PathBuf> {
    let dir = dir();
    std::fs::create_dir_all(&dir)?;
    let path = library::free_path(&dir, &library::file_name(name));
    crate::file::write_atomic(&path, &from_play(text, name))?;
    Ok(path)
}

/// The example, copied into `dir` under a name of its own.
pub fn copy_example(dir: &Path) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = library::free_path(dir, "ornek-oyun.tex");
    crate::file::write_atomic(&path, EXAMPLE)?;
    Ok(path)
}

#[cfg(test)]
mod tests;
