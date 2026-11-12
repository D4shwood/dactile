//! Scene summaries (Alt+E): a line or two about each scene, kept in the
//! play's `.dactile.toml` by the id at its heading (`#sahne Meydan %@k3`),
//! never printed. Shown faint under the headings and on the board.

use crate::play::Change;
use crate::view::{byte_at, unit_at};
use dactile::lang;
use dactile::render::Doc;
use dactile::sidecar::Sidecar;
use dactile::structure;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// Where the scene's heading starts and its first line ends.
    pub at: usize,
    pub end: usize,
    pub text: String,
}

fn line_end(text: &str, at: usize) -> usize {
    text[at..].find('\n').map_or(text.len(), |n| at + n)
}

/// Each scene's heading start, in bytes.
fn headings(doc: &Doc) -> Vec<usize> {
    doc.outline
        .scenes
        .iter()
        .map(|s| doc.blocks[doc.body.start + s.block].span.start)
        .collect()
}

/// The summaries of the scenes that have one.
pub fn list(text: &str, side: &Sidecar) -> Vec<Summary> {
    let doc = Doc::new(text);
    headings(&doc)
        .into_iter()
        .filter_map(|at| {
            let end = line_end(text, at);
            let id = structure::id_in(&text[at..end])?;
            let s = side.summaries.get(id)?;
            Some(Summary {
                at: unit_at(text, at),
                end: unit_at(text, end.min(text.len())),
                text: s.clone(),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct Here {
    /// The heading's start.
    pub at: usize,
    /// "Sahne 2 — Meydan".
    pub title: String,
    pub text: String,
}

/// The scene the cursor (UTF-16 `at`) is in, and its summary so far.
pub fn here(text: &str, at: usize, side: &Sidecar) -> Result<Here, String> {
    let doc = Doc::new(text);
    let byte = byte_at(text, at);
    let k = doc
        .block_at(byte)
        .and_then(|b| doc.scene_of(b))
        .ok_or_else(|| {
            lang::l(
                "İmleç bir sahnenin içinde değil.",
                "The cursor is in no scene.",
            )
            .to_string()
        })?;
    let start = headings(&doc)[k];
    let end = line_end(text, start);
    let scene = &doc.outline.scenes[k];
    let now = structure::id_in(&text[start..end])
        .and_then(|id| side.summaries.get(id))
        .cloned()
        .unwrap_or_default();
    Ok(Here {
        at: unit_at(text, start),
        title: {
            let label = if scene.label.chars().all(|c| c.is_ascii_digit()) {
                format!("{} {}", lang::l("Sahne", "Scene"), scene.label)
            } else {
                scene.label.clone()
            };
            match scene.place.trim() {
                "" => label,
                place => format!("{label} — {place}"),
            }
        },
        text: now,
    })
}

/// The summary written for the heading at `at`: kept beside the play, the
/// heading given an id first when it has none (the change to make).
pub fn set(text: &str, at: usize, summary: &str, play: &Path) -> anyhow::Result<Option<Change>> {
    let at = byte_at(text, at);
    let end = line_end(text, at);
    let summary = summary.trim();
    let mut change = None;
    let id = match structure::id_in(&text[at..end]) {
        Some(id) => id.to_string(),
        None if summary.is_empty() => return Ok(None),
        None => {
            let id = structure::new_id(text);
            let e = structure::give_id(text, at, &id);
            let from = unit_at(text, e.range.start);
            change = Some(Change {
                from,
                to: unit_at(text, e.range.end),
                at: from,
                insert: e.text,
            });
            id
        }
    };
    let mut side = Sidecar::load(play)?;
    if summary.is_empty() {
        side.summaries.remove(&id);
    } else {
        side.summaries.insert(id, summary.to_string());
    }
    side.save(play)?;
    Ok(change)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

    #[test]
    fn a_summary_gives_its_scene_an_id_and_comes_back() {
        let dir = std::env::temp_dir().join(format!("dactile-summary-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let play = dir.join("oyun.tex");
        std::fs::write(&play, PLAY).unwrap();
        let at = unit_at(PLAY, PLAY.find("Amca, yine").unwrap());
        let h = here(PLAY, at, &Sidecar::default()).unwrap();
        assert!(h.text.is_empty());
        let c = set(PLAY, h.at, "Taş durur, ihtiyar gelir.", &play)
            .unwrap()
            .unwrap();
        let u: Vec<u16> = PLAY.encode_utf16().collect();
        let mut t = u[..c.from].to_vec();
        t.extend(c.insert.encode_utf16());
        t.extend(&u[c.to..]);
        let t = String::from_utf16(&t).unwrap();
        assert!(t.contains("%@k1"));
        let side = Sidecar::load(&play).unwrap();
        let l = list(&t, &side);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].text, "Taş durur, ihtiyar gelir.");
        assert_eq!(
            here(&t, at, &side).unwrap().text,
            "Taş durur, ihtiyar gelir."
        );
        // Emptied: gone, the id stays.
        assert!(set(&t, l[0].at, " ", &play).unwrap().is_none());
        assert!(list(&t, &Sidecar::load(&play).unwrap()).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
