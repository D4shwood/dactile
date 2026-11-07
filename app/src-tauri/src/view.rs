//! The page view of a play for the editor: the text stays as it is, and
//! each block says how it reads on the page. A mark with a label covers its
//! range with that label (a chip: `\se ` is "SELİN"); one without styles
//! it. The editor shows a block's source while the cursor is in it.
//!
//! Offsets are in UTF-16 units, as the editor counts them.

use dactile::cast;
use dactile::lang;
use dactile::lex::{self, Inline, Kind};
use dactile::outline;
use dactile::render::Doc;
use serde::Serialize;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mark {
    pub from: usize,
    pub to: usize,
    pub cls: &'static str,
    /// What shows instead of the range; `None` styles it.
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block {
    pub from: usize,
    pub to: usize,
    /// How the block is set: "line", "direction", "act", "scene"…
    pub cls: &'static str,
    pub marks: Vec<Mark>,
    /// The speaker of a line of dialogue.
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Person {
    pub alias: String,
    /// As a line of dialogue shows it.
    pub name: String,
}

/// The cover: everything before `\begin{document}`, shown as a card.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cover {
    pub from: usize,
    pub to: usize,
    pub title: String,
    pub author: String,
    /// The cast table's rows, as the PDF prints them.
    pub cast: Vec<CastRow>,
}

/// A row of the cover's cast list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CastRow {
    /// "SELİN — Ada'nın kızı".
    Person { name: String },
    /// A `\karaktergrubu`: its members, bracketed, and its name beside them.
    Group { name: String, members: Vec<String> },
}

impl CastRow {
    /// One line of text: "Köylüler (Birinci Kadın, ÇOBAN)" for a group.
    pub fn line(&self) -> String {
        match self {
            CastRow::Person { name } => name.clone(),
            CastRow::Group { name, members } if name.is_empty() => members.join(", "),
            CastRow::Group { name, members } => format!("{name} ({})", members.join(", ")),
        }
    }
}

fn cast_entry(c: &cast::Character) -> String {
    match c.desc.as_deref().filter(|d| !d.trim().is_empty()) {
        Some(d) => format!("{} — {}", c.display_name(), d.trim()),
        None => c.display_name().to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    pub cover: Option<Cover>,
    /// Who can speak: the characters, in the cast's order.
    pub cast: Vec<Person>,
    pub blocks: Vec<Block>,
    /// `\end{document}` and after: hidden.
    pub tail: Option<Range<usize>>,
    /// Where the scene headings stand: left, center or right.
    pub scene_align: &'static str,
}

/// Byte offsets to UTF-16 ones.
struct Units(Vec<u32>);

impl Units {
    fn of(text: &str) -> Units {
        let mut v = Vec::with_capacity(text.len() + 1);
        let mut n = 0u32;
        for c in text.chars() {
            for _ in 0..c.len_utf8() {
                v.push(n);
            }
            n += c.len_utf16() as u32;
        }
        v.push(n);
        Units(v)
    }

    fn at(&self, byte: usize) -> usize {
        self.0[byte.min(self.0.len() - 1)] as usize
    }
}

/// UTF-16 offset `at` as a byte offset of `text`.
pub fn byte_at(text: &str, at: usize) -> usize {
    let mut n = 0;
    for (i, c) in text.char_indices() {
        if n >= at {
            return i;
        }
        n += c.len_utf16();
    }
    text.len()
}

/// A byte offset of `text` in UTF-16 units.
pub fn unit_at(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].encode_utf16().count()
}

/// Marks in bytes, turned into units at the end.
struct Out<'a> {
    text: &'a str,
    doc: &'a Doc,
    marks: Vec<(Range<usize>, &'static str, Option<String>)>,
}

impl Out<'_> {
    fn chip(&mut self, r: Range<usize>, cls: &'static str, label: impl Into<String>) {
        self.marks.push((r, cls, Some(label.into())));
    }

    fn style(&mut self, r: Range<usize>, cls: &'static str) {
        if r.start < r.end {
            self.marks.push((r, cls, None));
        }
    }

    fn name(&self, alias: &str) -> String {
        self.doc
            .cast
            .get(alias)
            .map_or_else(|| alias.to_string(), |c| c.script.clone())
    }

    /// The pieces of a line's text, as `render` shows them.
    fn inline(&mut self, range: Range<usize>, dialogue: bool) {
        for p in lex::inline(self.text, range, &self.doc.cast, dialogue) {
            let s = &self.text[p.span.clone()];
            match &p.kind {
                Inline::Text => {}
                Inline::Ref(alias) => {
                    let name = self
                        .doc
                        .cast
                        .inline_name(alias)
                        .unwrap_or_else(|| alias.clone());
                    // The space a macro swallows stays in the source.
                    let end = p.span.start + s.trim_end().len();
                    self.chip(p.span.start..end, "ref", name);
                }
                Inline::Ic { inner } => {
                    self.chip(p.span.start..inner.start, "dir", "(");
                    self.style(inner.clone(), "dir");
                    self.inline(inner.clone(), dialogue);
                    self.chip(inner.end..p.span.end, "dir", ")");
                }
                Inline::Under { inner } => {
                    self.chip(p.span.start..inner.start, "hide", "");
                    self.style(inner.clone(), "under");
                    self.chip(inner.end..p.span.end, "hide", "");
                }
                Inline::Wrap { name, inner } if name == "dipnot" || name == "footnote" => {
                    self.chip(p.span.start..inner.start, "dir", "[*");
                    self.style(inner.clone(), "wrap");
                    self.inline(inner.clone(), dialogue);
                    self.chip(inner.end..p.span.end, "dir", "]");
                }
                Inline::Wrap { name, inner } => {
                    let cls = match name.as_str() {
                        "textit" | "emph" => "em",
                        "textbf" => "strong",
                        _ => "wrap",
                    };
                    self.chip(p.span.start..inner.start, "hide", "");
                    self.style(inner.clone(), cls);
                    self.inline(inner.clone(), dialogue);
                    self.chip(inner.end..p.span.end, "hide", "");
                }
                Inline::Special if s.starts_with("\\\\") => {
                    self.chip(p.span.clone(), "verse", "↵");
                }
                Inline::Special => {
                    let shown = match s {
                        "--" => "–",
                        "---" => "—",
                        "~" => "\u{a0}",
                        _ => s.get(1..).unwrap_or(s),
                    };
                    let shown = if shown.is_empty() { s } else { shown };
                    self.chip(p.span.clone(), "special", shown);
                }
                Inline::Raw => self.style(p.span.clone(), "raw"),
            }
        }
    }

    /// How block `b` reads; its class.
    fn block(&mut self, b: usize) -> &'static str {
        let block = &self.doc.blocks[b];
        let span = block.span.clone();
        let src = &self.text[span.clone()];
        let lead = span.start + (src.len() - src.trim_start().len());
        match &block.kind {
            Kind::Blank => "blank",
            Kind::Comment => "comment",
            Kind::Line { alias, text } => {
                let Some(c) = self.doc.cast.get(alias) else {
                    return "unknown";
                };
                let cls = if c.star { "name star" } else { "name" };
                let head = lead + self.text[lead..text.start].trim_end().len();
                self.chip(lead..head, cls, c.script.clone());
                self.inline(text.clone(), true);
                "line"
            }
            Kind::Do { aliases, text } => {
                let names: Vec<String> = aliases
                    .iter()
                    .map(|a| self.doc.cast.inline_name(a).unwrap_or_else(|| a.clone()))
                    .collect();
                self.chip(lead..text.start, "dir", format!("({} ", names.join(", ")));
                self.inline(text.clone(), true);
                self.chip(text.end..span.end, "dir", ")");
                "direction"
            }
            Kind::Action { text } => {
                self.chip(lead..text.start, "dir", "(");
                self.inline(text.clone(), false);
                self.chip(text.end..span.end, "dir", ")");
                "direction"
            }
            Kind::Stage { text, right } => {
                self.chip(lead..text.start, "hide", "");
                self.inline(text.clone(), false);
                self.chip(text.end..span.end, "hide", "");
                if *right { "middir" } else { "stage" }
            }
            Kind::Move { enter, aliases } => {
                let names: Vec<String> = aliases.iter().map(|a| self.name(a)).collect();
                let and = format!(" {} ", lang::l("ile", "and"));
                let who = match names.split_last() {
                    Some((last, rest)) if !rest.is_empty() => {
                        format!("{}{and}{last}", rest.join(", "))
                    }
                    _ => names.join(""),
                };
                let verb = match (enter, names.len() > 1) {
                    (true, false) => lang::l("girer", "enters"),
                    (true, true) => lang::l("girerler", "enter"),
                    (false, false) => lang::l("çıkar", "exits"),
                    (false, true) => lang::l("çıkarlar", "exit"),
                };
                self.chip(lead..span.end, "dir", format!("({who} {verb}.)"));
                "direction"
            }
            Kind::Beat => {
                self.chip(lead..span.end, "dir", lang::l("(Sus.)", "(Pause.)"));
                "direction"
            }
            Kind::Act { title, .. } => {
                let n = self
                    .doc
                    .act_of(b)
                    .map_or(0, |a| self.doc.outline.acts[a].number);
                let mut head = format!("{}. {}", outline::roman(n), lang::l("PERDE", "ACT"));
                if !title.trim().is_empty() {
                    head.push('\n');
                    head.push_str(title.trim());
                }
                self.chip(lead..span.end, "act", head);
                "act"
            }
            Kind::Scene { place, .. } => {
                let scene = self.doc.scene_of(b).map(|s| &self.doc.outline.scenes[s]);
                let mut head =
                    scene.map_or_else(|| format!("{} ?", lang::l("Sahne", "Scene")), |s| s.title());
                if !place.trim().is_empty() {
                    head.push_str(&format!(" — {}", place.trim()));
                }
                if let Some(s) = scene.filter(|s| !s.cast.is_empty()) {
                    let names: Vec<String> = s.cast.iter().map(|a| self.name(a)).collect();
                    head.push('\n');
                    head.push_str(&names.join(", "));
                }
                self.chip(lead..span.end, "scene", head);
                "scene"
            }
            Kind::Section { which, title, .. } => {
                let name = if title.trim().is_empty() {
                    match which {
                        lex::Section::Preface => lang::l("ÖNSÖZ", "PREFACE"),
                        lex::Section::Notes => lang::l("REJİSÖRE NOTLAR", "NOTES FOR THE DIRECTOR"),
                    }
                    .to_string()
                } else {
                    cast::upper_tr(title.trim())
                };
                self.chip(lead..span.end, "section", name);
                "section"
            }
            Kind::Unknown { .. } => "unknown",
            Kind::Other => {
                let cmd = self.text[lead..span.end].trim();
                if cmd == "\\maketitle" {
                    self.chip(lead..span.end, "faint", lang::l("· kapak ·", "· cover ·"));
                    "page"
                } else if cmd == "\\kisiler" {
                    self.chip(lead..span.end, "faint", lang::l("· kişiler ·", "· cast ·"));
                    "page"
                } else if matches!(cmd, "\\newpage" | "\\clearpage" | "\\pagebreak") {
                    self.chip(
                        lead..span.end,
                        "faint",
                        lang::l("· yeni sayfa ·", "· new page ·"),
                    );
                    "page"
                } else if !cmd.starts_with('\\') {
                    self.inline(lead..span.end, false);
                    "prose"
                } else {
                    "raw"
                }
            }
        }
    }
}

pub fn view(text: &str) -> View {
    let doc = Doc::new(text);
    let u = Units::of(text);
    let mut out = Out {
        text,
        doc: &doc,
        marks: Vec::new(),
    };
    let mut blocks = Vec::new();
    for b in doc.body.clone() {
        out.marks.clear();
        let cls = out.block(b);
        let span = doc.blocks[b].span.clone();
        let marks = out
            .marks
            .drain(..)
            .filter(|(r, _, label)| {
                r.start < r.end || label.as_deref().is_some_and(|l| !l.is_empty())
            })
            .map(|(r, cls, label)| Mark {
                from: u.at(r.start),
                to: u.at(r.end),
                cls,
                label,
            })
            .collect();
        let alias = match &doc.blocks[b].kind {
            Kind::Line { alias, .. } if cls == "line" => Some(alias.clone()),
            _ => None,
        };
        blocks.push(Block {
            from: u.at(span.start),
            to: u.at(span.end),
            cls,
            marks,
            alias,
        });
    }
    let body = doc.parts.body.clone();
    let cover = (body.start > 0).then(|| Cover {
        from: 0,
        to: u.at(body.start),
        title: doc.title(text).unwrap_or_default(),
        author: dactile::info::get(text, "author").unwrap_or_default(),
        cast: doc
            .cast
            .rows()
            .into_iter()
            .map(|r| match r {
                cast::Row::Character(i) => CastRow::Person {
                    name: cast_entry(&doc.cast.characters[i]),
                },
                cast::Row::Group(g) => {
                    let group = &doc.cast.groups[g];
                    CastRow::Group {
                        name: group.desc.trim().to_string(),
                        members: group
                            .members
                            .iter()
                            .map(|&m| cast_entry(&doc.cast.characters[m]))
                            .collect(),
                    }
                }
            })
            .collect(),
    });
    let tail = (body.end < text.len()).then(|| u.at(body.end)..u.at(text.len()));
    let cast = doc
        .cast
        .characters
        .iter()
        .map(|c| Person {
            alias: c.alias.clone(),
            name: c.script.clone(),
        })
        .collect();
    View {
        cover,
        cast,
        blocks,
        tail,
        scene_align: doc.scene_align.option(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../../../examples/ornek-oyun.tex");

    fn source(text: &str, m: &Mark) -> String {
        let s: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&s[m.from..m.to]).unwrap()
    }

    #[test]
    fn offsets_round_trip() {
        let t = "Şü 😀 a";
        for (i, _) in t.char_indices() {
            assert_eq!(byte_at(t, unit_at(t, i)), i);
        }
        assert_eq!(unit_at(t, t.len()), t.encode_utf16().count());
        let u = Units::of(t);
        assert_eq!(u.at(t.len()), t.encode_utf16().count());
        assert_eq!(u.at(5), unit_at(t, 5));
    }

    #[test]
    fn speakers_are_chips() {
        lang::set(lang::Lang::Tr);
        let v = view(PLAY);
        assert!(v.cover.as_ref().is_some_and(|c| !c.title.is_empty()));
        let line = v.blocks.iter().find(|b| b.cls == "line").unwrap();
        let name = &line.marks[0];
        assert!(name.cls.starts_with("name"));
        // The chip covers the macro, not the space after it.
        assert!(source(PLAY, name).starts_with('\\'));
        assert!(!source(PLAY, name).ends_with(' '));
        assert!(v.blocks.iter().any(|b| b.cls == "scene"));
        assert!(v.blocks.iter().any(|b| b.cls == "act"));
    }

    #[test]
    fn marks_stay_in_their_block_in_order() {
        let v = view(PLAY);
        let n = PLAY.encode_utf16().count();
        for b in &v.blocks {
            assert!(b.from <= b.to && b.to <= n);
            for m in &b.marks {
                assert!(b.from <= m.from && m.to <= b.to, "{m:?} in {b:?}");
            }
        }
        for w in v.blocks.windows(2) {
            assert!(w[0].to <= w[1].from);
        }
    }

    #[test]
    fn the_cover_lists_the_cast_table() {
        let v = view(PLAY);
        let cast = &v.cover.unwrap().cast;
        let group = cast
            .iter()
            .find_map(|r| match r {
                CastRow::Group { name, members } => Some((name, members)),
                _ => None,
            })
            .unwrap();
        assert_eq!(group.0, "Köylüler");
        assert_eq!(group.1, &["Birinci Kadın", "ÇOBAN"]);
        // Members show only in their group; `\gizlikarakter` not at all.
        let lines: Vec<String> = cast.iter().map(CastRow::line).collect();
        assert!(!lines.iter().any(|l| l == "ÇOBAN" || l.contains("HERKES")));
    }

    #[test]
    fn a_play_without_a_preamble_has_no_cover() {
        let v = view("Düz bir metin.\n");
        assert!(v.cover.is_none());
        assert!(v.tail.is_none());
    }
}
