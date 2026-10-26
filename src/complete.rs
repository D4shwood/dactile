//! What can follow a backslash: the play's characters, the class's commands
//! and its other macros, ranked for what is being typed. Also the verbs a
//! `\xdo` direction usually starts with.

use crate::cast::{self, Cast, Kind as CastKind};
use crate::lang;
use crate::lex::Kind;
use crate::render::Doc;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Insert this text; the cursor goes `cursor` bytes into it.
    Insert { text: String, cursor: usize },
    /// Open the scene form.
    Scene,
    /// Open the act form.
    Act,
    /// Open the new-character form.
    NewCharacter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// What the menu shows on the left: "se", "sahne".
    pub key: String,
    /// What it means: "SELİN — Köy öğretmeni", "yeni sahne".
    pub label: String,
    pub group: Group,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// Characters in the current scene.
    InScene,
    Character,
    Command,
    Macro,
}

fn insert(text: &str, cursor: usize) -> Action {
    Action::Insert {
        text: text.to_string(),
        cursor,
    }
}

/// What can follow a `#` at a line's start: the headings, in the play's
/// words. `nl` ends the line a preface or notes heading takes.
pub fn headings(nl: &str) -> Vec<Item> {
    let tr = lang::is_tr();
    let (scene, act, preface, notes) = if tr {
        ("sahne", "perde", "önsöz", "notlar")
    } else {
        ("scene", "act", "preface", "notes")
    };
    let line = |key: &str| {
        let text = format!("#{key}{nl}{nl}");
        insert(&text, text.len())
    };
    vec![
        Item {
            key: scene.into(),
            label: lang::l("yeni sahne: yer ve zaman", "new scene: place and time").into(),
            group: Group::Command,
            action: Action::Scene,
        },
        Item {
            key: act.into(),
            label: lang::l("yeni perde", "new act").into(),
            group: Group::Command,
            action: Action::Act,
        },
        Item {
            key: preface.into(),
            label: lang::l(
                "önsöz: yeni sayfa, düz yazı",
                "a preface: a new page of prose",
            )
            .into(),
            group: Group::Command,
            action: line(preface),
        },
        Item {
            key: notes.into(),
            label: lang::l("rejisöre notlar", "notes for the director").into(),
            group: Group::Command,
            action: line(notes),
        },
    ]
}

/// Every suggestion for the block at `block`, before filtering.
pub fn items(doc: &Doc, block: Option<usize>) -> Vec<Item> {
    let scene_cast: Vec<String> = block
        .and_then(|b| doc.scene_of(b))
        .map(|s| doc.outline.scenes[s].cast.clone())
        .unwrap_or_default();
    let mut out = Vec::new();
    for c in &doc.cast.characters {
        let desc = c.desc.as_deref().filter(|d| !d.trim().is_empty());
        let label = match (desc, c.kind) {
            (Some(d), _) => format!("{} — {d}", c.script),
            (None, CastKind::Hidden) => {
                lang::f("{} (tabloda yok)", "{} (not in the table)", &[&c.script])
            }
            (None, _) => c.script.clone(),
        };
        let group = if scene_cast.contains(&c.alias) {
            Group::InScene
        } else {
            Group::Character
        };
        let text = format!("\\{} ", c.alias);
        out.push(Item {
            key: c.alias.clone(),
            label,
            group,
            action: insert(&text, text.len()),
        });
    }
    // Each character's direction: \\xdo Girer.
    let speaker = block.and_then(|b| doc.speaker(b));
    for c in &doc.cast.characters {
        let text = format!("\\{}do ", c.alias);
        let group = if speaker == Some(c.alias.as_str()) {
            Group::InScene
        } else {
            Group::Macro
        };
        out.push(Item {
            key: format!("{}do", c.alias),
            label: lang::f(
                "yönerge: {} bir şey yapar",
                "direction: {} does something",
                &[&c.script],
            ),
            group,
            action: insert(&text, text.len()),
        });
    }
    let commands: [(&str, &str, Action); 8] = [
        (
            "sahne",
            lang::l(
                "yeni sahne: yer ve zaman sorulur",
                "new scene: asks for place and time",
            ),
            Action::Scene,
        ),
        ("perde", lang::l("yeni perde", "new act"), Action::Act),
        (
            "karakter",
            lang::l("yeni karakter", "new character"),
            Action::NewCharacter,
        ),
        (
            "action",
            lang::l(
                "isimsiz sahne yönü: (Işıklar söner.)",
                "a direction: (The lights fade.)",
            ),
            insert("\\action{}", 8),
        ),
        (
            "stagedir",
            lang::l("ortalanmış sahne tasviri", "a centred stage description"),
            insert("\\stagedir{}", 10),
        ),
        (
            "middir",
            lang::l("sağa yaslı kısa not", "a short note on the right"),
            insert("\\middir{}", 8),
        ),
        (
            "ic",
            lang::l(
                "replik içinde yönerge: (güler)",
                "a direction inside a line: (laughs)",
            ),
            insert("\\ic{}", 4),
        ),
        (
            "beat",
            lang::l("kısa sessizlik: (Sus.)", "a short pause"),
            insert("\\beat", 5),
        ),
    ];
    for (key, label, action) in commands {
        out.push(Item {
            key: key.into(),
            label: label.into(),
            group: Group::Command,
            action,
        });
    }
    let macros: [(&str, &str); 9] = [
        (
            "ru",
            lang::l("Rusça (Kiril) bölüm", "a Russian (Cyrillic) passage"),
        ),
        ("en", lang::l("İngilizce bölüm", "an English passage")),
        ("de", lang::l("Almanca bölüm", "a German passage")),
        ("fr", lang::l("Fransızca bölüm", "a French passage")),
        ("el", lang::l("Yunanca bölüm", "a Greek passage")),
        ("textit", lang::l("italik", "italics")),
        ("textbf", lang::l("kalın", "bold")),
        (
            "setinlinename",
            lang::l(
                "replik içinde geçen adı değiştir",
                "the name used inside lines",
            ),
        ),
        ("clearpage", lang::l("yeni sayfa", "new page")),
    ];
    for (key, label) in macros {
        let text = if key == "clearpage" {
            format!("\\{key}")
        } else {
            format!("\\{key}{{}}")
        };
        let cursor = text.len() - usize::from(key != "clearpage");
        out.push(Item {
            key: key.into(),
            label: label.into(),
            group: Group::Macro,
            action: insert(&text, cursor),
        });
    }
    out
}

/// The items matching `query`, best first.
pub fn filter(items: &[Item], query: &str) -> Vec<Item> {
    let mut scored: Vec<(i32, &Item)> = items
        .iter()
        .filter_map(|it| {
            let by_key = score(query, &it.key);
            let by_label = score(query, &it.label).map(|s| s - 20);
            by_key.max(by_label).map(|s| (s, it))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.group.cmp(&b.1.group)));
    scored.into_iter().map(|(_, it)| it.clone()).collect()
}

/// How well `query` matches `target` as a subsequence, letters folded to
/// ASCII; None when it doesn't. An exact match, a prefix and matches at word
/// starts score higher.
pub fn score(query: &str, target: &str) -> Option<i32> {
    let q: Vec<char> = cast::fold_ascii(query).chars().collect();
    if q.is_empty() {
        return Some(0);
    }
    // Fold letter by letter, keeping spaces as word breaks.
    let t: Vec<char> = cast::lower_tr(target)
        .chars()
        .map(|c| {
            cast::fold_ascii(&c.to_string())
                .chars()
                .next()
                .unwrap_or(' ')
        })
        .collect();
    let mut qi = 0;
    let mut total = 0;
    let mut prev_hit: Option<usize> = None;
    for (ti, &c) in t.iter().enumerate() {
        if qi < q.len() && c == q[qi] {
            let mut s = 1;
            if ti == 0 || t[ti - 1] == ' ' {
                s += 8;
            }
            if prev_hit == Some(ti.wrapping_sub(1)) {
                s += 5;
            }
            total += s;
            prev_hit = Some(ti);
            qi += 1;
        }
    }
    if qi < q.len() {
        return None;
    }
    let tq: String = t.iter().collect();
    let qs: String = q.iter().collect();
    if tq == qs {
        total += 100;
    } else if tq.starts_with(&qs) {
        total += 40;
    }
    Some(total - t.len() as i32 / 4)
}

/// Verbs a `\xdo` direction starts with: the play's own first, most used
/// first, then common ones.
pub fn verbs(doc: &Doc, text: &str) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for b in &doc.blocks[doc.body.clone()] {
        if let Kind::Do { text: t, .. } = &b.kind {
            let first = text[t.clone()]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            if first.chars().next().is_some_and(char::is_alphabetic) {
                match counts.iter_mut().find(|(v, _)| *v == first) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((first, 1)),
                }
            }
        }
    }
    counts.sort_by_key(|c| std::cmp::Reverse(c.1));
    let mut out: Vec<String> = counts.into_iter().map(|(v, _)| v).collect();
    let common: &[&str] = if lang::is_tr() {
        &[
            "Girer.", "Çıkar.", "Oturur.", "Kalkar.", "Güler.", "Susar.", "Ağlar.", "Bakar.",
            "Döner.", "Bekler.",
        ]
    } else {
        &[
            "Enters.", "Exits.", "Sits.", "Stands.", "Laughs.", "Pauses.", "Cries.", "Looks.",
            "Turns.", "Waits.",
        ]
    };
    for v in common {
        if !out.iter().any(|o| o == v) {
            out.push(v.to_string());
        }
    }
    out
}

/// Words used before, for a field: earlier scenes' places.
pub fn places(doc: &Doc) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in doc.outline.scenes.iter().rev() {
        let place = s.place.split(" — ").next().unwrap_or("").trim().to_string();
        if !place.is_empty() && !out.contains(&place) {
            out.push(place);
        }
    }
    out
}

/// Times of day used in earlier scenes, then the usual ones.
pub fn times(doc: &Doc) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in doc.outline.scenes.iter().rev() {
        if let Some((_, time)) = s.place.split_once(" — ") {
            let time = time.trim().to_string();
            if !time.is_empty() && !out.contains(&time) {
                out.push(time);
            }
        }
    }
    let usual: &[&str] = if lang::is_tr() {
        &["Gece", "Sabah", "Öğle", "Akşam", "Gün batımı", "Şafak"]
    } else {
        &["Night", "Morning", "Noon", "Evening", "Sunset", "Dawn"]
    };
    for t in usual {
        if !out.iter().any(|o| o == t) {
            out.push(t.to_string());
        }
    }
    out
}

/// Whether `alias` names a character in the play.
pub fn is_character(cast: &Cast, alias: &str) -> bool {
    cast.get(alias).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    #[test]
    fn heading_menu() {
        let h = headings("\n");
        let keys: Vec<&str> = h.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(keys, ["sahne", "perde", "önsöz", "notlar"]);
        assert_eq!(h[0].action, Action::Scene);
        assert_eq!(
            h[2].action,
            Action::Insert {
                text: "#önsöz\n\n".into(),
                cursor: 10
            }
        );
    }

    #[test]
    fn scoring() {
        assert!(score("se", "se").unwrap() > score("se", "sedo").unwrap());
        assert!(score("sah", "sahne").is_some());
        assert!(score("sne", "sahne").is_some());
        assert!(score("xq", "sahne").is_none());
        // Turkish letters fold: "ih" finds İHTİYAR, "coban" ÇOBAN.
        assert!(score("ih", "İHTİYAR").is_some());
        assert!(score("coban", "ÇOBAN").is_some());
    }

    #[test]
    fn suggestions() {
        let doc = Doc::new(PLAY);
        let at = PLAY.find("\\se Amca").unwrap();
        let block = doc.block_at(at);
        let all = items(&doc, block);
        let got = filter(&all, "s");
        // Characters of the scene come before the rest at equal score.
        assert_eq!(got[0].key, "se");
        assert!(got.iter().any(|i| i.key == "sahne"));
        assert!(got.iter().any(|i| i.key == "sedo"));
        let got = filter(&all, "oza");
        assert_eq!(got[0].key, "oz");
        assert_eq!(
            got[0].action,
            Action::Insert {
                text: "\\oz ".into(),
                cursor: 4
            }
        );
        let got = filter(&all, "act");
        assert_eq!(
            got[0].action,
            Action::Insert {
                text: "\\action{}".into(),
                cursor: 8
            }
        );
    }

    #[test]
    fn verbs_and_places() {
        let doc = Doc::new(PLAY);
        let v = verbs(&doc, PLAY);
        assert_eq!(&v[..3], &["Kapıyı", "Birbirlerine", "Çıkar."]);
        assert!(v.contains(&"Girer.".to_string()));
        assert_eq!(places(&doc), vec!["Dış, Meydan", "İç, Değirmen"]);
        assert_eq!(&times(&doc)[..2], &["Gece", "Akşam"]);
    }
}
