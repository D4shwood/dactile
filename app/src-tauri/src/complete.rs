//! The menu a backslash (or Ctrl+K, or a `#` at a line's start) opens: core's
//! `complete`, ranked for what has been typed, with offsets in UTF-16 units.

use crate::play::Change;
use crate::view::unit_at;
use dactile::complete::{self, Action, Group};
use dactile::render::Doc;
use dactile::{info, source, tex};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub key: String,
    pub label: String,
    /// "scene" (characters in this scene), "character", "command", "macro".
    pub group: &'static str,
    /// "insert", or a form to open: "scene", "act", "character".
    pub action: &'static str,
    pub text: String,
    /// Where the cursor goes in `text`.
    pub cursor: usize,
}

fn group(g: Group) -> &'static str {
    match g {
        Group::InScene => "scene",
        Group::Character => "character",
        Group::Command => "command",
        Group::Macro => "macro",
    }
}

fn item(it: complete::Item) -> Item {
    let (action, text, cursor) = match it.action {
        Action::Insert { text, cursor } => {
            let cursor = text[..cursor].encode_utf16().count();
            ("insert", text, cursor)
        }
        Action::Scene => ("scene", String::new(), 0),
        Action::Act => ("act", String::new(), 0),
        Action::NewCharacter => ("character", String::new(), 0),
    };
    Item {
        key: it.key,
        label: it.label,
        group: group(it.group),
        action,
        text,
        cursor,
    }
}

/// What can follow `\` (or `#`, with `hash`) at UTF-16 offset `at`, for
/// `query`, best first.
pub fn items(text: &str, at: usize, query: &str, hash: bool) -> Vec<Item> {
    let all = if hash {
        complete::headings(source::line_ending(text))
    } else {
        let doc = Doc::new(text);
        let byte = crate::view::byte_at(text, at);
        complete::items(&doc, doc.block_at(byte))
    };
    complete::filter(&all, query)
        .into_iter()
        .take(60)
        .map(item)
        .collect()
}

/// A new scene's heading (`place`, `time`) or act's (`place` is its title),
/// as the terminal app's forms write it: on the cursor's line when that is
/// empty, else on a line of its own after the cursor.
pub fn heading(text: &str, at: usize, act: bool, place: &str, time: &str) -> Change {
    let hash = !info::has_option(text, "nohash");
    let parts: Vec<String> = [place, time]
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(tex::escape_text)
        .collect();
    let (word, macro_) = if act {
        ("#perde", "\\perde")
    } else {
        ("#sahne", "\\sahne")
    };
    let line = match (hash, parts.is_empty()) {
        (true, true) => word.to_string(),
        (true, false) => format!("{word} {}", parts.join(" — ")),
        (false, true) => macro_.to_string(),
        (false, false) => format!("{macro_}[{}]", parts.join(" — ")),
    };
    let nl = source::line_ending(text);
    let at = crate::view::byte_at(text, at);
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find(['\r', '\n']).map_or(text.len(), |i| at + i);
    let (range, insert) = if text[start..end].trim().is_empty() {
        (start..end, format!("{line}{nl}"))
    } else {
        (at..at, format!("{nl}{line}{nl}"))
    };
    let from = unit_at(text, range.start);
    Change {
        from,
        to: unit_at(text, range.end),
        at: from + insert.encode_utf16().count(),
        insert,
    }
}

/// What a scene's form offers: places and times used before.
#[derive(Debug, Clone, Serialize)]
pub struct Fields {
    pub places: Vec<String>,
    pub times: Vec<String>,
}

pub fn fields(text: &str) -> Fields {
    let doc = Doc::new(text);
    Fields {
        places: complete::places(&doc),
        times: complete::times(&doc),
    }
}

/// The verbs a `\xdo` direction usually starts with.
pub fn verbs(text: &str) -> Vec<String> {
    complete::verbs(&Doc::new(text), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    #[test]
    fn a_name_comes_first_for_its_letters() {
        let at = PLAY.find("Amca, yine").unwrap();
        let at = PLAY[..at].encode_utf16().count();
        let found = items(PLAY, at, "se", false);
        assert_eq!(found[0].key, "se");
        assert_eq!(found[0].action, "insert");
        assert_eq!(found[0].text, "\\se ");
        assert_eq!(found[0].cursor, 4);
        let c = heading(PLAY, at, false, "Avlu", "Gece");
        assert_eq!(c.insert, "\n#sahne Avlu — Gece\n");
        assert!(
            items(PLAY, at, "", true)
                .iter()
                .any(|i| i.action == "scene")
        );
    }
}
