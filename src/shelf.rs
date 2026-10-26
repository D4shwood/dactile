//! Putting scenes and acts away: hidden in the play as `%~` comment lines
//! (out of the PDF, one tap from coming back), or moved to the play's trash
//! file next to it, from where they can be put back or destroyed.

use crate::lex::{self, Kind};
use crate::outline;
use crate::render::Doc;
use crate::tex::Edit;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// What starts a hidden line.
pub const HIDDEN: &str = "%~";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Act,
    Scene,
}

/// An act or a scene as a piece of the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub kind: UnitKind,
    pub hidden: bool,
    /// From its heading to its last line that has something on it.
    pub span: Range<usize>,
    /// "II. Perde", "Sahne 2"; for a hidden one without its number.
    pub label: String,
    /// The act's title or the scene's place.
    pub title: String,
    /// Lines of dialogue in it.
    pub lines: usize,
}

impl Unit {
    pub fn name(&self) -> String {
        if self.title.trim().is_empty() {
            self.label.clone()
        } else {
            format!("{} · {}", self.label, self.title.trim())
        }
    }
}

/// Whether a block is a hidden (`%~`) line.
pub fn is_hidden(text: &str, b: &lex::Block) -> bool {
    matches!(b.kind, Kind::Comment) && text[b.span.clone()].trim_start().starts_with(HIDDEN)
}

/// `text` with one level of `%~` taken off each line.
pub fn strip(text: &str) -> String {
    text.split('\n')
        .map(|l| {
            let t = l.trim_start();
            match t.strip_prefix(HIDDEN) {
                Some(rest) => rest.strip_prefix(' ').unwrap_or(rest),
                None => l,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The acts and scenes of the body in order, hidden ones included.
pub fn units(text: &str, doc: &Doc) -> Vec<Unit> {
    // (block, kind, hidden, label, title, lines)
    struct Head {
        block: usize,
        kind: UnitKind,
        hidden: bool,
        label: String,
        title: String,
        lines: usize,
        /// The last block of a hidden run.
        last: usize,
    }
    let mut heads: Vec<Head> = Vec::new();
    // Blocks that end a unit: a section starts a page of its own.
    let mut stops: Vec<(usize, Option<UnitKind>)> = Vec::new();
    let body = doc.body.clone();
    let mut i = body.start;
    while i < body.end {
        let b = &doc.blocks[i];
        if is_hidden(text, b) {
            let mut j = i;
            while j + 1 < body.end && is_hidden(text, &doc.blocks[j + 1]) {
                j += 1;
            }
            let inner = strip(&text[b.span.start..doc.blocks[j].span.end]);
            let blocks = lex::blocks(&inner, 0..inner.len(), &doc.cast);
            let first = blocks
                .iter()
                .find(|x| !matches!(x.kind, Kind::Blank | Kind::Comment));
            let (kind, title) = match first.map(|x| &x.kind) {
                Some(Kind::Act { title, .. }) => (Some(UnitKind::Act), title.clone()),
                Some(Kind::Scene { place, .. }) => (Some(UnitKind::Scene), place.clone()),
                _ => (None, String::new()),
            };
            if let Some(kind) = kind {
                let lines = blocks
                    .iter()
                    .filter(|x| matches!(x.kind, Kind::Line { .. }))
                    .count();
                heads.push(Head {
                    block: i,
                    kind,
                    hidden: true,
                    label: match kind {
                        UnitKind::Act => crate::lang::l("Perde", "Act").to_string(),
                        UnitKind::Scene => crate::lang::l("Sahne", "Scene").to_string(),
                    },
                    title,
                    lines,
                    last: j,
                });
                stops.push((i, Some(kind)));
            }
            i = j + 1;
            continue;
        }
        let k = i - body.start;
        match &b.kind {
            Kind::Act { title, .. } => {
                let a = doc
                    .outline
                    .act_at(k)
                    .map_or(0, |a| doc.outline.acts[a].number);
                heads.push(Head {
                    block: i,
                    kind: UnitKind::Act,
                    hidden: false,
                    label: format!("{}. {}", outline::roman(a), crate::lang::l("Perde", "Act")),
                    title: title.clone(),
                    lines: 0,
                    last: i,
                });
                stops.push((i, Some(UnitKind::Act)));
            }
            Kind::Scene { place, .. } => {
                let s = doc.scene_of(i).map(|s| &doc.outline.scenes[s]);
                heads.push(Head {
                    block: i,
                    kind: UnitKind::Scene,
                    hidden: false,
                    label: format!(
                        "{} {}",
                        crate::lang::l("Sahne", "Scene"),
                        s.map_or(String::new(), |s| s.label.clone())
                    ),
                    title: place.clone(),
                    lines: s.map_or(0, |s| s.lines),
                    last: i,
                });
                stops.push((i, Some(UnitKind::Scene)));
            }
            Kind::Section { .. } => stops.push((i, None)),
            _ => {}
        }
        i += 1;
    }
    heads
        .into_iter()
        .map(|h| {
            // A scene runs to the next heading of any kind, an act to the
            // next act or section.
            let end = stops
                .iter()
                .filter(|(at, _)| *at > h.last)
                .find(|(_, k)| match h.kind {
                    UnitKind::Scene => true,
                    UnitKind::Act => *k != Some(UnitKind::Scene),
                })
                .map_or(body.end, |(at, _)| *at);
            let last = (h.block..end)
                .rev()
                .find(|&x| !matches!(doc.blocks[x].kind, Kind::Blank))
                .unwrap_or(h.block);
            Unit {
                kind: h.kind,
                hidden: h.hidden,
                span: doc.blocks[h.block].span.start..doc.blocks[last].span.end,
                label: h.label,
                title: h.title,
                lines: h.lines,
            }
        })
        .collect()
}

/// Hides `span`: a `%~` before each of its lines.
pub fn hide(text: &str, span: Range<usize>) -> Edit {
    let hidden: Vec<String> = text[span.clone()]
        .split('\n')
        .map(|l| {
            if l.trim().is_empty() {
                format!("{HIDDEN}{l}")
            } else {
                format!("{HIDDEN} {l}")
            }
        })
        .collect();
    Edit::new(span, hidden.join("\n"))
}

/// Brings `span` back: one `%~` off each line.
pub fn unhide(text: &str, span: Range<usize>) -> Edit {
    Edit::new(span.clone(), strip(&text[span]))
}

/// Takes `span` out, with the blank line after it.
pub fn cut(text: &str, span: Range<usize>) -> Edit {
    let mut end = span.end;
    // Up to and including one empty line after it.
    for _ in 0..2 {
        if let Some(rest) = text.get(end..) {
            let line_end = rest.find('\n').map_or(text.len(), |i| end + i + 1);
            if text[end..line_end].trim().is_empty() && line_end > end {
                end = line_end;
            } else {
                break;
            }
        }
    }
    Edit::new(span.start..end, "")
}

/// A scene or act in the trash.
#[derive(Debug, Clone, PartialEq)]
pub struct Trashed {
    /// "2026-10-26 15:50".
    pub when: String,
    /// "Sahne 2 · Blues".
    pub what: String,
    /// The line just before it in the play, and the one just after: where it
    /// goes back.
    pub after: String,
    pub before: String,
    pub text: String,
}

const MARK: &str = "%%% dactile-trash";

/// `.oyun.tex.cop` next to `oyun.tex`.
pub fn trash_path(play: &Path) -> PathBuf {
    let name = play
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    play.with_file_name(format!(".{name}.cop"))
}

pub fn load(path: &Path) -> Vec<Trashed> {
    let Ok(s) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    s.split(MARK)
        .skip(1)
        .filter_map(|rec| {
            let mut t = Trashed {
                when: String::new(),
                what: String::new(),
                after: String::new(),
                before: String::new(),
                text: String::new(),
            };
            let mut body = Vec::new();
            for line in rec.trim_start_matches(['\r', '\n']).lines() {
                if body.is_empty()
                    && let Some(kv) = line.strip_prefix("%% ")
                    && let Some((k, v)) = kv.split_once(": ")
                {
                    match k {
                        "when" => t.when = v.to_string(),
                        "what" => t.what = v.to_string(),
                        "after" => t.after = v.to_string(),
                        "before" => t.before = v.to_string(),
                        _ => {}
                    }
                    continue;
                }
                body.push(line);
            }
            t.text = body.join("\n").trim_end().to_string();
            (!t.text.is_empty()).then_some(t)
        })
        .collect()
}

pub fn save(path: &Path, items: &[Trashed]) -> anyhow::Result<()> {
    if items.is_empty() {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        return Ok(());
    }
    let mut s = String::from(
        "% Dactile: the play's trash. These scenes are no longer in the play;\n\
         % in Dactile, ^G then Tab puts them back.\n",
    );
    for t in items {
        s.push_str(&format!(
            "{MARK}\n%% when: {}\n%% what: {}\n%% after: {}\n%% before: {}\n{}\n",
            t.when, t.what, t.after, t.before, t.text
        ));
    }
    crate::file::write_atomic(path, &s)
}

/// The lines around `span` that say where it was.
pub fn neighbours(text: &str, span: Range<usize>) -> (String, String) {
    let after = text[..span.start]
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();
    let before = text[span.end..]
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();
    (after, before)
}

/// Where a trashed piece goes back: after the line it followed (the one
/// followed by the line it preceded, if that repeats), else at the end of
/// the body. Returns the edit that puts it there.
pub fn restore(text: &str, doc: &Doc, t: &Trashed) -> Edit {
    let body = doc.parts.body.clone();
    let lines = crate::tex::lines(&text[body.clone()]);
    let mut spot = None;
    if !t.after.is_empty() {
        for (n, l) in lines.iter().enumerate() {
            if text[body.start + l.start..body.start + l.end].trim() != t.after {
                continue;
            }
            let next = lines[n + 1..]
                .iter()
                .map(|x| text[body.start + x.start..body.start + x.end].trim())
                .find(|x| !x.is_empty())
                .unwrap_or("");
            let end = body.start + l.end;
            if next == t.before {
                spot = Some(end);
                break;
            }
            spot.get_or_insert(end);
        }
    }
    match spot {
        Some(at) => Edit::new(at..at, format!("\n\n{}", t.text)),
        None => {
            let at = body.end;
            let lead = if text[..at].ends_with("\n\n") {
                ""
            } else {
                "\n"
            };
            Edit::new(at..at, format!("{lead}{}\n\n", t.text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tex::apply;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    fn names(text: &str) -> Vec<String> {
        units(text, &Doc::new(text))
            .iter()
            .map(|u| format!("{}{}", if u.hidden { "~" } else { "" }, u.name()))
            .collect()
    }

    #[test]
    fn acts_and_scenes() {
        assert_eq!(
            names(PLAY),
            [
                "I. Perde · Kar Yağarken",
                "Sahne 1 · İç, Değirmen — Akşam",
                "Sahne 2 · Dış, Meydan — Gece",
                "II. Perde",
                "Sahne 2.1"
            ]
        );
        let u = units(PLAY, &Doc::new(PLAY));
        // A scene ends at its last line, an act with its last scene.
        assert!(PLAY[u[1].span.clone()].ends_with("\\middir{Rüzgârın sesi giderek artar.}"));
        assert_eq!(
            u[0].span.start,
            u[1].span.start - "#perde Kar Yağarken\n\n".len()
        );
        assert_eq!(u[0].span.end, u[2].span.end);
    }

    #[test]
    fn hiding_and_back() {
        let u = units(PLAY, &Doc::new(PLAY));
        let hidden = apply(PLAY, &[hide(PLAY, u[2].span.clone())]);
        assert!(hidden.contains("%~ #sahne Dış, Meydan — Gece\n%~\n%~ \\ka Kar dinmeyecek"));
        // Out of the outline, still a unit.
        let d = Doc::new(&hidden);
        assert_eq!(d.outline.scenes.len(), 2);
        assert_eq!(names(&hidden)[2], "~Sahne · Dış, Meydan — Gece");
        let h = units(&hidden, &d);
        assert_eq!(h[2].lines, 4);
        // Back exactly as it was.
        let back = apply(&hidden, &[unhide(&hidden, h[2].span.clone())]);
        assert_eq!(back, PLAY);
        // A hidden act takes its scenes along; a scene hidden inside stays
        // hidden when the act comes back.
        let twice = apply(&hidden, &[hide(&hidden, h[0].span.clone())]);
        let t = units(&twice, &Doc::new(&twice));
        assert_eq!(names(&twice)[0], "~Perde · Kar Yağarken");
        let act_back = apply(&twice, &[unhide(&twice, t[0].span.clone())]);
        assert_eq!(act_back, hidden);
    }

    #[test]
    fn trash_and_back() {
        let doc = Doc::new(PLAY);
        let u = units(PLAY, &doc);
        let span = u[2].span.clone();
        let (after, before) = neighbours(PLAY, span.clone());
        assert_eq!(after, "\\middir{Rüzgârın sesi giderek artar.}");
        assert_eq!(before, "\\perde[]");
        let t = Trashed {
            when: "2026-10-26 15:50".into(),
            what: u[2].name(),
            after,
            before,
            text: PLAY[span.clone()].to_string(),
        };
        let gone = apply(PLAY, &[cut(PLAY, span)]);
        assert_eq!(names(&gone).len(), 4);
        // Saved and read back.
        let dir = std::env::temp_dir().join(format!("dactile-shelf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = trash_path(&dir.join("oyun.tex"));
        assert!(path.ends_with(".oyun.tex.cop"));
        save(&path, std::slice::from_ref(&t)).unwrap();
        assert_eq!(load(&path), vec![t.clone()]);
        save(&path, &[]).unwrap();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
        // Put back where it was.
        let back = apply(&gone, &[restore(&gone, &Doc::new(&gone), &t)]);
        assert_eq!(back, PLAY);
    }
}
