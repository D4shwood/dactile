//! Learning a role by heart: the play as plain cues, the role's own lines
//! hidden until asked for, first as the first letters of their words.

use crate::cast::Cast;
use crate::lex::{self, Inline, Kind};
use crate::render::Doc;
use std::ops::Range;

/// A line of the play as memorizing shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    /// An index into `doc.blocks`.
    pub block: usize,
    /// The speaker's script name; None for directions and headings.
    pub who: Option<String>,
    pub text: String,
    /// A line of the role being learnt.
    pub ours: bool,
    pub heading: bool,
}

/// What a line reads as: names for `\alias`, escapes as what they print,
/// inline directions in parentheses.
pub(crate) fn plain(text: &str, range: Range<usize>, cast: &Cast) -> String {
    let mut out = String::new();
    for p in lex::inline(text, range, cast, false) {
        match p.kind {
            Inline::Text => out.push_str(&text[p.span]),
            Inline::Ref(a) => out.push_str(&cast.inline_name(&a).unwrap_or(a)),
            Inline::Ic { inner } => {
                out.push('(');
                out.push_str(&plain(text, inner, cast));
                out.push(')');
            }
            Inline::Under { inner } | Inline::Wrap { inner, .. } => {
                out.push_str(&plain(text, inner, cast))
            }
            Inline::Special => out.push_str(&crate::tex::unescape_text(&text[p.span])),
            Inline::Raw => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn names(cast: &Cast, aliases: &[String]) -> Vec<String> {
    aliases
        .iter()
        .map(|a| cast.get(a).map_or(a.clone(), |c| c.script.clone()))
        .collect()
}

/// The play's body as cues, `role` (an alias) marked as ours.
pub fn cues(text: &str, doc: &Doc, role: &str) -> Vec<Cue> {
    let tr = crate::lang::is_tr();
    let mut out = Vec::new();
    let cue = |block, who, text: String, ours, heading| Cue {
        block,
        who,
        text,
        ours,
        heading,
    };
    let mut scene = 0;
    for b in doc.body.clone() {
        let block = &doc.blocks[b];
        match &block.kind {
            Kind::Act { title, .. } => {
                let n = doc
                    .outline
                    .acts
                    .iter()
                    .find(|a| a.block == b - doc.body.start)
                    .map_or(0, |a| a.number);
                let act = if tr { "Perde" } else { "Act" };
                let head = format!("{}. {act} {title}", crate::outline::roman(n));
                out.push(cue(b, None, head.trim().to_string(), false, true));
            }
            Kind::Scene { .. } => {
                let s = doc.outline.scenes.get(scene);
                scene += 1;
                let word = if tr { "Sahne" } else { "Scene" };
                let head = s.map_or(word.to_string(), |s| match (s.prologue, tr) {
                    (true, true) => format!("Ön Sahne — {}", s.place),
                    (true, false) => format!("Prologue — {}", s.place),
                    _ => format!("{word} {} — {}", s.label, s.place),
                });
                out.push(cue(b, None, head, false, true));
            }
            Kind::Line { alias, text: t } => {
                let who = doc
                    .cast
                    .get(alias)
                    .map_or(alias.clone(), |c| c.script.clone());
                let line = plain(text, t.clone(), &doc.cast);
                out.push(cue(b, Some(who), line, alias == role, false));
            }
            Kind::Do { aliases, text: t } => {
                let line = format!(
                    "({}, {})",
                    names(&doc.cast, aliases).join(", "),
                    plain(text, t.clone(), &doc.cast)
                );
                out.push(cue(b, None, line, false, false));
            }
            Kind::Move { enter, aliases } => {
                let who = names(&doc.cast, aliases);
                let verb = match (enter, who.len() > 1, tr) {
                    (true, false, true) => "girer",
                    (true, true, true) => "girerler",
                    (false, false, true) => "çıkar",
                    (false, true, true) => "çıkarlar",
                    (true, false, false) => "enters",
                    (true, true, false) => "enter",
                    (false, false, false) => "exits",
                    (false, true, false) => "exit",
                };
                let and = if tr { " ile " } else { " and " };
                out.push(cue(
                    b,
                    None,
                    format!("({}{verb}.)", who.join(and) + " "),
                    false,
                    false,
                ));
            }
            Kind::Action { text: t } | Kind::Stage { text: t, .. } => {
                let line = format!("({})", plain(text, t.clone(), &doc.cast));
                out.push(cue(b, None, line, false, false));
            }
            _ => {}
        }
    }
    out
}

/// "Ben öyle mi dedim?" → "B·· ö··· m· d····?": each word's first letter, its
/// length in dots, punctuation as it is.
pub fn initials(line: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for c in line.chars() {
        if c.is_alphanumeric() {
            out.push(if first { c } else { '·' });
            first = false;
        } else {
            out.push(c);
            first = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints() {
        assert_eq!(initials("Ben öyle mi dedim?"), "B·· ö··· m· d····?");
    }

    #[test]
    fn the_example() {
        let text = include_str!("../../examples/ornek-oyun.tex");
        let doc = Doc::new(text);
        let all = cues(text, &doc, "oz");
        let ours: Vec<&Cue> = all.iter().filter(|c| c.ours).collect();
        assert_eq!(
            ours[0].text,
            "Durdu kızım. (elini taşa koyar) Soğuk her şeyi durdurur."
        );
        assert!(
            all.iter()
                .any(|c| c.heading && c.text.starts_with("Sahne 1 — "))
        );
    }
}
