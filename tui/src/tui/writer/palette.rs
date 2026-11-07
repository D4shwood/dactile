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
    Docx,
    /// Which part of the cover or cast page to restyle.
    CoverLook,
    NotesPaste,
    NotesFile,
    About,
    /// The Help tab: what the commands do, beside the writing.
    Guide,
    SaveTemplate,
    /// A snippet on a line of its own, or at the cursor; `‸` is where the
    /// cursor goes.
    Insert(&'static str),
    InsertInline(&'static str),
    FoldRibbon,
    /// Dosya's page, on the play's details.
    Info,
    /// The play's own settings.
    PlaySettings,
    /// What belongs to the play.
    Files,
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
    for t in super::ribbon::tabs() {
        for b in t.btns() {
            if all.iter().all(|(_, _, a)| *a != b.act) {
                all.push((format!("{} › {}", t.name(), b.name()), b.key, b.act));
            }
        }
    }
    let file = super::ribbon::tabs()[super::ribbon::FILE_TAB].name();
    for (tr, en, key, act) in super::ribbon::backstage() {
        if all.iter().all(|(_, _, a)| *a != act) {
            let name = if lang::is_tr() { tr } else { en };
            all.push((format!("{file} › {name}"), key, act));
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
