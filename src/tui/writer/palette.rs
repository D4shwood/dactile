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
}

/// An entry of the command palette.
pub struct PItem {
    pub name: String,
    /// The key that does the same, if any.
    pub key: &'static str,
    pub act: PAct,
}

/// Everything the palette offers, the best matches for `q` first.
pub(super) fn palette_items(q: &str) -> Vec<PItem> {
    use KeyCode::{Char, F};
    let (c, a, n) = (KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::NONE);
    let k = |code, mods| PAct::Key(code, mods);
    let mut all: Vec<(&str, &str, &'static str, PAct)> = vec![
        ("Yeni sahne", "New scene", "#sahne", PAct::NewScene),
        ("Yeni perde", "New act", "#perde", PAct::NewAct),
        ("Yeni karakter", "New character", "", PAct::NewCharacter),
        (
            "Kişiler: düzenle, birleştir, böl",
            "Cast: edit, merge, split",
            "^O",
            k(Char('o'), c),
        ),
        (
            "Dağıtım: oyuncular, sahne/rol tablosu, sayılar",
            "Casting: actors, the scene/role table, numbers",
            "F9",
            k(F(9), n),
        ),
        (
            "Notlar: AI'dan, atölyeden, okurdan; yanda",
            "Notes from an AI, a workshop, a reader; beside",
            "^N",
            k(Char('n'), c),
        ),
        (
            "Buraya not: seçili söze ya da kelimeye",
            "A note here, on the selection or the word",
            "Alt+N",
            k(Char('n'), a),
        ),
        (
            "Notlu PDF kopyası: açık notlar kenarda",
            "A PDF copy with the open notes in the margin",
            "",
            PAct::NotesPdf,
        ),
        (
            "Sahne şeridi: yazının yanında",
            "The scene strip, beside the writing",
            "^G",
            k(Char('g'), c),
        ),
        (
            "Pano: sahneler, kartlar, taşı, gizle, çöp kutusu",
            "The board: scenes, cards, move, hide, trash",
            "^G ^G",
            PAct::Board,
        ),
        (
            "Sahne özeti",
            "The scene's summary",
            "Alt+E",
            k(Char('e'), a),
        ),
        (
            "Sahneyi imleçten böl",
            "Split the scene at the cursor",
            "Alt+B",
            k(Char('b'), a),
        ),
        (
            "Sonraki sahneyle birleştir",
            "Join with the next scene",
            "Alt+J",
            k(Char('j'), a),
        ),
        (
            "Karakter ya da komut ara",
            "Search characters and commands",
            "^K",
            k(Char('k'), c),
        ),
        ("Bul ve değiştir", "Find and replace", "^F", k(Char('f'), c)),
        (
            "Oyun bilgisi, taslak/final",
            "Cover details, draft/final",
            "Alt+İ",
            k(Char('i'), a),
        ),
        (
            "Yazım önerileri",
            "Spelling suggestions",
            "Alt+S",
            k(Char('s'), a),
        ),
        (
            "Kişi tanımlarını kaynakta aç",
            "Show the cast lines of the source",
            "Alt+P",
            k(Char('p'), a),
        ),
        (
            "Bütün oyun kaynak olarak",
            "The whole play as source",
            "F2",
            k(F(2), n),
        ),
        ("Daktilo modu", "Typewriter mode", "F3", k(F(3), n)),
        ("Sahne odağı", "Scene focus", "F4", k(F(4), n)),
        ("PDF derle", "Build the PDF", "F5", k(F(5), n)),
        ("PDF'i aç", "Open the PDF", "Alt+O", k(Char('o'), a)),
        (
            "Derleme logunu aç",
            "Open the build log",
            "Alt+L",
            k(Char('l'), a),
        ),
        ("İmleç dışını soldur", "Dim the rest", "F8", k(F(8), n)),
        (
            "Ezber: imleçteki rolün replikleri gizli",
            "Memorize: the cursor's role's lines hidden",
            "",
            PAct::Memorize,
        ),
        (
            "Rol görünümü: imleçteki rolün replikleri parlak",
            "Role view: the cursor's role's lines lit",
            "Shift+F8",
            k(F(8), KeyModifiers::SHIFT),
        ),
        ("Sayfa görünümü", "Page view", "F7", k(F(7), n)),
        (
            "Animasyonları aç/kapat",
            "Animations on/off",
            "",
            PAct::Animations,
        ),
        ("Tuşlar", "Keys", "F1", k(F(1), n)),
        ("Kaydet", "Save", "^S", k(Char('s'), c)),
        ("Geri al", "Undo", "^Z", k(Char('z'), c)),
        ("Yinele", "Redo", "^Y", k(Char('y'), c)),
        ("Hepsini seç", "Select all", "^A", k(Char('a'), c)),
        ("Çık", "Quit", "^Q", k(Char('q'), c)),
    ];
    let mut all: Vec<(String, &'static str, PAct)> = all
        .drain(..)
        .map(|(tr, en, key, act)| ((if lang::is_tr() { tr } else { en }).to_string(), key, act))
        .collect();
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
