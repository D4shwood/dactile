//! The play's shape: scenes and acts moved, split and merged as pieces of
//! the source, and the short ids at a scene's heading (`#sahne Meydan %@k3`)
//! that tie it to its summary in the sidecar wherever it goes.

use crate::lex::{self, Kind};
use crate::render::Doc;
use crate::shelf::{self, Unit, UnitKind};
use crate::tex::{self, Edit};
use std::ops::Range;

/// What starts an id, as a comment at the heading's end.
pub const MARK: &str = "%@";

/// The heading's first line without the `%~` of a hidden unit: the part
/// TeX would read, and where it starts in the line.
fn unhidden(line: &str) -> (usize, &str) {
    let mut rest = line;
    loop {
        let t = rest.trim_start();
        match t.strip_prefix(shelf::HIDDEN) {
            Some(r) => rest = r,
            None => {
                let t = rest.trim_start();
                return (line.len() - t.len(), t);
            }
        }
    }
}

/// The id at the end of a heading's first line: `k3` of `… %@k3`.
pub fn id_in(heading: &str) -> Option<&str> {
    let line = heading.split('\n').next()?;
    let (_, code) = unhidden(line);
    let c = tex::comment_start(code)?;
    let rest = code[c..].strip_prefix(MARK)?;
    let end = rest
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    Some(&rest[..end]).filter(|id| !id.is_empty())
}

/// Every id in the play.
pub fn ids(text: &str) -> Vec<&str> {
    text.split('\n').filter_map(id_in).collect()
}

/// An id no heading has yet: k1, k2…
pub fn new_id(text: &str) -> String {
    let taken = ids(text);
    (1..)
        .map(|n| format!("k{n}"))
        .find(|id| !taken.contains(&id.as_str()))
        .unwrap_or_default()
}

/// Gives the heading that starts at `at` the id `id`: at its line's end, or
/// before a comment already there.
pub fn give_id(text: &str, at: usize, id: &str) -> Edit {
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    let line = &text[at..end];
    let (from, code) = unhidden(line);
    match tex::comment_start(code) {
        Some(c) => Edit::new(at + from + c..at + from + c, format!("{MARK}{id} ")),
        None => {
            let code_end = at + line.trim_end().len();
            Edit::new(code_end..code_end, format!(" {MARK}{id}"))
        }
    }
}

/// Where a unit goes, as "before `units[u]`", `u == units.len()` being after
/// the last one. The places unit `i` can go to, its own among them.
pub fn places(units: &[Unit], i: usize) -> Vec<usize> {
    let Some(me) = units.get(i) else {
        return Vec::new();
    };
    let n = units.len();
    match me.kind {
        // Before any unit (before an act: at the end of the act before it),
        // except before the first act when nothing comes before it.
        UnitKind::Scene => (0..=n)
            .filter(|&u| u != i + 1)
            .filter(|&u| !(u == 0 && units[0].kind == UnitKind::Act))
            .collect(),
        // Before another act, or after the last; not before the next act,
        // where it stands already.
        UnitKind::Act => {
            let next = (i + 1..n).find(|&u| units[u].kind == UnitKind::Act);
            (0..=n)
                .filter(|&u| {
                    if u == n {
                        next.is_some()
                    } else {
                        units[u].kind == UnitKind::Act && Some(u) != next
                    }
                })
                .collect()
        }
    }
}

/// The place one step up or down from where unit `i` stands, if any.
pub fn step(units: &[Unit], i: usize, down: bool) -> Option<usize> {
    let me = units.get(i)?;
    let n = units.len();
    match (me.kind, down) {
        (UnitKind::Scene, false) => {
            let u = i.checked_sub(1)?;
            (!(u == 0 && units[0].kind == UnitKind::Act)).then_some(u)
        }
        // Past the next unit: over a scene, or into the next act as its
        // first scene.
        (UnitKind::Scene, true) => (i + 1 < n).then_some(i + 2),
        (UnitKind::Act, false) => (0..i).rev().find(|&u| units[u].kind == UnitKind::Act),
        (UnitKind::Act, true) => {
            let next = (i + 1..n).find(|&u| units[u].kind == UnitKind::Act)?;
            Some(
                (next + 1..n)
                    .find(|&u| units[u].kind == UnitKind::Act)
                    .unwrap_or(n),
            )
        }
    }
}

/// The trailing whitespace of `s`.
fn ws_tail(s: &str) -> &str {
    &s[s.trim_end().len()..]
}

/// The blank lines after `span`: up to the start of the next line that has
/// something on it.
fn sep_after(text: &str, span: &Range<usize>) -> usize {
    let rest = &text[span.end..];
    let ws = rest.len() - rest.trim_start().len();
    // Back to just after the last line end in it.
    rest[..ws]
        .rfind('\n')
        .map_or(span.end, |n| span.end + n + 1)
}

/// Moves unit `i` to place `to` (from `places`): one edit, and where the
/// unit starts in the new text.
pub fn relocate(text: &str, units: &[Unit], i: usize, to: usize) -> Option<(Edit, usize)> {
    let me = units.get(i)?;
    let span = me.span.clone();
    let moved = &text[span.clone()];
    let after = sep_after(text, &span);
    let sep = &text[span.end..after];
    if to == i || to == i + 1 && me.kind == UnitKind::Scene {
        return None;
    }
    if let Some(u) = units.get(to) {
        let p = u.span.start;
        if p < span.start {
            // Up: it takes the gap that was before it.
            let between = &text[p..span.start];
            let tail = ws_tail(between);
            let core = &between[..between.len() - tail.len()];
            let new = format!("{moved}{tail}{core}{sep}");
            return Some((Edit::new(p..after, new), p));
        }
        if p >= after {
            let mid = &text[after..p];
            let tail = ws_tail(mid);
            let core = &mid[..mid.len() - tail.len()];
            let new = format!("{core}{sep}{moved}{tail}");
            let start = span.start + core.len() + sep.len();
            return Some((Edit::new(span.start..p, new), start));
        }
        return None;
    }
    // After the last unit.
    let e = units.last()?.span.end;
    if e <= span.end || after > e {
        return None;
    }
    let mid = &text[after..e];
    let new = format!("{mid}{sep}{moved}");
    let start = span.start + mid.len() + sep.len();
    Some((Edit::new(span.start..e, new), start))
}

/// Moves several units (indexes in order, none inside another) to place
/// `to` together, in their order: the edits, and where the first starts.
pub fn gather(
    text: &str,
    units: &[Unit],
    picked: &[usize],
    to: usize,
) -> Option<(Vec<Edit>, usize)> {
    let spans: Vec<Range<usize>> = picked
        .iter()
        .map(|&i| units.get(i).map(|u| u.span.clone()))
        .collect::<Option<_>>()?;
    let (p, end) = match units.get(to) {
        Some(u) => (u.span.start, false),
        None => (units.last()?.span.end, true),
    };
    if spans
        .iter()
        .any(|s| s.start <= p && p < s.end || (end && s.end == p))
    {
        return None;
    }
    let mut edits: Vec<Edit> = spans
        .iter()
        .map(|s| Edit::new(s.start..sep_after(text, s).max(s.end), ""))
        .collect();
    // A unit right before the place keeps its gap to it: the cut takes the
    // gap before it instead.
    for (e, s) in edits.iter_mut().zip(&spans) {
        if e.range.end >= p && !end {
            let gap = text[..s.start].trim_end().len();
            e.range = gap..s.end;
        }
    }
    // Cuts that meet become one.
    edits.sort_by_key(|e| e.range.start);
    let mut cuts: Vec<Edit> = Vec::new();
    for e in edits {
        match cuts.last_mut() {
            Some(last) if e.range.start <= last.range.end => {
                last.range.end = last.range.end.max(e.range.end);
            }
            _ => cuts.push(e),
        }
    }
    let mut edits = cuts;
    let joined = spans
        .iter()
        .map(|s| &text[s.clone()])
        .collect::<Vec<_>>()
        .join("\n\n");
    let removed_before: usize = edits
        .iter()
        .filter(|e| e.range.end <= p)
        .map(|e| e.range.len())
        .sum();
    let (insert, start) = if end {
        (format!("\n\n{joined}"), p - removed_before + 2)
    } else {
        (format!("{joined}\n\n"), p - removed_before)
    };
    if edits.iter().any(|e| e.range.start < p && p < e.range.end) {
        return None;
    }
    edits.push(Edit::new(p..p, insert));
    Some((edits, start))
}

/// The heading word as it was written: `#sahne`, `#scene`, `\sahne`.
fn heading_word(line: &str) -> String {
    let t = line.trim_start();
    if t.starts_with('#') {
        let w = lex::hash_line(t).map_or("sahne", |(w, _, _)| w);
        return format!("#{w} ");
    }
    let name: String = t[1..].chars().take_while(|c| c.is_alphabetic()).collect();
    format!("\\{name}")
}

/// A new scene from the line at `at` on, with the place of the scene it was
/// in: the edit and where the cursor goes (the place's end, to change it).
pub fn split(text: &str, doc: &Doc, at: usize) -> Option<(Edit, usize)> {
    let b = doc.block_at(at)?;
    if !doc.body.contains(&b) || shelf::is_hidden(text, &doc.blocks[b]) {
        return None;
    }
    let s = &doc.outline.scenes[doc.scene_of(b)?];
    let head = doc.body.start + s.block;
    if b == head {
        return None;
    }
    // Not on the blank lines right after the heading either.
    if (head + 1..b).all(|x| matches!(doc.blocks[x].kind, Kind::Blank | Kind::Comment))
        && matches!(doc.blocks[b].kind, Kind::Blank)
    {
        return None;
    }
    let first = text[doc.blocks[head].span.clone()]
        .split('\n')
        .next()
        .unwrap_or("");
    let word = heading_word(first);
    let place = s.place.trim();
    let heading = if word.starts_with('#') {
        format!("{word}{place}").trim_end().to_string()
    } else {
        format!("{word}[{place}]")
    };
    let p = doc.blocks[b].span.start;
    let cursor = p + heading.len() - usize::from(!word.starts_with('#'));
    Some((Edit::new(p..p, format!("{heading}\n\n")), cursor))
}

/// Scene `k` (of `outline.scenes`) and the one after it as one: the second
/// heading becomes a direction with its place, or goes when it had none.
/// The edit, and the second scene's id if it had one.
pub fn merge(text: &str, doc: &Doc, k: usize) -> Option<(Edit, Option<String>)> {
    let sc = &doc.outline.scenes;
    let (a, b) = (sc.get(k)?, sc.get(k + 1)?);
    if a.act != b.act || a.end != b.block {
        return None;
    }
    let head = &doc.blocks[doc.body.start + b.block];
    let id = id_in(&text[head.span.clone()]).map(str::to_string);
    let place = b.place.trim();
    let edit = if place.is_empty() {
        shelf::cut(text, head.span.clone())
    } else {
        Edit::new(head.span.clone(), format!("\\action{{{place}}}"))
    };
    Some((edit, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = "\\documentclass{tiyatro}\n\\karakter{SELİN}{se}\n\\begin{document}\n\n#perde Bir\n\n#sahne Ev\n\n\\se A.\n\n#sahne Bahçe %@k1\n\n\\se B.\n\n#perde İki\n\n#sahne Yol\n\n\\se C.\n\\end{document}\n";

    fn order(text: &str) -> Vec<String> {
        let doc = Doc::new(text);
        shelf::units(text, &doc)
            .iter()
            .map(|u| u.title.clone())
            .collect()
    }

    fn moved(text: &str, i: usize, down: bool) -> String {
        let doc = Doc::new(text);
        let units = shelf::units(text, &doc);
        let to = step(&units, i, down).unwrap();
        let (e, start) = relocate(text, &units, i, to).unwrap();
        let out = tex::apply(text, &[e]);
        assert_eq!(
            &out[start..start + units[i].span.len()],
            &text[units[i].span.clone()]
        );
        out
    }

    #[test]
    fn ids() {
        assert_eq!(id_in("#sahne Bahçe %@k1"), Some("k1"));
        assert_eq!(id_in("%~ #sahne Bahçe %@k12 % not"), Some("k12"));
        assert_eq!(id_in("#sahne 100\\%@x"), None);
        assert_eq!(new_id(PLAY), "k2");
        let at = PLAY.find("#sahne Ev").unwrap();
        let t = tex::apply(PLAY, &[give_id(PLAY, at, "k2")]);
        assert!(t.contains("#sahne Ev %@k2\n"));
        let t = "#sahne Ev % not\n";
        assert_eq!(
            tex::apply(t, &[give_id(t, 0, "k3")]),
            "#sahne Ev %@k3 % not\n"
        );
        let t = "%~ #sahne Ev\n";
        assert_eq!(tex::apply(t, &[give_id(t, 0, "k3")]), "%~ #sahne Ev %@k3\n");
        // The heading reads the same.
        assert_eq!(
            order(&tex::apply(PLAY, &[give_id(PLAY, at, "k2")])),
            order(PLAY)
        );
    }

    #[test]
    fn scenes_step_through_acts() {
        // Bir, Ev, Bahçe, İki, Yol
        assert_eq!(order(PLAY), ["Bir", "Ev", "Bahçe", "İki", "Yol"]);
        let t = moved(PLAY, 1, true);
        assert_eq!(order(&t), ["Bir", "Bahçe", "Ev", "İki", "Yol"]);
        assert!(t.contains("#sahne Bahçe %@k1\n\n\\se B.\n\n#sahne Ev\n\n\\se A.\n\n#perde İki"));
        // Over the act's edge: the first scene of the next act.
        let t = moved(PLAY, 2, true);
        assert_eq!(order(&t), ["Bir", "Ev", "İki", "Bahçe", "Yol"]);
        // And back up: the last of the act before.
        let t2 = moved(&t, 3, false);
        assert_eq!(t2, PLAY);
        // The last scene of the play up, and down again.
        let t = moved(PLAY, 4, false);
        assert_eq!(order(&t), ["Bir", "Ev", "Bahçe", "Yol", "İki"]);
        assert!(t.ends_with("\\se C.\n\n#perde İki\n\\end{document}\n"));
        assert_eq!(moved(&t, 3, true), PLAY);
    }

    #[test]
    fn acts_move_whole() {
        let t = moved(PLAY, 3, false);
        assert_eq!(order(&t), ["İki", "Yol", "Bir", "Ev", "Bahçe"]);
        assert!(t.contains("\\se C.\n\n#perde Bir"));
        assert!(t.contains("\\se B.\n\\end{document}"));
        assert_eq!(moved(&t, 0, true), PLAY);
        let doc = Doc::new(PLAY);
        let units = shelf::units(PLAY, &doc);
        assert_eq!(step(&units, 0, false), None);
        assert_eq!(step(&units, 3, true), None);
        assert_eq!(places(&units, 0), [0, 5]);
        assert_eq!(places(&units, 3), [0, 3]);
        assert_eq!(places(&units, 1), [1, 3, 4, 5]);
    }

    #[test]
    fn several_at_once() {
        let doc = Doc::new(PLAY);
        let units = shelf::units(PLAY, &doc);
        // Ev and Yol before Bahçe.
        let (e, start) = gather(PLAY, &units, &[1, 4], 2).unwrap();
        let t = tex::apply(PLAY, &e);
        assert_eq!(order(&t), ["Bir", "Ev", "Yol", "Bahçe", "İki"]);
        assert!(t[start..].starts_with("#sahne Ev"));
        assert!(t.contains("\\se C.\n\n#sahne Bahçe"));
        assert!(t.contains("#perde İki\n\\end{document}"));
        // Both to the end.
        let (e, start) = gather(PLAY, &units, &[1, 2], 5).unwrap();
        let t = tex::apply(PLAY, &e);
        assert_eq!(order(&t), ["Bir", "İki", "Yol", "Ev", "Bahçe"]);
        assert!(t[start..].starts_with("#sahne Ev"));
        assert!(t.contains("#perde Bir\n\n#perde İki"));
        // Two that meet, to where they are.
        let (e, _) = gather(PLAY, &units, &[1, 2], 3).unwrap();
        assert_eq!(order(&tex::apply(PLAY, &e)), order(PLAY));
    }

    #[test]
    fn hidden_scenes_move_too() {
        let doc = Doc::new(PLAY);
        let units = shelf::units(PLAY, &doc);
        let t = tex::apply(PLAY, &[shelf::hide(PLAY, units[1].span.clone())]);
        let t = moved(&t, 1, true);
        assert!(t.contains("#sahne Bahçe %@k1\n\n\\se B.\n\n%~ #sahne Ev"));
    }

    #[test]
    fn split_and_merge() {
        let doc = Doc::new(PLAY);
        let at = PLAY.find("\\se B.").unwrap();
        let (e, cursor) = split(PLAY, &doc, at).unwrap();
        let t = tex::apply(PLAY, &[e]);
        assert!(t.contains("#sahne Bahçe %@k1\n\n#sahne Bahçe\n\n\\se B."));
        assert!(t[..cursor].ends_with("#sahne Bahçe") && t[cursor..].starts_with("\n\n\\se B."));
        // On the heading itself, nothing.
        assert!(split(PLAY, &doc, PLAY.find("#sahne Ev").unwrap()).is_none());
        let t = "\\begin{document}\n\\sahne[İç][2]\n\\se A.\n\\se B.\n\\end{document}";
        let (e, _) = split(t, &Doc::new(t), t.find("\\se B").unwrap()).unwrap();
        assert!(tex::apply(t, &[e]).contains("\\se A.\n\\sahne[İç]\n\n\\se B."));

        let (e, id) = merge(PLAY, &doc, 0).unwrap();
        assert_eq!(id.as_deref(), Some("k1"));
        let t = tex::apply(PLAY, &[e]);
        assert!(t.contains("\\se A.\n\n\\action{Bahçe}\n\n\\se B."));
        // Not across acts.
        assert!(merge(PLAY, &doc, 1).is_none());
    }
}
