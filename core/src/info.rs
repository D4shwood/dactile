//! The play's cover details in the preamble: `\title`, `\author`, `\draft`,
//! `\copyrightnotice`, `\address`, `\email`, `\telephone`, and the class's
//! draft or final mode.

use crate::source;
use crate::tex::{self, Edit};

/// The cover commands, in the order a new one is written.
pub const FIELDS: [(&str, &str); 7] = [
    ("title", "Başlık"),
    ("author", "Yazar"),
    ("draft", "Taslak no"),
    ("copyrightnotice", "Telif"),
    ("address", "Adres"),
    ("email", "E-posta"),
    ("telephone", "Telefon"),
];

/// The value of `\name{…}` in the preamble.
pub fn get(text: &str, name: &str) -> Option<String> {
    let pre = source::split(text).preamble;
    let c = tex::commands(text, pre)
        .into_iter()
        .find(|c| c.name == name)?;
    tex::group(text, c.span.end).map(|g| g.inner.to_string())
}

/// Sets `\name{value}`: changes it in place, adds it after the last cover
/// command (or after `\documentclass`), or removes it when `value` is empty.
pub fn set(text: &str, name: &str, value: &str) -> Option<Edit> {
    let pre = source::split(text).preamble;
    let cmds = tex::commands(text, pre.clone());
    if let Some(c) = cmds.iter().find(|c| c.name == name) {
        let g = tex::group(text, c.span.end)?;
        if value.is_empty() {
            return Some(remove_line(text, c.span.start..g.span.end));
        }
        return (g.inner != value).then(|| Edit::new(g.inner_span, value));
    }
    if value.is_empty() {
        return None;
    }
    add_line(text, &format!("\\{name}{{{value}}}"))
}

/// Takes a command out, and its line with it when it is alone there.
fn remove_line(text: &str, span: std::ops::Range<usize>) -> Edit {
    let line_start = text[..span.start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[span.end..]
        .find('\n')
        .map_or(text.len(), |i| span.end + i + 1);
    let alone = text[line_start..span.start].trim().is_empty()
        && text[span.end..line_end].trim().is_empty();
    if alone {
        Edit::new(line_start..line_end, "")
    } else {
        Edit::new(span, "")
    }
}

/// A new line after the last cover command, or after `\documentclass`.
fn add_line(text: &str, line: &str) -> Option<Edit> {
    let nl = source::line_ending(text);
    let after = tex::commands(text, source::split(text).preamble)
        .iter()
        .filter(|c| FIELDS.iter().any(|(n, _)| *n == c.name) || c.name == "documentclass")
        .filter_map(|c| {
            let mut end = c.span.end;
            while let Some(g) = tex::optional(text, end) {
                end = g.span.end;
            }
            tex::group(text, end).map(|g| g.span.end)
        })
        .max()?;
    let line_end = text[after..].find('\n').map_or(text.len(), |i| after + i);
    let line_end = if text[..line_end].ends_with('\r') {
        line_end - 1
    } else {
        line_end
    };
    Some(Edit::new(line_end..line_end, format!("{nl}{line}")))
}

/// Whether the class is in final mode.
pub fn is_final(text: &str) -> bool {
    has_option(text, "final")
}

/// Whether `\documentclass[…]` has `option`.
pub fn has_option(text: &str, option: &str) -> bool {
    documentclass_option(text).is_some_and(|(_, o)| o.split(',').any(|x| x.trim() == option))
}

fn documentclass_option(text: &str) -> Option<(std::ops::Range<usize>, &str)> {
    let o = tex::optional(text, class_end(text)?)?;
    Some((o.inner_span.clone(), o.inner))
}

/// Where `\documentclass` itself ends: its options, if any, start there.
fn class_end(text: &str) -> Option<usize> {
    let c = tex::commands(text, source::split(text).preamble)
        .into_iter()
        .find(|c| c.name == "documentclass")?;
    Some(c.span.end)
}

/// Where the scene headings stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Center,
    Right,
}

impl Side {
    /// A side as `\scenetitleflush` takes it, in either language.
    pub fn parse(word: &str) -> Option<Side> {
        match word {
            "left" | "sol" => Some(Side::Left),
            "right" | "sağ" | "sag" => Some(Side::Right),
            "center" | "orta" => Some(Side::Center),
            _ => None,
        }
    }

    /// The word that asks for it.
    pub fn word(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Center => "center",
            Side::Right => "right",
        }
    }

    /// The next way round: center, left, right.
    pub fn next(self) -> Side {
        match self {
            Side::Center => Side::Left,
            Side::Left => Side::Right,
            Side::Right => Side::Center,
        }
    }
}

const FLUSH: &str = "scenetitleflush";

/// Where the scene headings stand, by `\scenetitleflush[…]` in the preamble.
pub fn scene_align(text: &str) -> Side {
    scene_align_in(text, source::split(text).preamble)
}

/// The same, for a preamble already found. The last one counts, as in the
/// class.
pub(crate) fn scene_align_in(text: &str, preamble: std::ops::Range<usize>) -> Side {
    tex::commands(text, preamble)
        .iter()
        .rfind(|c| c.name == FLUSH)
        .and_then(|c| tex::optional(text, c.span.end))
        .and_then(|o| Side::parse(o.inner.trim()))
        .unwrap_or(Side::Center)
}

/// Sets where the scene headings stand; the center, the class's own, leaves
/// no line behind.
pub fn set_scene_align(text: &str, side: Side) -> Option<Edit> {
    if scene_align(text) == side {
        return None;
    }
    let cmds = tex::commands(text, source::split(text).preamble);
    let mut lines = cmds.iter().filter(|c| c.name == FLUSH);
    let Some(c) = lines.next_back() else {
        return add_line(text, &format!("\\{FLUSH}[{}]", side.word()));
    };
    let option = tex::optional(text, c.span.end);
    // With one before it, taking this one out would bring that one back.
    if side == Side::Center && lines.next().is_none() {
        let end = option.map_or(c.span.end, |o| o.span.end);
        return Some(remove_line(text, c.span.start..end));
    }
    Some(match option {
        Some(o) => Edit::new(o.inner_span, side.word()),
        None => Edit::new(c.span.end..c.span.end, format!("[{}]", side.word())),
    })
}

/// Switches `\documentclass[draft]` and `[final]`.
pub fn set_final(text: &str, fin: bool) -> Option<Edit> {
    let (want, other) = if fin {
        ("final", "draft")
    } else {
        ("draft", "final")
    };
    match documentclass_option(text) {
        Some((range, opts)) => {
            let mut parts: Vec<&str> = opts
                .split(',')
                .map(str::trim)
                .filter(|o| !o.is_empty() && *o != other)
                .collect();
            if parts.contains(&want) {
                return None;
            }
            parts.push(want);
            Some(Edit::new(range, parts.join(",")))
        }
        None => {
            let at = class_end(text)?;
            Some(Edit::new(at..at, format!("[{want}]")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tex::apply;

    const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

    #[test]
    fn scene_headings_stand_where_the_preamble_says() {
        assert_eq!(scene_align(PLAY), Side::Center);
        let left = apply(PLAY, &[set_scene_align(PLAY, Side::Left).unwrap()]);
        assert!(left.contains("\n\\scenetitleflush[left]\n"));
        assert_eq!(scene_align(&left), Side::Left);
        let right = apply(&left, &[set_scene_align(&left, Side::Right).unwrap()]);
        assert!(right.contains("\n\\scenetitleflush[right]\n"));
        let back = apply(&right, &[set_scene_align(&right, Side::Center).unwrap()]);
        assert_eq!(back, PLAY);
        assert!(set_scene_align(PLAY, Side::Center).is_none());
        let doc = |pre: &str| format!("\\documentclass{{tiyatro}}\n{pre}\n\\begin{{document}}");
        assert_eq!(scene_align(&doc("\\scenetitleflush[sağ]")), Side::Right);
        assert_eq!(scene_align(&doc("\\scenetitleflush[sag]")), Side::Right);
        // The class's options don't reach the headings.
        let option = "\\documentclass[left]{tiyatro}\n\\begin{document}";
        assert_eq!(scene_align(option), Side::Center);
        // A word the class refuses, and none at all.
        assert_eq!(scene_align(&doc("\\scenetitleflush[üst]")), Side::Center);
        let bare = doc("\\scenetitleflush");
        let t = apply(&bare, &[set_scene_align(&bare, Side::Left).unwrap()]);
        assert_eq!(scene_align(&t), Side::Left);
        // The last one counts, and stays when an earlier one is behind it.
        let two = doc("\\scenetitleflush[left]\n\\scenetitleflush[right]");
        assert_eq!(scene_align(&two), Side::Right);
        let t = apply(&two, &[set_scene_align(&two, Side::Center).unwrap()]);
        assert_eq!(scene_align(&t), Side::Center);
    }

    #[test]
    fn reading_and_writing() {
        assert_eq!(get(PLAY, "title").as_deref(), Some("Kar Masalı"));
        assert_eq!(get(PLAY, "telephone"), None);
        let out = apply(PLAY, &[set(PLAY, "title", "Kar").unwrap()]);
        assert!(out.contains("\\title{Kar}\n"));
        assert_eq!(set(PLAY, "title", "Kar Masalı"), None);
        let out = apply(PLAY, &[set(PLAY, "telephone", "0212").unwrap()]);
        assert!(out.contains("\\email{ornek@example.com}\n\\telephone{0212}\n"));
        let out = apply(PLAY, &[set(PLAY, "email", "").unwrap()]);
        assert!(!out.contains("\\email") && out.contains("\\address{İstanbul}\n\n\\karakter"));
        assert!(!is_final(PLAY));
        let out = apply(PLAY, &[set_final(PLAY, true).unwrap()]);
        assert!(out.contains("\\documentclass[final]{tiyatro}"));
        assert!(is_final(&out));
        assert_eq!(set_final(&out, true), None);
    }
}
