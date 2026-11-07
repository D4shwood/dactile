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
            let line_start = text[..c.span.start].rfind('\n').map_or(0, |i| i + 1);
            let line_end = text[g.span.end..]
                .find('\n')
                .map_or(text.len(), |i| g.span.end + i + 1);
            let alone = text[line_start..c.span.start].trim().is_empty()
                && text[g.span.end..line_end].trim().is_empty();
            return Some(if alone {
                Edit::new(line_start..line_end, "")
            } else {
                Edit::new(c.span.start..g.span.end, "")
            });
        }
        return (g.inner != value).then(|| Edit::new(g.inner_span, value));
    }
    if value.is_empty() {
        return None;
    }
    let nl = source::line_ending(text);
    let after = cmds
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
    Some(Edit::new(
        line_end..line_end,
        format!("{nl}\\{name}{{{value}}}"),
    ))
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
    class_option_in(text, source::split(text).preamble)
}

fn class_option_in(
    text: &str,
    preamble: std::ops::Range<usize>,
) -> Option<(std::ops::Range<usize>, &str)> {
    let o = tex::optional(text, class_end(text, preamble)?)?;
    Some((o.inner_span.clone(), o.inner))
}

/// Where `\documentclass` itself ends: its options, if any, start there.
fn class_end(text: &str, preamble: std::ops::Range<usize>) -> Option<usize> {
    let c = tex::commands(text, preamble)
        .into_iter()
        .find(|c| c.name == "documentclass")?;
    Some(c.span.end)
}

/// Rewrites the class's options: those `drop` names go, `add` comes last.
fn set_class_options(text: &str, drop: impl Fn(&str) -> bool, add: Option<&str>) -> Option<Edit> {
    match documentclass_option(text) {
        Some((range, opts)) => {
            let mut parts: Vec<&str> = opts
                .split(',')
                .map(str::trim)
                .filter(|o| !o.is_empty() && !drop(o))
                .collect();
            parts.extend(add);
            Some(Edit::new(range, parts.join(",")))
        }
        None => {
            let at = class_end(text, source::split(text).preamble)?;
            Some(Edit::new(at..at, format!("[{}]", add?)))
        }
    }
}

/// Where the scene headings stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Center,
    Right,
}

impl Side {
    /// A class option's side, in either language.
    pub fn parse(option: &str) -> Option<Side> {
        match option {
            "left" | "sol" => Some(Side::Left),
            "right" | "sag" => Some(Side::Right),
            "center" | "orta" => Some(Side::Center),
            _ => None,
        }
    }

    /// The class option that asks for it.
    pub fn option(self) -> &'static str {
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

/// Where the scene headings stand, by the class's option.
pub fn scene_align(text: &str) -> Side {
    scene_align_in(text, source::split(text).preamble)
}

/// The same, for a preamble already found.
pub(crate) fn scene_align_in(text: &str, preamble: std::ops::Range<usize>) -> Side {
    let Some((_, opts)) = class_option_in(text, preamble) else {
        return Side::Center;
    };
    // The last one counts, as in the class.
    let side = opts.split(',').rev().find_map(|o| Side::parse(o.trim()));
    side.unwrap_or(Side::Center)
}

/// Sets where the scene headings stand; the center, the class's own, leaves
/// no option behind.
pub fn set_scene_align(text: &str, side: Side) -> Option<Edit> {
    if scene_align(text) == side {
        return None;
    }
    let want = (side != Side::Center).then_some(side.option());
    set_class_options(text, |o| Side::parse(o).is_some(), want)
}

/// Switches `\documentclass[draft]` and `[final]`.
pub fn set_final(text: &str, fin: bool) -> Option<Edit> {
    let (want, other) = if fin {
        ("final", "draft")
    } else {
        ("draft", "final")
    };
    if has_option(text, want) {
        return None;
    }
    set_class_options(text, |o| o == other, Some(want))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tex::apply;

    const PLAY: &str = include_str!("../../examples/ornek-oyun.tex");

    #[test]
    fn scene_headings_stand_where_the_class_says() {
        assert_eq!(scene_align(PLAY), Side::Center);
        let left = apply(PLAY, &[set_scene_align(PLAY, Side::Left).unwrap()]);
        assert!(left.contains("\\documentclass[draft,left]{tiyatro}"));
        assert_eq!(scene_align(&left), Side::Left);
        let right = apply(&left, &[set_scene_align(&left, Side::Right).unwrap()]);
        assert!(right.contains("\\documentclass[draft,right]{tiyatro}"));
        let back = apply(&right, &[set_scene_align(&right, Side::Center).unwrap()]);
        assert_eq!(back, PLAY);
        assert!(set_scene_align(PLAY, Side::Center).is_none());
        assert_eq!(
            scene_align("\\documentclass[sag]{tiyatro}\n\\begin{document}"),
            Side::Right
        );
        let bare = "\\documentclass{tiyatro}\n\\begin{document}";
        let t = apply(bare, &[set_scene_align(bare, Side::Left).unwrap()]);
        assert!(t.starts_with("\\documentclass[left]{tiyatro}"));
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
