//! ^P: the command palette's entries.

use super::*;

/// What a palette entry does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PAct {
    /// What this key does in the writer.
    Key(KeyCode, KeyModifiers),
    Theme(&'static str),
    Animations,
    NewScene,
    NewAct,
    NewCharacter,
    Board,
    Memorize,
    NotesPdf,
    NotesPaste,
    NotesFile,
    About,
    SaveTemplate,
}

/// An entry of the command palette.
pub struct PItem {
    pub name: String,
    /// The key that does the same, if any.
    pub key: &'static str,
    pub act: PAct,
}

/// Everything the palette offers, the best matches for `q` first: what the
/// menus have, each once, and the themes.
pub(super) fn palette_items(q: &str) -> Vec<PItem> {
    let mut all: Vec<(String, &'static str, PAct)> = Vec::new();
    for d in super::menu::menus() {
        for it in d.items.iter().flatten() {
            if all.iter().all(|(_, _, a)| *a != it.act) {
                all.push((format!("{} › {}", d.name(), it.name()), it.key, it.act));
            }
        }
    }
    for t in theme::THEMES {
        let name = lang::f("Tema: {}", "Theme: {}", &[&theme::theme_name(t)]);
        all.push((name, "F6", PAct::Theme(t)));
    }
    let mut out: Vec<(i32, PItem)> = all
        .into_iter()
        .filter_map(|(name, key, act)| {
            let name = name.as_str();
            let score = if q.is_empty() {
                0
            } else {
                complete::score(q, name)?
            };
            Some((
                score,
                PItem {
                    name: name.to_string(),
                    key,
                    act,
                },
            ))
        })
        .collect();
    out.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
    out.into_iter().map(|(_, i)| i).collect()
}
