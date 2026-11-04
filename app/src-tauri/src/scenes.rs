//! Splitting a scene at the cursor (Alt+B) and joining it with the next
//! (Alt+J), through core's `structure`, as the terminal app does: the
//! second scene's summary joins the first's, and the play as it was is kept
//! once as a draft beside it.

use crate::people::{Made, diff};
use crate::view::{byte_at, unit_at};
use dactile::render::Doc;
use dactile::sidecar::Sidecar;
use dactile::{lang, library, structure, tex};
use std::path::{Path, PathBuf};

/// The play as it was, kept once as a draft before its shape changes:
/// what to add to the message.
pub fn keep_shape(play: &Path, text: &str, kept: &mut Option<PathBuf>) -> String {
    if kept.is_some() {
        return String::new();
    }
    match library::keep_draft(play, text) {
        Ok(p) => {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            *kept = Some(p);
            lang::f(
                " Önceki hâli taslak olarak saklandı: {}",
                " The play as it was is kept as a draft: {}",
                &[&name],
            )
        }
        Err(e) => format!(" {e:#}"),
    }
}

/// A new scene from the line at UTF-16 `at` on.
pub fn split(
    text: &str,
    at: usize,
    play: &Path,
    kept: &mut Option<PathBuf>,
) -> Result<Made, String> {
    let doc = Doc::new(text);
    let (e, cursor) = structure::split(text, &doc, byte_at(text, at)).ok_or_else(|| {
        lang::l(
            "Bölmek için imleci sahnenin içinde, yeni sahnenin başlayacağı satıra koy.",
            "To split, put the cursor in the scene, on the line the new one starts with.",
        )
        .to_string()
    })?;
    let new = tex::apply(text, &[e]);
    let note = keep_shape(play, text, kept);
    Ok(Made {
        change: diff(text, &new),
        cursor: Some(unit_at(&new, cursor)),
        message: lang::l(
            "Sahne ikiye bölündü; imleç yeni başlıkta, yerini değiştirebilirsin.",
            "The scene is split in two; the cursor is on the new heading, to change its place.",
        )
        .to_string()
            + &note,
    })
}

/// The cursor's scene and the one after it as one; their summaries join.
pub fn join(
    text: &str,
    at: usize,
    play: &Path,
    kept: &mut Option<PathBuf>,
) -> Result<Made, String> {
    let doc = Doc::new(text);
    let none = || {
        lang::l(
            "Birleşecek bir sonraki sahne yok (aynı perdede, hemen ardından).",
            "No next scene to join (in the same act, right after it).",
        )
        .to_string()
    };
    let k = doc
        .block_at(byte_at(text, at))
        .and_then(|b| doc.scene_of(b))
        .ok_or_else(none)?;
    let (e, second) = structure::merge(text, &doc, k).ok_or_else(none)?;
    let scene = &doc.outline.scenes[k];
    let head = doc.blocks[doc.body.start + scene.block].span.start;
    let mut edits = vec![e];
    let mut side = Sidecar::load(play).unwrap_or_default();
    let theirs = second
        .as_ref()
        // Kept under its own id too: an undo brings the scene back.
        .and_then(|id| side.summaries.get(id).cloned())
        .filter(|s| !s.trim().is_empty());
    if let Some(theirs) = theirs {
        let id = scene.id.clone().unwrap_or_else(|| {
            let id = structure::new_id(text);
            edits.push(structure::give_id(text, head, &id));
            id
        });
        let mine = side.summaries.get(&id).cloned().unwrap_or_default();
        let joined = format!("{} {}", mine.trim(), theirs.trim())
            .trim()
            .to_string();
        side.summaries.insert(id, joined);
        side.save(play).map_err(|e| format!("{e:#}"))?;
    }
    let label = scene.title();
    let new = tex::apply(text, &edits);
    let note = keep_shape(play, text, kept);
    Ok(Made {
        change: diff(text, &new),
        cursor: None,
        message: lang::f(
            "{} ile sonraki birleşti; ikincinin yeri yönerge oldu.{}",
            "{} and the next are one; the second's place became a direction.{}",
            &[&label, &note],
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    fn apply(text: &str, m: &Made) -> String {
        let u: Vec<u16> = text.encode_utf16().collect();
        let c = &m.change;
        let mut t = u[..c.from].to_vec();
        t.extend(c.insert.encode_utf16());
        t.extend(&u[c.to..]);
        String::from_utf16(&t).unwrap()
    }

    #[test]
    fn split_then_join_is_one_scene_again() {
        let dir = std::env::temp_dir().join(format!("dactile-scenes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let play = dir.join("oyun.tex");
        std::fs::write(&play, PLAY).unwrap();
        let mut kept = None;
        let scenes = |t: &str| Doc::new(t).outline.scenes.len();
        let at = unit_at(PLAY, PLAY.find("\\oz Ben öyle").unwrap());
        let m = split(PLAY, at, &play, &mut kept).unwrap();
        let t = apply(PLAY, &m);
        assert_eq!(scenes(&t), scenes(PLAY) + 1);
        // Kept once.
        let draft = kept.clone().unwrap();
        assert!(draft.exists());
        let m = join(
            &t,
            unit_at(&t, t.find("Amca, yine").unwrap()),
            &play,
            &mut kept,
        )
        .unwrap();
        assert_eq!(kept, Some(draft));
        assert_eq!(scenes(&apply(&t, &m)), scenes(PLAY));
        assert!(split(PLAY, 0, &play, &mut kept).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
