//! The notes beside a play (`oyun.notlar.md`, core's `notes`), for the
//! notes panel: each note with where it is in the text, in UTF-16 units.

use crate::play::Change;
use crate::view::unit_at;
use dactile::lang;
use dactile::notes::{self, Index, Note, Notes, Refused, Reply, State};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NoteCard {
    pub i: usize,
    pub id: String,
    /// "open", "done" or "hidden".
    pub state: &'static str,
    pub archived: bool,
    pub author: String,
    pub text: String,
    pub group: String,
    pub line: Option<usize>,
    pub replies: Vec<(String, String)>,
    /// Where it is in the text, when it is found.
    pub from: Option<usize>,
    pub to: Option<usize>,
    /// What it points at seems put right.
    pub gone: bool,
    /// It suggests a “wrong” → right change.
    pub fixable: bool,
}

fn state_name(s: State) -> &'static str {
    match s {
        State::Open => "open",
        State::Done => "done",
        State::Hidden => "hidden",
    }
}

pub fn cards(text: &str, notes: &Notes) -> Vec<NoteCard> {
    let index = Index::new(text);
    notes
        .list
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let anchor = index.anchor(n);
            NoteCard {
                i,
                id: n.id.clone(),
                state: state_name(n.state),
                archived: n.archived,
                author: n.author.clone(),
                text: n.text.clone(),
                group: n.group.clone(),
                line: anchor.as_ref().map(|a| a.line).or(n.line),
                replies: n
                    .replies
                    .iter()
                    .map(|r| (r.author.clone(), r.text.clone()))
                    .collect(),
                from: anchor.as_ref().map(|a| unit_at(text, a.range.start)),
                to: anchor.as_ref().map(|a| unit_at(text, a.range.end)),
                gone: index.gone(n),
                fixable: !n.fixes().is_empty(),
            }
        })
        .collect()
}

/// A note at UTF-16 offset `at` of `text`.
pub fn add(
    notes: &mut Notes,
    text: &str,
    at: usize,
    author: &str,
    body: &str,
) -> anyhow::Result<()> {
    let body = body.trim();
    anyhow::ensure!(
        !body.trim_matches(['“', '”', ' ']).is_empty(),
        "{}",
        lang::l("Not boş.", "The note is empty.")
    );
    let byte = crate::view::byte_at(text, at);
    notes.list.push(Note {
        line: Some(Index::new(text).line_of(byte)),
        author: author.trim().to_string(),
        text: body.to_string(),
        ..Note::default()
    });
    notes.name_all();
    Ok(())
}

/// The note a command means: by its id, or (from a window older than ids)
/// by where it is in the list.
pub fn which(notes: &Notes, id: Option<&str>, i: Option<usize>) -> anyhow::Result<usize> {
    let found = match id {
        Some(id) => notes.find(id),
        None => i.filter(|&i| i < notes.list.len()),
    };
    found.ok_or_else(|| {
        anyhow::anyhow!(
            "{}",
            lang::l(
                "Bu not artık yok: başkası silmiş ya da arşivden kaldırmış olabilir.",
                "This note is gone: someone else may have deleted it.",
            )
        )
    })
}

/// Who writes a note: on a site, the person signed in (the gate names
/// them); elsewhere, whoever the window says.
pub fn author(said: &str) -> String {
    let site = std::env::var("DACTILE_WHO").unwrap_or_default();
    let name = if site.trim().is_empty() { said } else { &site };
    name.chars()
        .filter(|c| !c.is_control() && *c != '*')
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn set_state(notes: &mut Notes, i: usize, state: &str) -> anyhow::Result<()> {
    let n = notes
        .list
        .get_mut(i)
        .ok_or_else(|| anyhow::anyhow!("no note {i}"))?;
    match state {
        "open" => n.state = State::Open,
        "done" => n.state = State::Done,
        "hidden" => n.state = State::Hidden,
        "archive" => n.archived = true,
        "unarchive" => n.archived = false,
        "delete" => {
            notes.list.remove(i);
        }
        _ => anyhow::bail!("no state {state}"),
    }
    Ok(())
}

pub fn reply(notes: &mut Notes, i: usize, author: &str, text: &str) -> anyhow::Result<()> {
    let n = notes
        .list
        .get_mut(i)
        .ok_or_else(|| anyhow::anyhow!("no note {i}"))?;
    anyhow::ensure!(
        !text.trim().is_empty(),
        "{}",
        lang::l("Cevap boş.", "The reply is empty.")
    );
    n.replies.push(Reply {
        author: author.trim().to_string(),
        text: text.trim().to_string(),
    });
    Ok(())
}

/// Notes out of a pasted reply (`139: …` lines) or a notes file, taken in:
/// those already here bring only their new replies.
pub fn paste(notes: &mut Notes, reply: &str, author: &str) -> notes::Merged {
    notes.merge(notes::from_reply(reply, author.trim()))
}

/// A note's “wrong” → right change, as changes to make in the editor, and
/// the note marked done. Never one that would break the play.
pub fn fix(notes: &mut Notes, text: &str, i: usize) -> Result<Vec<Change>, String> {
    let note = notes.list.get(i).ok_or("no note")?.clone();
    let edits = notes::checked_fixes(text, &note).map_err(|r| match r {
        Refused::Nothing => lang::l(
            "Bu notta uygulanacak öneri yok (“yanlış” → doğru gibi).",
            "This note suggests nothing to apply (“wrong” → right).",
        )
        .to_string(),
        Refused::NotFound => lang::l(
            "Önerinin yanlışı metinde bulunamadı: belki düzeltilmiş.",
            "The words to change aren't in the text: put right already?",
        )
        .to_string(),
        Refused::Breaks { what, line } => lang::f(
            "Uygulanmadı: {}. satırda {} Elle düzelt.",
            "Not applied: at line {}, {} Fix it by hand.",
            &[&line, &what],
        ),
    })?;
    notes.list[i].state = State::Done;
    Ok(edits
        .into_iter()
        .map(|e| Change {
            from: unit_at(text, e.range.start),
            to: unit_at(text, e.range.end),
            at: unit_at(text, e.range.start),
            insert: e.text,
        })
        .collect())
}

/// The corrections of several notes as one change, and what became of them.
pub fn fix_many(notes: &mut Notes, text: &str, which: &[usize]) -> crate::people::Made {
    let (new, done, left) = notes::apply_fixes(text, notes, which);
    let message = if left > 0 {
        lang::f(
            "{} düzeltme uygulandı, {} tanesi açık kaldı. Ctrl+Z geri alır.",
            "{} corrections applied, {} left open. Ctrl+Z undoes them.",
            &[&done, &left],
        )
    } else {
        lang::f(
            "{} düzeltme uygulandı. Ctrl+Z geri alır.",
            "{} corrections applied. Ctrl+Z undoes them.",
            &[&done],
        )
    };
    crate::people::Made {
        change: crate::people::diff(text, &new),
        cursor: None,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

    #[test]
    fn a_note_finds_its_quote() {
        let mut n = Notes::default();
        let at = PLAY.encode_utf16().count() / 2;
        add(&mut n, PLAY, at, "Atölye", "“taş durdu” bir bak").unwrap();
        let c = &cards(PLAY, &n)[0];
        let quoted: String = String::from_utf16(
            &PLAY.encode_utf16().collect::<Vec<_>>()[c.from.unwrap()..c.to.unwrap()],
        )
        .unwrap();
        assert_eq!(quoted, "taş durdu");
        assert!(add(&mut n, PLAY, 0, "", "“”").is_err());
    }

    #[test]
    fn a_fix_is_one_change_and_done() {
        let mut n = Notes::default();
        paste(&mut n, "48: “Soğuk her şeyi” → “Ayaz her şeyi”", "Okur");
        assert_eq!(n.list.len(), 1);
        let changes = fix(&mut n, PLAY, 0).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].insert, "Ayaz");
        assert_eq!(n.list[0].state, State::Done);
    }

    #[test]
    fn a_note_is_found_by_its_id_whatever_came_before() {
        let mut n = Notes::default();
        add(&mut n, PLAY, 0, "Ada", "birinci").unwrap();
        add(&mut n, PLAY, 0, "Baran", "ikinci").unwrap();
        let id = cards(PLAY, &n)[1].id.clone();
        // Someone else's note comes in before it, and one goes.
        n.list.insert(
            0,
            Note {
                author: "Cem".into(),
                text: "araya".into(),
                ..Note::default()
            },
        );
        n.name_all();
        n.list.remove(1);
        let i = which(&n, Some(&id), None).unwrap();
        assert_eq!(n.list[i].text, "ikinci");
        assert!(which(&n, Some("yok"), None).is_err());
        // A window older than ids says where it is.
        assert_eq!(which(&n, None, Some(0)).unwrap(), 0);
        assert!(which(&n, None, Some(9)).is_err());
    }
}
