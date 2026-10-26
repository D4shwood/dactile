//! Help while writing: who speaks next, `NAME:` for a character, and text
//! pasted from elsewhere put in a play's shape.

use crate::buffer::escape_pasted;
use crate::cast::Cast;
use crate::lex::{self, Kind};
use crate::render::Doc;

/// Who probably speaks on the empty line at `block`: in a scene where A just
/// answered B, B; where only A has spoken yet in a scene of two, the other.
pub fn next_speaker(doc: &Doc, block: usize) -> Option<String> {
    let scene = &doc.outline.scenes[doc.scene_of(block)?];
    let first = scene.block + doc.body.start;
    let speakers: Vec<&str> = doc.blocks[first..block]
        .iter()
        .rev()
        .filter_map(|b| match &b.kind {
            Kind::Line { alias, .. } => Some(alias.as_str()),
            _ => None,
        })
        .collect();
    let last = *speakers.first()?;
    if let Some(before) = speakers.iter().find(|&&a| a != last) {
        return Some(before.to_string());
    }
    match scene.cast.as_slice() {
        [a, b] => Some(if a == last { b } else { a }.clone()),
        _ => None,
    }
}

/// `SELİN`, `Ozan Usta`: a name a writer might put before a colon.
fn looks_like_name(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.chars().count() <= 40
        && s.split_whitespace().count() <= 3
        && s.chars()
            .all(|c| c.is_alphabetic() || c == ' ' || c == '.' || c == '\'')
}

/// An unknown name written in capitals (`AHMET`), which reads as a new
/// character rather than an ordinary word before a colon (`Not`).
pub fn is_caps_name(s: &str) -> bool {
    looks_like_name(s)
        && s.chars().any(char::is_alphabetic)
        && s.chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase)
}

/// Pasted text, line by line: `SELİN: text` a line of dialogue for a known
/// name, a line in parentheses a direction, `#sahne …` a heading as it is;
/// everything else escaped as plain text. Also counts `NAME:` lines whose
/// name in capitals nobody declared.
pub fn paste_lines(s: &str, cast: &Cast) -> (String, usize) {
    let mut unknown = 0;
    let lines: Vec<String> = s
        .replace("\r\n", "\n")
        .split('\n')
        .map(|line| {
            let t = line.trim();
            if let Some((name, rest)) = t.split_once(':')
                && looks_like_name(name)
            {
                if let Some(c) = cast.by_name(name) {
                    return format!("\\{} {}", c.alias, escape_pasted(rest.trim(), true));
                }
                if is_caps_name(name) {
                    unknown += 1;
                }
            }
            if let Some((word, _, _)) = lex::hash_line(t)
                && lex::heading(word).is_some()
            {
                return t.to_string();
            }
            if t.len() > 2
                && let Some(inner) = t.strip_prefix('(').and_then(|r| r.strip_suffix(')'))
            {
                return format!("\\action{{{}}}", escape_pasted(inner.trim(), false));
            }
            escape_pasted(line, false)
        })
        .collect();
    (lines.join("\n"), unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    /// The block holding `needle`, as if the line there were still empty.
    fn block_of(doc: &Doc, needle: &str) -> usize {
        doc.block_at(PLAY.find(needle).unwrap()).unwrap()
    }

    #[test]
    fn who_speaks_next() {
        let doc = Doc::new(PLAY);
        // After SELİN then OZAN: SELİN again.
        let b = block_of(&doc, "\\se Ama");
        assert_eq!(next_speaker(&doc, b).as_deref(), Some("se"));
        // Before anyone has spoken: nobody.
        let b = block_of(&doc, "\\action{Işıklar");
        assert_eq!(next_speaker(&doc, b), None);
    }

    #[test]
    fn names_before_a_colon() {
        let cast = Cast::of_play(PLAY);
        assert_eq!(cast.by_name("SELİN").unwrap().alias, "se");
        assert_eq!(cast.by_name("selin").unwrap().alias, "se");
        assert_eq!(cast.by_name("Ozan Usta").unwrap().alias, "oz");
        assert!(cast.by_name("Not").is_none());
        assert!(is_caps_name("AHMET"));
        assert!(is_caps_name("YAŞLI KADIN"));
        assert!(!is_caps_name("Not"));
        assert!(!is_caps_name("Saat 5"));
    }

    #[test]
    fn pasting_a_draft() {
        let cast = Cast::of_play(PLAY);
        let draft = "#sahne Ahır — Sabah\r\nSELİN: Kim var orada? 100% eminim.\n(Kapı açılır)\nOzan Usta: Benim.\nAHMET: Ben de.\nNot: bu sahne kısa.";
        let (out, unknown) = paste_lines(draft, &cast);
        assert_eq!(
            out,
            "#sahne Ahır — Sabah\n\\se Kim var orada? 100\\% eminim.\n\\action{Kapı açılır}\n\\oz Benim.\nAHMET: Ben de.\nNot: bu sahne kısa."
        );
        assert_eq!(unknown, 1);
    }
}
