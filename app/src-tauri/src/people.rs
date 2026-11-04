//! The cast panel (Ctrl+O): who is in the play and how much each one says,
//! and the changes the terminal app's panel makes, through core's `cast`.
//! Each change comes back as one edit of the text, for one undo step.

use crate::play::Change;
use crate::view::{byte_at, unit_at};
use dactile::cast::{self as dcast, Cast, Kind, Row as CastRow, Spec};
use dactile::lang;
use dactile::lex::{self, Kind as BlockKind};
use dactile::render::Doc;
use dactile::tex::{self, Edit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    /// A character, or a group's heading (its members follow it).
    pub group: bool,
    pub alias: String,
    /// The name before a line, or the group's name.
    pub name: String,
    pub full: String,
    pub desc: String,
    pub star: bool,
    /// "cast", "plain", "hidden" or "member".
    pub kind: &'static str,
    pub lines: usize,
    pub words: usize,
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Cast => "cast",
        Kind::Plain => "plain",
        Kind::Hidden => "hidden",
        Kind::Member => "member",
    }
}

/// Lines and spoken words of each character, by index into the cast.
fn stats(doc: &Doc, text: &str) -> Vec<(usize, usize)> {
    let mut out = vec![(0, 0); doc.cast.characters.len()];
    for b in &doc.blocks[doc.body.clone()] {
        if let BlockKind::Line { alias, text: t } = &b.kind
            && let Some(i) = doc.cast.index(alias)
        {
            out[i].0 += 1;
            out[i].1 += lex::spoken_words(text, t.clone(), &doc.cast);
        }
    }
    out
}

/// The cast as the table lists it, or by how much each one speaks.
pub fn list(text: &str, by_lines: bool) -> Vec<Row> {
    let doc = Doc::new(text);
    let st = stats(&doc, text);
    let cast = &doc.cast;
    let plain = |s: &str| tex::unescape_text(s);
    let person = |i: usize| {
        let c = &cast.characters[i];
        Row {
            group: false,
            alias: c.alias.clone(),
            name: plain(&c.script),
            full: plain(c.full.as_deref().unwrap_or("")),
            desc: plain(c.desc.as_deref().unwrap_or("")),
            star: c.star,
            kind: kind_name(c.kind),
            lines: st[i].0,
            words: st[i].1,
        }
    };
    if by_lines {
        let mut idx: Vec<usize> = (0..cast.characters.len()).collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(st[i].0));
        return idx.into_iter().map(person).collect();
    }
    let mut out = Vec::new();
    for r in cast.rows() {
        match r {
            CastRow::Character(i) => out.push(person(i)),
            CastRow::Group(g) => {
                let grp = &cast.groups[g];
                out.push(Row {
                    group: true,
                    alias: format!("#{g}"),
                    name: plain(&grp.desc),
                    full: String::new(),
                    desc: String::new(),
                    star: false,
                    kind: "group",
                    lines: grp.members.iter().map(|&m| st[m].0).sum(),
                    words: grp.members.iter().map(|&m| st[m].1).sum(),
                });
                out.extend(grp.members.iter().map(|&m| person(m)));
            }
        }
    }
    // Speakers declared outside the table (hidden, plain) after it.
    for (i, c) in cast.characters.iter().enumerate() {
        if !out.iter().any(|r| !r.group && r.alias == c.alias) {
            out.push(person(i));
        }
    }
    out
}

/// What the character form sends: plain text, escaped here.
#[derive(Debug, Clone, Deserialize)]
pub struct Form {
    pub name: String,
    pub alias: String,
    pub full: String,
    pub desc: String,
    pub star: bool,
    pub in_table: bool,
    pub hidden: bool,
}

pub fn suggest_alias(text: &str, name: &str) -> String {
    Cast::of_play(text).suggest_alias(name)
}

fn spec(cast: &Cast, f: &Form, old: Option<&str>) -> Result<Spec, String> {
    let name = f.name.trim();
    let alias = f.alias.trim();
    if name.is_empty() {
        return Err(lang::l("Ad boş olamaz.", "The name can't be empty.").into());
    }
    if alias.is_empty() || !alias.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(lang::l(
            "Kısaltma yalnız İngilizce harflerden oluşmalı (ş, ç… olmadan).",
            "The alias takes plain letters a–z only.",
        )
        .into());
    }
    if Some(alias) != old && cast.alias_taken(alias) {
        return Err(lang::f(
            "\\{} kullanılıyor ya da bir LaTeX komutu; başka bir kısaltma seç.",
            "\\{} is taken, or a LaTeX command: pick another alias.",
            &[&alias],
        ));
    }
    let member = old
        .and_then(|o| cast.get(o))
        .is_some_and(|c| c.kind == Kind::Member);
    Ok(Spec {
        alias: alias.to_string(),
        script: tex::escape_text(name),
        full: tex::escape_text(f.full.trim()),
        desc: tex::escape_text(f.desc.trim()),
        star: f.star,
        kind: if member {
            Kind::Member
        } else if f.hidden {
            Kind::Hidden
        } else if f.in_table {
            Kind::Cast
        } else {
            Kind::Plain
        },
    })
}

/// The one change that turns `old` into `new`: what lies between their
/// common start and end.
pub fn diff(old: &str, new: &str) -> Change {
    let start = old
        .char_indices()
        .zip(new.chars())
        .find(|((_, a), b)| a != b)
        .map_or(old.len().min(new.len()), |((i, _), _)| i);
    let max = old.len().min(new.len()) - start;
    let mut end = 0;
    let (o, n) = (old.as_bytes(), new.as_bytes());
    while end < max && o[o.len() - 1 - end] == n[n.len() - 1 - end] {
        end += 1;
    }
    // Back to a character boundary in both.
    while !old.is_char_boundary(old.len() - end) || !new.is_char_boundary(new.len() - end) {
        end -= 1;
    }
    let insert = new[start..new.len() - end].to_string();
    let from = unit_at(old, start);
    Change {
        from,
        to: unit_at(old, old.len() - end),
        at: from + insert.encode_utf16().count(),
        insert,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Made {
    pub change: Change,
    /// Where the cursor goes, in the new text, when it should move.
    pub cursor: Option<usize>,
    pub message: String,
}

/// A new character, declared with the others; with `at` (from the menu),
/// its first line starts there.
pub fn add(text: &str, f: &Form, at: Option<usize>) -> Result<Made, String> {
    let cast = Cast::of_play(text);
    let spec = spec(&cast, f, None)?;
    let decl = dcast::add(text, &cast, &spec, None);
    let mut edits = vec![decl.clone()];
    let mut cursor = None;
    if let Some(at) = at {
        let at = byte_at(text, at);
        let use_text = format!("\\{} ", spec.alias);
        let mut c = at + use_text.len();
        if decl.range.start <= at {
            c += decl.text.len();
        }
        edits.push(Edit::new(at..at, use_text));
        cursor = Some(c);
    }
    let new = tex::apply(text, &edits);
    Ok(Made {
        cursor: cursor.map(|c| unit_at(&new, c)),
        change: diff(text, &new),
        message: lang::f(
            "{} eklendi: \\{}",
            "{} added: \\{}",
            &[&f.name.trim(), &spec.alias],
        ),
    })
}

/// How many places a rename would change, to ask first.
pub fn renames(text: &str, old: &str, new: &str) -> usize {
    dcast::rename(text, &Cast::of_play(text), old, new).len()
}

/// A character changed, its alias renamed everywhere when that changed.
pub fn edit(text: &str, old: &str, f: &Form) -> Result<Made, String> {
    let cast = Cast::of_play(text);
    let spec = spec(&cast, f, Some(old))?;
    let mut keep = spec.clone();
    keep.alias = old.to_string();
    let mut t = text.to_string();
    if let Some(e) = dcast::update(&cast, old, &keep) {
        t = tex::apply(&t, &[e]);
    }
    if spec.alias != old {
        let c = Cast::of_play(&t);
        t = tex::apply(&t, &dcast::rename(&t, &c, old, &spec.alias));
    }
    Ok(Made {
        change: diff(text, &t),
        cursor: None,
        message: String::new(),
    })
}

/// A character no one speaks as any more, taken out.
pub fn remove(text: &str, alias: &str) -> Result<Made, String> {
    let cast = Cast::of_play(text);
    let (lines, dos) = dcast::uses(text, dactile::source::split(text).body, alias);
    if lines + dos > 0 {
        return Err(lang::f(
            "\\{} oyunda {} kez geçiyor; önce o satırları değiştir ya da birleştir.",
            "\\{} is used {} times; change those lines first, or merge it.",
            &[&alias, &(lines + dos)],
        ));
    }
    let e = dcast::remove(text, &cast, alias).ok_or("no such character")?;
    let t = tex::apply(text, &[e]);
    Ok(Made {
        change: diff(text, &t),
        cursor: None,
        message: lang::f(
            "\\{} silindi (Ctrl+Z geri alır).",
            "\\{} deleted (Ctrl+Z undoes).",
            &[&alias],
        ),
    })
}

/// What `from` says and does becomes `into`'s; `from` goes.
pub fn merge(text: &str, from: &str, into: &str) -> Result<Made, String> {
    let cast = Cast::of_play(text);
    if from == into || cast.get(into).is_none() {
        return Err(lang::l(
            "Birleştirilecek karakteri seç.",
            "Pick the character to merge into.",
        )
        .into());
    }
    let (lines, dos) = dcast::uses(text, dactile::source::split(text).body, from);
    let t = tex::apply(text, &dcast::merge(text, &cast, from, into));
    Ok(Made {
        change: diff(text, &t),
        cursor: None,
        message: lang::f(
            "\\{} → \\{}: {} yer geçti (Ctrl+Z geri alır).",
            "\\{} → \\{}: {} places moved (Ctrl+Z undoes).",
            &[&from, &into, &(lines + dos)],
        ),
    })
}

/// A row of the table (a character, or a group by `#g`) a place up or down.
pub fn shift(text: &str, alias: &str, up: bool) -> Option<Made> {
    let cast = Cast::of_play(text);
    let edits = if let Some(g) = alias.strip_prefix('#') {
        let g: usize = g.parse().ok()?;
        let row = cast.rows().iter().position(|r| *r == CastRow::Group(g))?;
        dcast::move_row(text, &cast, row, up)?
    } else {
        let c = cast.get(alias)?;
        if c.kind == Kind::Member {
            dcast::move_member(&cast, alias, up, text)?
        } else {
            let i = cast.index(alias)?;
            let row = cast
                .rows()
                .iter()
                .position(|r| *r == CastRow::Character(i))?;
            dcast::move_row(text, &cast, row, up)?
        }
    };
    let t = tex::apply(text, &edits);
    Some(Made {
        change: diff(text, &t),
        cursor: None,
        message: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    fn apply(text: &str, c: &Change) -> String {
        let u: Vec<u16> = text.encode_utf16().collect();
        let mut out = u[..c.from].to_vec();
        out.extend(c.insert.encode_utf16());
        out.extend(&u[c.to..]);
        String::from_utf16(&out).unwrap()
    }

    fn form(name: &str, alias: &str) -> Form {
        Form {
            name: name.into(),
            alias: alias.into(),
            full: String::new(),
            desc: "Bir çoban".into(),
            star: false,
            in_table: true,
            hidden: false,
        }
    }

    #[test]
    fn the_cast_with_how_much_each_says() {
        let rows = list(PLAY, true);
        assert!(rows[0].lines >= rows[1].lines);
        assert!(rows.iter().any(|r| r.alias == "se" && r.name == "SELİN"));
    }

    #[test]
    fn a_new_character_and_its_first_line() {
        let at = PLAY.find("\\oz Ben öyle").unwrap();
        let at = unit_at(PLAY, at);
        let m = add(PLAY, &form("ÇOBAN", "cob"), Some(at)).unwrap();
        let t = apply(PLAY, &m.change);
        assert!(t.contains("\\karakter[Bir çoban]{ÇOBAN}{cob}"));
        assert!(t.contains("\\cob \\oz Ben öyle"));
        let c = m.cursor.unwrap();
        let u: Vec<u16> = t.encode_utf16().collect();
        assert_eq!(String::from_utf16(&u[c - 5..c]).unwrap(), "\\cob ");
        assert!(add(PLAY, &form("X", "se"), None).is_err());
        assert!(add(PLAY, &form("X", "çö"), None).is_err());
    }

    #[test]
    fn rename_merge_remove() {
        let m = edit(
            PLAY,
            "oz",
            &Form {
                name: "OZAN".into(),
                ..form("", "ozan")
            },
        )
        .unwrap();
        let t = apply(PLAY, &m.change);
        assert!(t.contains("\\ozan Ben öyle") && !t.contains("\\oz "));
        assert!(remove(PLAY, "se").is_err());
        let m = merge(PLAY, "ih", "oz").unwrap();
        let t = apply(PLAY, &m.change);
        assert!(!t.contains("{ih}"));
        assert!(Cast::of_play(&t).get("ih").is_none());
    }
}
